//! Composition root of the front end: the one place that knows every screen.
//!
//! [`App`] owns the game and the persistent log, decides which screen is active,
//! and draws the persistent layout around it. It reads no terminal and writes none
//! (the caller feeds it commands and hands it a frame), so tests drive it headless.

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use engine::addiction::AddictionBook;
use engine::character::PerkBook;
use engine::clock::Span;
use engine::combat::Dice;
use engine::companions::CompanionBook;
use engine::craft::{RecipeBook, StationBook};
use engine::dialogue::{Conversation, DialogueBook};
use engine::game::{Arsenal, Game, Outcome};
use engine::gather::FindBook;
use engine::inventory::{ItemBook, Slot};
use engine::quests::QuestBook;
use engine::reputation::ReputationBook;
use engine::rest::{BedBook, FULL_REST_HOURS};
use engine::saves::{SaveFolder, Slot as SaveSlot};
use engine::shared::{
    BedId, Effect, FindId, GameData, InventoryView, ItemId, LocationId, QuestId, QuestStage,
    StationId, VendorId, apply_all,
};
use engine::trade::VendorBook;
use engine::travel::TravelBook;
use engine::weapon_mods::WeaponModBook;
use engine::world::WorldBook;
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Alignment, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::character::{self, Creation, LevelUp};
use crate::combat::{self, Fight};
use crate::craft::{self, Craft};
use crate::dialogue::{self, Talk};
use crate::exploration::{self, Exploration, Person};
use crate::inventory::use_note;
use crate::inventory::{self, Pack};
use crate::quests::{self, Journal};
use crate::repair::{self, Repair};
use crate::reputation::{self, Reputation};
use crate::saves::{self, Purpose, Slots};
use crate::shared::dice::Rng;
use crate::shared::input::Command;
use crate::shared::layout::{self, MIN_HEIGHT, MIN_WIDTH};
use crate::shared::log::{self, Entry, Log};
use crate::shared::names::Names;
use crate::shared::theme;
use crate::trade::{self, Trade};
use crate::travel::{self, Travel, walk_time};

/// The quest a new game starts in, and the stage that opens it.
const OPENING_QUEST: &str = "ain_t_that_a_kick_in_the_head";
const OPENING_STAGE: u32 = 10;
/// What the player wields and wears. The opening quest hands over the pistol late
/// in Doc's scene; it is taken in hand when a fight starts and the hand is empty,
/// so the player is never sent to the fight unarmed by forgetting to equip it. The
/// armor is a stand-in worn from the start, and without it the Goodsprings fight
/// is not fair.
const START_WEAPON: &str = "9mm_pistol";
const START_ARMOR: &str = "leather_armor";

/// Where the saves are kept: a folder in the user's data folder, or in the working
/// folder when the environment names none.
fn default_save_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
        })
        .unwrap_or_default();
    base.join("fnv").join("saves")
}

enum Screen {
    /// Making the character, before the first scene.
    Creating(Creation),
    Exploring(Exploration),
    Talking(Talk),
    Fighting(Fight),
    /// Choosing a save to write or to load.
    Saves(Slots),
    Leveling(LevelUp),
    /// Reading the quest journal.
    Journaling(Journal),
    /// Reading the reputation screen.
    Standing(Reputation),
    /// Choosing where to walk.
    Traveling(Travel),
    /// Looking through the pack.
    Packing(Pack),
    /// Making things at a crafting station.
    Crafting(Craft),
    /// Having a vendor mend a weapon.
    Repairing(Repair),
    /// At a counter. `from` is the conversation that opened the shop, if one did, so
    /// leaving the shop picks it up again.
    Trading {
        trade: Trade,
        from: Option<Talk>,
    },
    /// The player fell in a fight; nothing more can be done but quit.
    Down,
}

/// What the front end knows about each place: its name, what stands inside it and
/// who lives there, so arriving somewhere needs no search through the loaded data.
struct Places {
    names: HashMap<LocationId, String>,
    /// The buildings and spots inside a place.
    children: HashMap<LocationId, Vec<LocationId>>,
    /// Everyone with a conversation to open, with the places they are found in.
    residents: Vec<(Vec<LocationId>, Person)>,
}

impl Places {
    fn from_data(data: &GameData, dialogue: &Arc<DialogueBook>) -> Self {
        let residents = data
            .npcs
            .iter()
            .filter_map(|npc| {
                // Most dialogue files are transcripts without a graph; only the
                // authored ones open.
                let dialogue_id = npc
                    .dialogue
                    .iter()
                    .find(|id| Conversation::open(Arc::clone(dialogue), id).is_ok())?;
                let person = Person {
                    npc: npc.id.clone(),
                    name: npc.name.clone(),
                    dialogue: dialogue_id.clone(),
                };
                Some((npc.location_ids.clone(), person))
            })
            .collect();
        Self {
            names: data
                .locations
                .iter()
                .map(|location| (location.id.clone(), location.name.clone()))
                .collect(),
            children: data
                .locations
                .iter()
                .map(|location| (location.id.clone(), location.children.clone()))
                .collect(),
            residents,
        }
    }

    /// The place and the buildings inside it: where the player can find a shop.
    fn here(&self, place: &LocationId) -> Vec<LocationId> {
        std::iter::once(place.clone())
            .chain(self.children.get(place).into_iter().flatten().cloned())
            .collect()
    }

    /// The name the player sees for a place.
    fn name(&self, place: &LocationId) -> String {
        self.names
            .get(place)
            .cloned()
            .unwrap_or_else(|| place.to_string())
    }

    /// Who can be talked to anywhere in `here`, by name.
    fn people(&self, here: &[LocationId]) -> Vec<Person> {
        let mut people: Vec<Person> = self
            .residents
            .iter()
            .filter(|(spots, _)| spots.iter().any(|spot| here.contains(spot)))
            .map(|(_, person)| person.clone())
            .collect();
        people.sort_by(|a, b| a.name.cmp(&b.name));
        people
    }
}

/// The whole front end's state.
pub struct App {
    game: Game,
    dialogue: Arc<DialogueBook>,
    place: String,
    place_id: LocationId,
    /// The place and the buildings inside it: where the player can find a shop.
    here: Vec<LocationId>,
    places: Places,
    vendors: Arc<VendorBook>,
    names: Arc<Names>,
    people: Vec<Person>,
    screen: Screen,
    log: Log,
    dice: Rng,
    /// The folder the named saves and the automatic save are kept in.
    saves: SaveFolder,
    /// Whether the game saves by itself at safe moments (after a walk, a sleep or a
    /// settled fight). Off until [`App::with_autosave`], so a test never writes to
    /// the player's folder.
    autosave: bool,
    running: bool,
}

impl App {
    /// A new game in `place`, with the opening quest started.
    ///
    /// # Errors
    ///
    /// When the place is unknown or the loaded data does not build into a game.
    pub fn new(data: &GameData, place: &LocationId) -> anyhow::Result<Self> {
        Self::with_dice(data, place, Rng::from_clock())
    }

    /// The same, with the dice a test chooses.
    ///
    /// # Errors
    ///
    /// The same as [`App::new`].
    pub fn with_dice(data: &GameData, place: &LocationId, dice: Rng) -> anyhow::Result<Self> {
        let location = data
            .locations
            .get(place)
            .with_context(|| format!("unknown starting place {place}"))?;
        let quests = Arc::new(QuestBook::new(data.quests.iter()).context("building the quests")?);
        let world = Arc::new(WorldBook::new(data.npcs.iter().map(|npc| &npc.id)));
        let dialogue =
            Arc::new(DialogueBook::new(data.dialogue.iter()).context("building the dialogue")?);

        let items = Arc::new(ItemBook::from_data(data).context("building the item book")?);
        let arsenal = Arc::new(Arsenal::from_data(data).context("building the arsenal")?);
        let vendors = Arc::new(VendorBook::from_data(data).context("building the vendors")?);
        let beds = Arc::new(BedBook::from_data(data).context("building the beds")?);
        let companions =
            Arc::new(CompanionBook::from_data(data).context("building the companions")?);
        let recipes = Arc::new(
            RecipeBook::from_data(data, |item| items.contains(item))
                .context("building the recipes")?,
        );
        let stations =
            Arc::new(StationBook::from_data(data).context("building the crafting stations")?);
        let finds = Arc::new(FindBook::from_data(data).context("building the finds")?);
        let reputation =
            Arc::new(ReputationBook::from_data(data).context("building the reputation book")?);
        let map = Arc::new(TravelBook::from_data(data).context("building the map")?);
        let addictions =
            Arc::new(AddictionBook::from_data(data).context("building the addiction book")?);
        let weapon_mods =
            Arc::new(WeaponModBook::from_data(data).context("building the weapon mods")?);
        let mut game = Game::with_world(quests, world)
            .with_items(items)
            .with_addictions(addictions)
            .with_weapon_mods(weapon_mods)
            .with_arsenal(arsenal)
            .with_perks(Arc::new(PerkBook::base_game(data)))
            .with_companions(companions)
            .with_beds(beds)
            .with_recipes(recipes)
            .with_stations(stations)
            .with_finds(finds)
            .with_reputation(reputation)
            .with_vendors(Arc::clone(&vendors))
            .context("stocking the vendors")?
            .with_map(map, place)
            .context("placing the player on the map")?;
        let opening = Effect::SetStage {
            quest: QuestId::try_new(OPENING_QUEST).context("opening quest id")?,
            stage: QuestStage::new(OPENING_STAGE),
        };
        let armor = ItemId::try_new(START_ARMOR).context("starting armor id")?;
        let gift = Effect::GiveItem {
            item: armor.clone(),
            count: NonZeroU32::MIN,
        };
        apply_all(&[opening, gift], &mut game).context("starting the opening quest")?;
        game.equip(&armor).context("wearing the starting armor")?;

        // Buildings sit inside the settlement, so their residents are "here" too.
        let places = Places::from_data(data, &dialogue);
        let here = places.here(place);
        let people = places.people(&here);

        let mut log = Log::default();
        log.push(Entry::Note(format!(
            "A new game begins in {}.",
            location.name
        )));
        let mut app = Self {
            game,
            dialogue,
            screen: Screen::Down,
            place: location.name.clone(),
            place_id: place.clone(),
            here,
            places,
            vendors,
            names: Arc::new(Names::from_data(data)),
            people,
            log,
            dice,
            saves: SaveFolder::new(default_save_dir()),
            autosave: false,
            running: true,
        };
        app.screen = app.exploring();
        Ok(app)
    }

    /// The same app saving to and loading from the folder `dir`; tests use it to
    /// keep the player's own saves out of reach.
    #[cfg(test)]
    #[must_use]
    pub fn with_save_dir(mut self, dir: PathBuf) -> Self {
        self.saves = SaveFolder::new(dir);
        self
    }

    /// The same app, saving by itself at safe moments: after a walk, after a sleep
    /// and after a fight is settled. The automatic save has a slot of its own, so it
    /// never writes over a save the player named.
    #[must_use]
    pub fn with_autosave(mut self) -> Self {
        self.autosave = true;
        self
    }

    /// Whether keys are being typed into a text field, so the front end must hand
    /// them to [`App::type_key`] and not read them as commands.
    #[must_use]
    pub fn is_typing(&self) -> bool {
        matches!(&self.screen, Screen::Saves(slots) if slots.is_typing())
    }

    /// Applies a key typed into a text field.
    pub fn type_key(&mut self, key: KeyCode) {
        let Screen::Saves(slots) = &mut self.screen else {
            return;
        };
        let next = slots.type_key(key);
        self.after_saves(next);
    }

    /// The same app, starting at the character creation screen. The game is the
    /// one built by [`App::new`]; making the character swaps in the new one.
    #[must_use]
    pub fn with_creation(mut self) -> Self {
        self.screen = Screen::Creating(Creation::new(self.game.perks()));
        self
    }

    /// False once the player has quit.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Applies one command to the active screen.
    pub fn update(&mut self, command: Command) {
        if command == Command::Quit {
            self.running = false;
            return;
        }
        match &mut self.screen {
            Screen::Creating(creation) => {
                if let Some(character) = creation.update(command, self.game.perks()) {
                    self.game = self.game.clone().with_character(character);
                    self.log.push(Entry::Note("Your story begins.".to_owned()));
                    self.screen = self.exploring();
                }
            }
            Screen::Exploring(scene) => match scene.update(command) {
                exploration::Next::Stay => {}
                exploration::Next::Quit => self.running = false,
                exploration::Next::Talk(index) => self.start_talk(index),
                exploration::Next::Shop(index) => {
                    if let Some((vendor, _)) = self.shops_here().into_iter().nth(index) {
                        self.open_shop(&vendor);
                    }
                }
                exploration::Next::Fight(index) => self.start_fight(index),
                exploration::Next::Sleep(index) => self.sleep(index),
                exploration::Next::Gather(index) => self.gather(index),
                exploration::Next::Craft(index) => self.open_station(index),
                exploration::Next::Repair(index) => self.open_repairs(index),
                exploration::Next::Character => self.screen = Screen::Leveling(LevelUp::new()),
                exploration::Next::Pack => {
                    let seed =
                        (0..8).fold(0u64, |seed, _| (seed << 8) | u64::from(self.dice.percent()));
                    self.screen = Screen::Packing(
                        Pack::open(&self.game, Arc::clone(&self.names)).with_dice(Rng::new(seed)),
                    );
                }
                exploration::Next::Wait { span, label } => self.wait(span, label),
                exploration::Next::Journal => {
                    self.screen = Screen::Journaling(Journal::open(&self.game));
                }
                exploration::Next::Reputation => {
                    self.screen = Screen::Standing(Reputation::open(&self.game));
                }
                exploration::Next::Travel => {
                    self.screen = Screen::Traveling(Travel::open(&self.game));
                }
                exploration::Next::Save => self.open_saves(Purpose::Save),
                exploration::Next::Load => self.open_saves(Purpose::Load),
            },
            Screen::Saves(slots) => {
                let next = slots.update(command);
                self.after_saves(next);
            }
            Screen::Journaling(journal) => {
                if journal.update(command) == quests::Next::Leave {
                    self.screen = self.exploring();
                }
            }
            Screen::Standing(standing) => {
                if standing.update(command) == reputation::Next::Leave {
                    self.screen = self.exploring();
                }
            }
            Screen::Traveling(travel) => match travel.update(command, &self.game) {
                travel::Next::Stay => {}
                travel::Next::Leave => self.screen = self.exploring(),
                travel::Next::Go(to) => self.travel_to(&to),
            },
            Screen::Leveling(sheet) => {
                if sheet.update(command, &mut self.game, &mut self.log) == character::Next::Leave {
                    self.screen = self.exploring();
                }
            }
            Screen::Packing(pack) => {
                if pack.update(command, &mut self.game, &mut self.log) == inventory::Next::Leave {
                    self.screen = self.exploring();
                }
            }
            Screen::Crafting(craft) => {
                if craft.update(command, &mut self.game, &mut self.log) == craft::Next::Leave {
                    self.screen = self.exploring();
                }
            }
            Screen::Repairing(repairs) => {
                if repairs.update(command, &mut self.game, &mut self.log) == repair::Next::Leave {
                    self.screen = self.exploring();
                }
            }
            Screen::Talking(talk) => match talk.update(command, &mut self.game, &mut self.log) {
                dialogue::Next::Stay => {}
                dialogue::Next::Leave => self.screen = self.exploring(),
                dialogue::Next::Shop(vendor) => self.open_shop(&vendor),
            },
            Screen::Trading { trade, .. } => {
                if trade.update(command, &mut self.game, &mut self.log) == trade::Next::Leave {
                    self.leave_shop();
                }
            }
            Screen::Fighting(fight) => {
                if let combat::Next::Done(outcome) =
                    fight.update(command, &mut self.game, &mut self.log, &mut self.dice)
                {
                    self.screen = self.after_fight(outcome);
                }
            }
            Screen::Down => {
                if matches!(command, Command::Select | Command::Back) {
                    self.running = false;
                }
            }
        }
        // A fight sets its own items off, round by round; everywhere else the game
        // checks after each command, because a wait, a poisoned drink or a settled
        // fight may have brought one's moment.
        if !matches!(self.screen, Screen::Fighting(_) | Screen::Down) {
            self.set_off_items();
        }
    }

    /// Uses the items that work by themselves and whose moment has come, and says so.
    fn set_off_items(&mut self) {
        for (item, used) in self.game.auto_use() {
            let name = self.names.name(&item);
            let note = use_note(&name, &used, self.game.inventory().item_use(&item));
            self.log
                .push(Entry::Note(format!("It goes off by itself. {note}")));
        }
    }

    /// The shops the player can reach from here, in the order the scene lists them.
    fn shops_here(&self) -> Vec<(VendorId, String)> {
        self.here
            .iter()
            .flat_map(|place| self.game.vendors_at(place))
            .map(|(id, name)| (id.clone(), name.to_owned()))
            .collect()
    }

    /// Opens the shop of `vendor`. A conversation that was open stays with the shop
    /// and comes back when the player leaves it.
    fn open_shop(&mut self, vendor: &VendorId) {
        let from = match std::mem::replace(&mut self.screen, Screen::Down) {
            Screen::Talking(talk) => Some(talk),
            _ => None,
        };
        let name = self
            .vendors
            .get(vendor)
            .map_or_else(|| vendor.to_string(), |info| info.name().to_owned());
        match Trade::open(
            vendor.clone(),
            name.clone(),
            &self.game,
            Arc::clone(&self.names),
        ) {
            Some(trade) => self.screen = Screen::Trading { trade, from },
            None => {
                self.log
                    .push(Entry::Note(format!("{name} has nothing to trade.")));
                self.screen = self.resume(from);
            }
        }
    }

    fn leave_shop(&mut self) {
        let from = match std::mem::replace(&mut self.screen, Screen::Down) {
            Screen::Trading { from, .. } => from,
            _ => None,
        };
        self.screen = self.resume(from);
    }

    /// Back to the conversation that opened a shop, unless that topic ended it.
    fn resume(&self, from: Option<Talk>) -> Screen {
        match from {
            Some(mut talk) if !talk.is_over() => {
                // What was bought or sold can open or close topics that ask for caps.
                talk.refresh(&self.game);
                Screen::Talking(talk)
            }
            _ => self.exploring(),
        }
    }

    /// Lets time pass and says when it is now, or why it could not.
    fn wait(&mut self, span: Span, label: &str) {
        let note = match self.game.wait(span) {
            Ok(()) => format!("{label}. It is now {}.", self.game.clock()),
            Err(error) => error.to_string(),
        };
        self.log.push(Entry::Note(note));
    }

    /// The beds the player can sleep in here, with the names to show.
    fn beds_here(&self) -> Vec<(BedId, String)> {
        self.here
            .iter()
            .flat_map(|place| self.game.beds_at(place))
            .map(|(id, name)| (id.clone(), name.to_owned()))
            .collect()
    }

    /// The things lying here that the player has not taken, with the names to show.
    fn finds_here(&self) -> Vec<(FindId, String)> {
        self.here
            .iter()
            .flat_map(|place| self.game.finds_at(place))
            .map(|(id, name)| (id.clone(), name.to_owned()))
            .collect()
    }

    /// The crafting stations here, with the names to show.
    fn stations_here(&self) -> Vec<(StationId, String)> {
        self.here
            .iter()
            .flat_map(|place| self.game.stations_at(place))
            .map(|(id, name)| (id.clone(), name.to_owned()))
            .collect()
    }

    /// The merchants here who mend weapons, with the names to show.
    fn repairers_here(&self) -> Vec<(VendorId, String)> {
        self.here
            .iter()
            .flat_map(|place| self.game.repairers_at(place))
            .map(|(id, name)| (id.clone(), name.to_owned()))
            .collect()
    }

    /// Picks up the find at `index` of [`App::finds_here`] and says what it was, or
    /// why it could not be done.
    fn gather(&mut self, index: usize) {
        let Some((find, name)) = self.finds_here().into_iter().nth(index) else {
            return;
        };
        let note = match self.game.gather(&find) {
            Ok(()) => format!("You pick up {name}."),
            Err(error) => error.to_string(),
        };
        self.log.push(Entry::Note(note));
        self.screen = self.exploring();
    }

    /// Opens the crafting station at `index` of [`App::stations_here`].
    fn open_station(&mut self, index: usize) {
        let Some((station, name)) = self.stations_here().into_iter().nth(index) else {
            return;
        };
        self.screen = Screen::Crafting(Craft::open(
            station,
            name,
            &self.game,
            Arc::clone(&self.names),
        ));
    }

    /// Opens the repair counter of the merchant at `index` of [`App::repairers_here`].
    fn open_repairs(&mut self, index: usize) {
        let Some((vendor, name)) = self.repairers_here().into_iter().nth(index) else {
            return;
        };
        self.screen = Screen::Repairing(Repair::open(
            vendor,
            name,
            &self.game,
            Arc::clone(&self.names),
        ));
    }

    /// Sleeps a full rest in the bed at `index` of [`App::beds_here`], and says what
    /// it restored, or why it could not be done.
    fn sleep(&mut self, index: usize) {
        let Some((bed, name)) = self.beds_here().into_iter().nth(index) else {
            return;
        };
        let Some(span) = u32::try_from(FULL_REST_HOURS)
            .ok()
            .and_then(NonZeroU32::new)
            .map(Span::hours)
        else {
            return;
        };
        let note = match self.game.sleep(&bed, span) {
            Ok(healed) => format!(
                "You sleep {FULL_REST_HOURS} hours in {name} and recover {healed} hit points. \
                 It is now {}.",
                self.game.clock()
            ),
            Err(error) => error.to_string(),
        };
        self.log.push(Entry::Note(note));
        self.autosave();
    }

    /// Walks to a linked place and describes the trip, or says why it was refused.
    /// Arriving changes who and what the scene offers, so they are looked up again.
    fn travel_to(&mut self, to: &LocationId) {
        match self.game.travel(to) {
            Ok(span) => {
                self.here = self.places.here(to);
                self.people = self.places.people(&self.here);
                self.place = self.places.name(to);
                self.place_id = to.clone();
                self.log.push(Entry::Note(format!(
                    "You walk to {} ({}). It is now {}.",
                    self.place,
                    walk_time(span),
                    self.game.clock()
                )));
            }
            Err(error) => self.log.push(Entry::Note(error.to_string())),
        }
        self.screen = self.exploring();
        self.autosave();
    }

    /// Opens the list of saves for saving or loading.
    fn open_saves(&mut self, purpose: Purpose) {
        // An older game's single save joins the folder, so every save is in one place.
        match self.saves.adopt_legacy() {
            Ok(Some(name)) => self.log.push(Entry::Note(format!(
                "The old save was moved into the saves folder as \"{name}\"."
            ))),
            Ok(None) => {}
            Err(error) => self.log.push(Entry::Note(format!("Old save: {error}"))),
        }
        let places = &self.places;
        self.screen = Screen::Saves(Slots::open(purpose, &self.saves, |place| {
            places.name(place)
        }));
    }

    /// Does what the saves screen asked for.
    fn after_saves(&mut self, next: saves::Next) {
        match next {
            saves::Next::Stay => {}
            saves::Next::Leave => self.screen = self.exploring(),
            saves::Next::Save(name) => self.save_game(&SaveSlot::Named(name)),
            saves::Next::Load(slot) => self.load_game(&slot),
            saves::Next::Delete(slot) => {
                let note = match self.saves.delete(&slot) {
                    Ok(()) => format!("Deleted \"{}\".", slot.label()),
                    Err(error) => format!("Could not delete: {error}"),
                };
                self.after_change(note);
            }
            saves::Next::Rename { from, to } => {
                let note = match self.saves.rename(&from, &to) {
                    Ok(()) => format!("Renamed \"{from}\" to \"{to}\"."),
                    Err(error) => format!("Could not rename: {error}"),
                };
                self.after_change(note);
            }
        }
    }

    /// Says what happened to a save and opens the list again, so it shows the folder
    /// as it is now.
    fn after_change(&mut self, note: String) {
        let purpose = match &self.screen {
            Screen::Saves(slots) => slots.purpose(),
            _ => Purpose::Load,
        };
        self.log.push(Entry::Note(note));
        self.open_saves(purpose);
    }

    /// Writes the game to a slot and says how it went. The write replaces the file
    /// whole or not at all, so a failure keeps the old save.
    fn save_game(&mut self, slot: &SaveSlot) {
        let note = match self.saves.write(slot, &self.game.save()) {
            Ok(()) => format!("Game saved as \"{}\".", slot.label()),
            Err(error) => format!("Could not save: {error}"),
        };
        self.log.push(Entry::Note(note));
        self.screen = self.exploring();
    }

    /// Saves by itself, when that is switched on and the player is alive. A failure
    /// is said in the log, never silent, and never stops the game.
    fn autosave(&mut self) {
        if !self.autosave || !self.game.is_alive() {
            return;
        }
        let note = match self.saves.write_automatic(&self.game.save()) {
            Ok(()) => "Autosaved.".to_owned(),
            Err(error) => format!("Could not autosave: {error}"),
        };
        self.log.push(Entry::Note(note));
    }

    /// Replaces the game with the saved one, if the file loads. A file that is
    /// missing, damaged or from another version leaves the game as it is. Loading
    /// moves the player, so the scene's people and shops are looked up again.
    fn load_game(&mut self, slot: &SaveSlot) {
        let loaded = self
            .saves
            .read(slot)
            .and_then(|save| self.game.restore(&save))
            .map_err(|error| format!("Could not load: {error}"));
        match loaded {
            Ok(game) => {
                let place = game.at().cloned().unwrap_or_else(|| self.place_id.clone());
                self.game = game;
                self.here = self.places.here(&place);
                self.people = self.places.people(&self.here);
                self.place = self.places.name(&place);
                self.place_id = place;
                self.log.push(Entry::Note(format!(
                    "Game loaded. You are in {}. It is {}.",
                    self.place,
                    self.game.clock()
                )));
            }
            Err(note) => self.log.push(Entry::Note(note)),
        }
        self.screen = self.exploring();
    }

    fn exploring(&self) -> Screen {
        let shops = self
            .shops_here()
            .into_iter()
            .map(|(_, name)| name)
            .collect();
        let beds = self.beds_here().into_iter().map(|(_, name)| name).collect();
        let finds = self
            .finds_here()
            .into_iter()
            .map(|(_, name)| name)
            .collect();
        let stations = self
            .stations_here()
            .into_iter()
            .map(|(_, name)| name)
            .collect();
        let repairers = self
            .repairers_here()
            .into_iter()
            .map(|(_, name)| name)
            .collect();
        let fights = self
            .game
            .encounters_at(&self.place_id)
            .into_iter()
            .map(|encounter| encounter.name.clone())
            .collect();
        let character = self.game.character();
        let (points, picks) = (character.skill_points(), character.perk_picks());
        let label = if points + picks > 0 {
            format!("Level up ({points} skill point(s), {picks} perk pick(s))")
        } else {
            "Character".to_owned()
        };
        Screen::Exploring(
            Exploration::new(self.place.clone(), self.people.clone())
                .with_shops(shops)
                .with_fights(fights)
                .with_beds(beds)
                .with_finds(finds)
                .with_stations(stations)
                .with_repairers(repairers)
                .with_character_label(label),
        )
    }

    /// Wields the starting weapon when the hand is empty. If it is not carried yet
    /// nothing happens, and starting the fight says why it cannot begin.
    fn take_weapon_in_hand(&mut self) {
        if self.game.inventory().equipped(Slot::Weapon).is_some() {
            return;
        }
        if let Ok(weapon) = ItemId::try_new(START_WEAPON) {
            let _ = self.game.equip(&weapon);
        }
    }

    fn start_fight(&mut self, index: usize) {
        let Some(id) = self
            .game
            .encounters_at(&self.place_id)
            .get(index)
            .map(|encounter| encounter.id.clone())
        else {
            return;
        };
        self.take_weapon_in_hand();
        match self.game.start_battle(&id) {
            Ok(battle) => {
                self.log.push(Entry::Note("Guns are drawn.".to_owned()));
                let fight = Fight::new(
                    battle,
                    Arc::clone(&self.names),
                    &mut self.log,
                    &mut self.dice,
                );
                self.screen = if fight.is_over() {
                    // The quick foes finished it before the player could act.
                    let outcome = match fight.finish(&mut self.game, &mut self.log) {
                        combat::Next::Done(outcome) => outcome,
                        combat::Next::Stay => None,
                    };
                    self.after_fight(outcome)
                } else {
                    Screen::Fighting(fight)
                };
            }
            Err(error) => self.log.push(Entry::Note(error.to_string())),
        }
    }

    /// The screen after a fight is settled: the game over screen when the player is
    /// down, the scene otherwise.
    fn after_fight(&mut self, outcome: Option<Outcome>) -> Screen {
        match outcome {
            Some(Outcome::Defeat) => Screen::Down,
            Some(_) => {
                self.autosave();
                self.exploring()
            }
            None => self.exploring(),
        }
    }

    fn start_talk(&mut self, index: usize) {
        let Some(person) = self.people.get(index) else {
            return;
        };
        if let Err(error) = self.game.meet(&person.npc) {
            self.log.push(Entry::Note(error.to_string()));
            return;
        }
        match Talk::open(
            Arc::clone(&self.dialogue),
            &person.dialogue,
            person.name.clone(),
            &self.game,
        ) {
            Ok(talk) => self.screen = Screen::Talking(talk),
            Err(error) => self.log.push(Entry::Note(error.to_string())),
        }
    }

    /// Draws the persistent layout and the active screen inside it.
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        let Some(regions) = layout::regions(area) else {
            render_too_small(frame, area);
            return;
        };
        if let Screen::Creating(creation) = &self.screen {
            creation.render(frame, area);
            return;
        }
        let active = self.game.quests().active().count();
        frame.render_widget(
            Paragraph::new(format!(
                " {} | {} | Lv {} | HP {}/{} | {} caps{} | {active} active quest(s)",
                self.place,
                self.game.clock(),
                self.game.character().level().get(),
                self.game.hp(),
                self.game.character().max_hp(),
                self.game.caps(),
                ailments_note(&self.game),
            ))
            .style(theme::heading()),
            regions.status,
        );
        let keys = match &self.screen {
            Screen::Creating(_) => "",
            Screen::Exploring(scene) => {
                scene.render(frame, regions.scene);
                "Up/Down move | Enter choose | 1-9 pick | q quit"
            }
            Screen::Talking(talk) => {
                talk.render(frame, regions.scene);
                "Up/Down move | Enter say | 1-9 pick | Esc walk away | q quit"
            }
            Screen::Saves(slots) => {
                slots.render(frame, regions.scene);
                slots.keys()
            }
            Screen::Fighting(fight) => {
                fight.render(frame, regions.scene);
                "Up/Down move | Enter choose | 1-9 pick | q quit"
            }
            Screen::Leveling(sheet) => {
                sheet.render(frame, regions.scene, &self.game);
                "Up/Down move | Enter choose | 1-9 pick | Esc back | q quit"
            }
            Screen::Journaling(journal) => {
                journal.render(frame, regions.scene, &self.game);
                journal.keys()
            }
            Screen::Standing(standing) => {
                standing.render(frame, regions.scene, &self.game);
                standing.keys()
            }
            Screen::Traveling(travel) => {
                travel.render(frame, regions.scene, &self.game);
                travel.keys()
            }
            Screen::Packing(pack) => {
                pack.render(frame, regions.scene, &self.game);
                pack.keys()
            }
            Screen::Trading { trade, .. } => {
                trade.render(frame, regions.scene, &self.game);
                trade.keys()
            }
            Screen::Crafting(craft) => {
                craft.render(frame, regions.scene, &self.game);
                craft.keys()
            }
            Screen::Repairing(repairs) => {
                repairs.render(frame, regions.scene, &self.game);
                repairs.keys()
            }
            Screen::Down => {
                render_down(frame, regions.scene);
                "Enter or Esc to leave | q quit"
            }
        };
        log::render(frame, regions.log, &self.log);
        quests::render(frame, regions.quests, &self.game);
        frame.render_widget(
            Paragraph::new(format!(" {keys}")).style(theme::dim()),
            regions.footer,
        );
    }
}

fn render_down(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::styled("You are down.", theme::warning()),
        Line::default(),
        Line::styled(
            "Your journey ends in the dust of Goodsprings.",
            theme::text(),
        ),
    ];
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(theme::dim())
        .title(" Game over ");
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_too_small(frame: &mut Frame, area: Rect) {
    let message = format!(
        "Terminal too small: {}x{}. Need at least {MIN_WIDTH}x{MIN_HEIGHT}.",
        area.width, area.height
    );
    frame.render_widget(
        Paragraph::new(Line::styled(message, theme::warning())).alignment(Alignment::Center),
        area,
    );
}

/// " | 276 rads | 40 poison | 1 addiction(s)" for a player who has any, and nothing
/// for one who has none, so the status line stays short for a healthy player.
fn ailments_note(game: &Game) -> String {
    let mut note = String::new();
    if game.rads().get() > 0 {
        note.push_str(&format!(" | {} rads", game.rads().get()));
    }
    if game.poison() > 0 {
        note.push_str(&format!(" | {} poison", game.poison()));
    }
    if game.addictions().any() {
        note.push_str(&format!(" | {} addiction(s)", game.addictions().len()));
    }
    note
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use engine::shared::Scope;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    fn app() -> App {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
        let place = LocationId::try_new("goodsprings").expect("valid id");
        let data = GameData::load(&dir, &Scope::Around(place.clone())).expect("data loads");
        App::new(&data, &place).expect("the app builds")
    }

    fn app_with_seed(seed: u64) -> App {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
        let place = LocationId::try_new("goodsprings").expect("valid id");
        let data = GameData::load(&dir, &Scope::Around(place.clone())).expect("data loads");
        App::with_dice(&data, &place, Rng::new(seed)).expect("the app builds")
    }

    /// Doc has handed over the pistol and Sunny has joined the line, so the Powder
    /// Gangers can be fought.
    fn with_the_gunfight_open(mut app: App) -> App {
        let stage = |quest: &str, stage| Effect::SetStage {
            quest: QuestId::try_new(quest).expect("valid id"),
            stage: QuestStage::new(stage),
        };
        // The town is fully prepared, so the fight is the short one: Cobb and two
        // gangers. The unprepared fight has six attackers.
        let helped = |flag: &str| Effect::SetFlag {
            flag: engine::shared::FlagId::try_new(flag).expect("valid id"),
            value: true,
        };
        let steps = [
            stage(OPENING_QUEST, 50),
            stage("ghost_town_gunfight", 5),
            stage("ghost_town_gunfight", 10),
            stage("ghost_town_gunfight", 60),
            stage("ghost_town_gunfight", 65),
            helped("trudy_joined_militia"),
            helped("easy_pete_joined_militia"),
            helped("chet_supplied_militia"),
        ];
        apply_all(&steps, &mut app.game).expect("the quests advance");
        app.screen = app.exploring();
        app
    }

    fn screen(app: &App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
        terminal.draw(|frame| app.render(frame)).expect("draws");
        let buffer = terminal.backend().buffer();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn the_first_frame_shows_the_place_the_people_the_quest_and_the_keys() {
        // Tall enough for every row of the town, which now lists finds, stations and
        // a repairer too.
        let text = screen(&app(), 100, 50);
        for expected in [
            "You are in Goodsprings",
            "Talk to Doc Mitchell",
            "Talk to Sunny Smiles",
            "Quit game",
            "Ain't That a Kick in the Head",
            "Up/Down move",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
        }
    }

    #[test]
    fn the_pack_opens_from_the_scene_shows_the_worn_armor_and_leaves_again() {
        let mut app = app();
        // The cursor wraps up from the first row to Quit; save, load, the two waits,
        // travel, reputation and the journal sit between it and Pack.
        for _ in 0..9 {
            app.update(Command::Up);
        }
        app.update(Command::Select);
        assert!(matches!(app.screen, Screen::Packing(_)));
        let text = screen(&app, 100, 30);
        for expected in ["Your pack", "Leather armor", "[worn]", "Carrying"] {
            assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
        }
        app.update(Command::Back);
        assert!(matches!(app.screen, Screen::Exploring(_)));
    }

    /// Moves the scene's cursor to the row that says `label` and chooses it.
    fn choose_row(app: &mut App, label: &str) {
        for _ in 0..40 {
            if selected_row(app).contains(label) {
                app.update(Command::Select);
                return;
            }
            app.update(Command::Down);
        }
        panic!("no row says {label:?}");
    }

    /// The text of the highlighted row of the scene.
    fn selected_row(app: &App) -> String {
        screen(app, 100, 40)
            .lines()
            .find(|line| line.contains("> "))
            .map(str::to_owned)
            .unwrap_or_default()
    }

    #[test]
    fn the_journal_opens_from_the_scene_lists_the_opening_quest_and_leaves_again() {
        let mut app = app();
        choose_row(&mut app, "Journal");
        assert!(matches!(app.screen, Screen::Journaling(_)));
        let text = screen(&app, 100, 30);
        for expected in ["Journal", "[active] Ain't That a Kick in the Head", "Entry"] {
            assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
        }
        app.update(Command::Back);
        assert!(matches!(app.screen, Screen::Exploring(_)));
    }

    #[test]
    fn the_reputation_screen_opens_from_the_scene_and_leaves_again() {
        let mut app = app();
        choose_row(&mut app, "Reputation");
        assert!(matches!(app.screen, Screen::Standing(_)));
        let text = screen(&app, 100, 30);
        for expected in ["Karma:", "Goodsprings: Neutral"] {
            assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
        }
        app.update(Command::Back);
        assert!(matches!(app.screen, Screen::Exploring(_)));
    }

    #[test]
    fn travelling_moves_the_player_the_clock_and_the_scene_together() {
        let mut app = app();
        let start = app.game.clock();
        choose_row(&mut app, "Travel");
        assert!(matches!(app.screen, Screen::Traveling(_)));
        let text = screen(&app, 100, 30);
        assert!(text.contains("Where do you go?"), "{text}");
        assert!(text.contains("Goodsprings cave"), "{text}");
        // Enter walks to the nearest place.
        app.update(Command::Select);
        assert!(matches!(app.screen, Screen::Exploring(_)));
        assert_eq!(app.game.at(), Some(&app.place_id));
        assert_ne!(app.place, "Goodsprings");
        assert!(app.game.clock().as_minutes() > start.as_minutes());
        assert!(app.people.is_empty(), "nobody lives in the wilds");
        let text = screen(&app, 100, 30);
        assert!(
            text.contains(&format!("You are in {}", app.place)),
            "{text}"
        );
        assert!(text.contains("You walk to"), "{text}");
    }

    #[test]
    fn walking_back_brings_the_town_and_its_people_back() {
        let mut app = app();
        let people = app.people.len();
        choose_row(&mut app, "Travel");
        app.update(Command::Select);
        choose_row(&mut app, "Travel");
        let text = screen(&app, 100, 30);
        assert!(text.contains("Goodsprings"), "{text}");
        app.update(Command::Pick(
            engine_routes(&app)
                .iter()
                .position(|name| name == "Goodsprings")
                .expect("a way back to the town"),
        ));
        assert_eq!(app.place, "Goodsprings");
        assert_eq!(app.people.len(), people);
        assert_eq!(app.game.at(), Some(&app.place_id));
    }

    /// The names of the ways out of where the player stands, in the order offered.
    fn engine_routes(app: &App) -> Vec<String> {
        app.game.routes().into_iter().map(|r| r.name).collect()
    }

    #[test]
    fn a_player_the_town_likes_sees_the_discount_in_trudys_shop() {
        let mut app = app();
        let town = engine::shared::FactionId::try_new("goodsprings").expect("valid id");
        apply_all(
            &[Effect::SetReputation {
                faction: town,
                level: engine::shared::ReputationLevel::Accepted,
            }],
            &mut app.game,
        )
        .expect("the town tracks reputation");
        let trudy = VendorId::try_new("trudy").expect("valid id");
        app.open_shop(&trudy);
        assert!(matches!(app.screen, Screen::Trading { .. }));
        assert!(
            screen(&app, 110, 30).contains("25% off for you"),
            "{}",
            screen(&app, 110, 30)
        );
    }

    #[test]
    fn chet_gives_no_cut_however_much_the_town_likes_the_player() {
        let mut app = app();
        let town = engine::shared::FactionId::try_new("goodsprings").expect("valid id");
        apply_all(
            &[Effect::SetReputation {
                faction: town,
                level: engine::shared::ReputationLevel::Idolized,
            }],
            &mut app.game,
        )
        .expect("the town tracks reputation");
        let chet = VendorId::try_new("chet").expect("valid id");
        app.open_shop(&chet);
        assert!(!screen(&app, 110, 30).contains("off for you"));
    }

    #[test]
    fn a_stranger_pays_the_list_price_in_chets_shop() {
        let mut app = app();
        let chet = VendorId::try_new("chet").expect("valid id");
        app.open_shop(&chet);
        assert!(!screen(&app, 110, 30).contains("off for you"));
    }

    #[test]
    fn a_new_game_opens_on_character_creation_and_beginning_swaps_in_the_character() {
        use engine::shared::{CharacterView, Skill, Special};

        let mut app = app().with_creation();
        let text = screen(&app, MIN_WIDTH, MIN_HEIGHT);
        assert!(text.contains("Create your character"), "{text}");
        assert!(text.contains("Begin (locked)"), "{text}");
        // Beginning too early is refused and the screen stays.
        app.update(Command::Pick(3));
        assert!(matches!(app.screen, Screen::Creating(_)));
        // Five points into Strength, then tag the first three skills.
        app.update(Command::Pick(0));
        for _ in 0..5 {
            app.update(Command::Right);
        }
        app.update(Command::Back);
        app.update(Command::Pick(1));
        for _ in 0..3 {
            app.update(Command::Select);
            app.update(Command::Down);
        }
        app.update(Command::Back);
        app.update(Command::Pick(3));
        assert!(matches!(app.screen, Screen::Exploring(_)));
        let character = app.game.character();
        assert_eq!(character.special(Special::Strength).get(), 10);
        assert!(character.is_tagged(Skill::Barter));
        let text = screen(&app, 100, 30);
        assert!(text.contains("Your story begins."), "{text}");
        assert_eq!(app.game.character().carry_weight(), 250);
    }

    #[test]
    fn the_status_line_shows_the_level_and_the_caps() {
        let text = screen(&app(), 100, 30);
        assert!(text.contains("Lv 1 | HP 200/200 | 0 caps"), "{text}");
    }

    #[test]
    fn the_status_line_shows_radiation_and_addictions_only_when_there_are_some() {
        let well = screen(&app(), 140, 30);
        assert!(
            !well.contains("rads") && !well.contains("addiction"),
            "{well}"
        );
        let mut sick = app();
        sick.game = sick
            .game
            .clone()
            .with_rads(engine::shared::Rads::try_new(276).expect("in range"))
            .with_addiction(
                &engine::addiction::AddictionKind::try_new("jet").expect("a valid kind"),
            );
        let text = screen(&sick, 140, 30);
        assert!(text.contains("| 276 rads | 1 addiction(s) |"), "{text}");
    }

    #[test]
    fn the_status_line_shows_the_day_and_time() {
        let text = screen(&app(), 100, 30);
        assert!(text.contains("| Day 1, 08:00 | Lv 1"), "{text}");
    }

    #[test]
    fn waiting_moves_the_clock_and_the_log_says_when_it_is_now() {
        let mut app = app();
        // The cursor wraps up from the first row to Quit; load and save sit above it,
        // then the 8-hour wait.
        for _ in 0..4 {
            app.update(Command::Up);
        }
        app.update(Command::Select);
        assert_eq!(app.game.clock().to_string(), "Day 1, 16:00");
        let text = screen(&app, 100, 30);
        assert!(
            text.contains("Wait 8 hours. It is now Day 1, 16:00."),
            "{text}"
        );
        assert!(text.contains("| Day 1, 16:00 | Lv 1"), "{text}");
        assert!(matches!(app.screen, Screen::Exploring(_)));
    }

    /// The scene row of the first bed offered.
    fn sleep_row(app: &App) -> usize {
        app.people.len() + app.shops_here().len() + app.game.encounters_at(&app.place_id).len()
    }

    #[test]
    fn sleeping_in_the_bed_heals_the_player_moves_the_clock_and_the_log_says_so() {
        let mut app = app();
        app.game = app.game.clone().with_wounds(50);
        app.screen = app.exploring();
        app.update(Command::Pick(sleep_row(&app)));
        assert_eq!(app.game.hp(), app.game.character().max_hp());
        assert_eq!(app.game.clock().to_string(), "Day 1, 16:00");
        let text = screen(&app, 100, 34);
        // The log wraps at the panel width, so the sentence is checked in two parts.
        assert!(text.contains("recover 50"), "{text}");
        assert!(text.contains("It is now Day 1, 16:00."), "{text}");
        assert!(matches!(app.screen, Screen::Exploring(_)));
    }

    #[test]
    fn a_place_without_a_bed_offers_no_sleep() {
        let mut app = app();
        app.game = app.game.clone().with_beds(Arc::default());
        app.screen = app.exploring();
        assert!(app.beds_here().is_empty());
        assert!(!screen(&app, 100, 34).contains("Sleep in"));
    }

    #[test]
    fn the_cursor_row_stays_visible_when_the_town_lists_more_rows_than_fit() {
        let mut app = app();
        // The cursor wraps up from the first row to Quit, which a short scene cuts off
        // until it scrolls.
        app.update(Command::Up);
        let text = screen(&app, MIN_WIDTH, MIN_HEIGHT);
        assert!(text.contains("Quit game"), "{text}");
    }

    #[test]
    fn the_layout_fits_the_smallest_supported_terminal() {
        let text = screen(&app(), MIN_WIDTH, MIN_HEIGHT);
        assert!(text.contains("Talk to Doc Mitchell"));
        assert!(text.contains("q quit"));
    }

    #[test]
    fn a_tiny_terminal_gets_a_message_instead_of_a_broken_layout() {
        assert!(screen(&app(), 40, 10).contains("Terminal too small"));
    }

    #[test]
    fn talking_to_doc_mitchell_shows_his_menu_and_goodbye_returns_to_the_scene() {
        let mut app = app();
        let people: Vec<_> = app.people.iter().map(|p| p.name.clone()).collect();
        let doc = people
            .iter()
            .position(|n| n == "Doc Mitchell")
            .expect("Doc is here");
        app.update(Command::Pick(doc));
        let text = screen(&app, 100, 30);
        assert!(text.contains("You are talking to Doc Mitchell"), "{text}");
        assert!(text.contains("What town is this?"), "{text}");

        // "Goodbye" is the last start topic.
        let Screen::Talking(talk) = &app.screen else {
            panic!("expected a conversation");
        };
        let last = talk.choice_count() - 1;
        app.update(Command::Pick(last.min(8)));
        if last > 8 {
            panic!("goodbye is beyond the pick keys");
        }
        assert!(matches!(app.screen, Screen::Exploring(_)));
        assert!(screen(&app, 100, 30).contains("Goodbye"));
    }

    #[test]
    fn a_topic_said_goes_to_the_log_with_the_speaker() {
        let mut app = app();
        let doc = app
            .people
            .iter()
            .position(|p| p.name == "Doc Mitchell")
            .expect("Doc");
        app.update(Command::Pick(doc));
        app.update(Command::Pick(0));
        let text = screen(&app, 100, 30);
        assert!(text.contains("Doc Mitchell:"), "{text}");
    }

    #[test]
    fn the_player_starts_armored_with_full_health_and_takes_the_pistol_when_a_fight_starts() {
        let mut app = app();
        let item = |text| ItemId::try_new(text).expect("valid id");
        assert!(app.game.is_equipped(&item(START_ARMOR)));
        assert_eq!(app.game.hp(), app.game.character().max_hp());
        assert!(!app.game.is_equipped(&item(START_WEAPON)));

        app = with_the_gunfight_open(app);
        let fight = app.people.len() + app.shops_here().len();
        app.update(Command::Pick(fight));
        assert!(app.game.is_equipped(&item(START_WEAPON)));
    }

    #[test]
    fn a_fight_without_a_weapon_is_refused_with_the_reason() {
        let mut app = app();
        let stage = |quest: &str, stage| Effect::SetStage {
            quest: QuestId::try_new(quest).expect("valid id"),
            stage: QuestStage::new(stage),
        };
        apply_all(
            &[
                stage("ghost_town_gunfight", 5),
                stage("ghost_town_gunfight", 10),
                stage("ghost_town_gunfight", 60),
                stage("ghost_town_gunfight", 65),
            ],
            &mut app.game,
        )
        .expect("the quest advances");
        app.screen = app.exploring();
        let fight = app.people.len() + app.shops_here().len();
        app.update(Command::Pick(fight));
        assert!(matches!(app.screen, Screen::Exploring(_)));
        assert!(screen(&app, 100, 30).contains("nothing is equipped"));
    }

    #[test]
    fn the_gunfight_is_not_offered_until_the_town_is_ready() {
        let app = app();
        assert!(!screen(&app, 100, 30).contains("Fight the Powder Gangers"));
        let app = with_the_gunfight_open(app);
        assert!(screen(&app, 100, 30).contains("Fight the Powder Gangers"));
    }

    #[test]
    fn choosing_the_fight_opens_the_combat_screen_with_every_fighter_and_the_actions() {
        let mut app = with_the_gunfight_open(app_with_seed(1));
        let fight = app.people.len() + app.shops_here().len();
        app.update(Command::Pick(fight));
        assert!(matches!(app.screen, Screen::Fighting(_)));
        let text = screen(&app, 100, 40);
        for expected in [
            "Round 1 | your turn | 80 AP left",
            "You",
            "On your side: Sunny Smiles",
            "Joe Cobb",
            "Powder Ganger 1",
            "Powder Ganger 2",
            "Shoot Joe Cobb",
            "End turn",
            "HP 200/200",
            "Ammo 13/17",
            "Reload",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
        }
    }

    #[test]
    fn shooting_uses_rounds_and_reloading_refills_the_clip() {
        let mut app = with_the_gunfight_open(app_with_seed(1));
        let fight = app.people.len() + app.shops_here().len();
        app.update(Command::Pick(fight));
        // The actions are a shot at each foe still standing, then Reload, Use an
        // item, Run away and End turn, so Reload is fourth from the end.
        app.update(Command::Pick(0));
        let text = screen(&app, 100, 40);
        assert!(text.contains("Ammo 12/17"), "{text}");
        let Screen::Fighting(fight) = &app.screen else {
            panic!("the fight is still on");
        };
        let reload = fight.action_count() - 4;
        app.update(Command::Pick(reload));
        let text = screen(&app, 100, 40);
        assert!(text.contains("Ammo 13/16"), "{text}");
        assert!(text.contains("You reload: 13 rounds"), "{text}");
    }

    fn carried(app: &App, item: &str) -> u32 {
        app.game
            .inventory()
            .item_count(&engine::shared::ItemId::try_new(item).expect("valid id"))
    }

    #[test]
    fn using_an_item_in_a_fight_lists_the_pack_and_spends_the_item_only_when_it_works() {
        let mut app = with_the_gunfight_open(app_with_seed(1));
        give(&mut app, "stimpak", 2);
        give(&mut app, "radaway", 1);
        let fight = app.people.len() + app.shops_here().len();
        app.update(Command::Pick(fight));
        let items = |app: &App| {
            let Screen::Fighting(fight) = &app.screen else {
                panic!("the fight is on");
            };
            // "Use an item" sits before "Run away" and "End turn".
            fight.action_count() - 3
        };
        app.update(Command::Pick(items(&app)));
        let text = screen(&app, 100, 40);
        assert!(text.contains("Use which item? (2 AP)"), "{text}");
        assert!(
            text.contains("RadAway x1") && text.contains("Stimpak x2"),
            "{text}"
        );
        // RadAway is refused (no rads to clear): kept, said, and the list stays open.
        app.update(Command::Pick(0));
        let text = screen(&app, 100, 40);
        assert!(text.contains("Use which item?"), "{text}");
        assert_eq!(carried(&app, "radaway"), 1);
        // Esc goes back to the fight without spending anything.
        app.update(Command::Back);
        assert!(screen(&app, 100, 40).contains("What do you do?"));
        assert_eq!((carried(&app, "stimpak"), carried(&app, "radaway")), (2, 1));
        // The foes shot first, so a stimpak heals what the fight took.
        app.update(Command::Pick(items(&app)));
        app.update(Command::Pick(1));
        let text = screen(&app, 100, 40);
        assert!(text.contains("You use the Stimpak and recover"), "{text}");
        assert!(
            text.contains("78 AP left"),
            "two action points spent: {text}"
        );
        assert_eq!(carried(&app, "stimpak"), 1);
    }

    #[test]
    fn with_nothing_usable_the_fight_says_so_and_stays_on_the_menu() {
        let mut app = with_the_gunfight_open(app_with_seed(1));
        let fight = app.people.len() + app.shops_here().len();
        app.update(Command::Pick(fight));
        let Screen::Fighting(fight) = &app.screen else {
            panic!("the fight is on");
        };
        let at = fight.action_count() - 3;
        app.update(Command::Pick(at));
        let text = screen(&app, 100, 40);
        assert!(text.contains("You carry nothing you can use."), "{text}");
        assert!(text.contains("What do you do?"), "{text}");
    }

    #[test]
    fn an_auto_inject_stimpak_goes_off_in_the_scene_once_health_is_low() {
        let mut app = app();
        give(&mut app, "auto_inject_stimpak", 1);
        let max = app.game.character().max_hp();
        app.game = app.game.clone().with_wounds(max - max / 4);
        // Any command gives the scene its moment to check.
        app.update(Command::Down);
        assert_eq!(carried(&app, "auto_inject_stimpak"), 0);
        assert!(app.game.hp() > max / 4);
        let text = screen(&app, 100, 50);
        assert!(text.contains("It goes off by itself."), "{text}");
    }

    #[test]
    fn the_back_key_points_at_the_run_away_action_during_a_fight() {
        let mut app = with_the_gunfight_open(app_with_seed(1));
        let fight = app.people.len() + app.shops_here().len();
        app.update(Command::Pick(fight));
        app.update(Command::Back);
        assert!(matches!(app.screen, Screen::Fighting(_)));
        assert!(screen(&app, 100, 30).contains("Run away"));
    }

    #[test]
    fn running_away_ends_the_fight_unwon_and_the_fight_stays_on_offer() {
        let mut escapes = 0;
        for seed in 1..=8 {
            let mut app = with_the_gunfight_open(app_with_seed(seed));
            let fight = app.people.len() + app.shops_here().len();
            app.update(Command::Pick(fight));
            for _ in 0..60 {
                let Screen::Fighting(fight) = &app.screen else {
                    break;
                };
                // "Run away" sits just before "End turn", which is last.
                app.update(Command::Pick(fight.action_count() - 2));
            }
            if matches!(app.screen, Screen::Exploring(_)) {
                let text = screen(&app, 100, 30);
                // Five allies can finish the fight before a roll gets the player out;
                // that is a win, not an escape, and says nothing about running away.
                if text.contains("The fight is won.") {
                    continue;
                }
                escapes += 1;
                assert!(text.contains("You got away from the fight."), "{text}");
                assert!(text.contains("Fight the Powder Gangers"), "{text}");
                assert!(app.game.is_alive());
            }
        }
        assert!(escapes > 0, "some seed gets away");
    }

    #[test]
    fn the_run_away_row_shows_its_chance() {
        let mut app = with_the_gunfight_open(app_with_seed(1));
        let fight = app.people.len() + app.shops_here().len();
        app.update(Command::Pick(fight));
        let text = screen(&app, 100, 40);
        assert!(text.contains("Run away ("), "{text}");
        assert!(text.contains("% to get out)"), "{text}");
    }

    #[test]
    fn a_fight_played_to_the_end_leaves_the_scene_or_the_game_over_screen() {
        for seed in 1..=6 {
            let mut app = with_the_gunfight_open(app_with_seed(seed));
            let fight = app.people.len() + app.shops_here().len();
            app.update(Command::Pick(fight));
            // Shoot the first foe four times (80 AP buys four 17-AP shots), then
            // pass; the fight must end in a bounded number of rounds.
            for _ in 0..500 {
                for step in 0..5 {
                    // Passing is the last action, and the list shrinks as foes fall.
                    let Screen::Fighting(fight) = &app.screen else {
                        break;
                    };
                    let pick = if step < 4 {
                        0
                    } else {
                        fight.action_count() - 1
                    };
                    app.update(Command::Pick(pick.min(8)));
                }
                if !matches!(app.screen, Screen::Fighting(_)) {
                    break;
                }
            }
            let text = screen(&app, 100, 30);
            match app.screen {
                Screen::Exploring(_) => {
                    assert!(text.contains("The fight is won."), "{text}");
                    assert!(!text.contains("Fight the Powder Gangers"), "{text}");
                    assert!(app.game.is_alive());
                }
                Screen::Down => {
                    assert!(text.contains("You are down."), "{text}");
                    assert_eq!(app.game.hp(), 0);
                }
                _ => panic!("seed {seed}: the fight did not end"),
            }
            let hp = format!("HP {}/{}", app.game.hp(), app.game.character().max_hp());
            assert!(text.contains(&hp), "seed {seed}: {text}");
        }
    }

    /// Enough experience for level 3, which banks skill points and a perk pick.
    fn with_levels_waiting(mut app: App) -> App {
        apply_all(&[Effect::Xp { amount: 550.0 }], &mut app.game).expect("experience applies");
        app.screen = app.exploring();
        app
    }

    /// Opens the character screen from the scene, where its row follows the people,
    /// the shops, the fights, the beds, the finds, the stations and the repairers.
    fn open_character(app: &mut App) {
        let row = app.people.len()
            + app.shops_here().len()
            + app.game.encounters_at(&app.place_id).len()
            + app.beds_here().len()
            + app.finds_here().len()
            + app.stations_here().len()
            + app.repairers_here().len();
        app.update(Command::Pick(row));
        assert!(matches!(app.screen, Screen::Leveling(_)));
    }

    #[test]
    fn the_scene_offers_the_character_screen_and_says_when_points_are_waiting() {
        let app = app();
        assert!(screen(&app, 100, 50).contains("Character"));
        let app = with_levels_waiting(app);
        assert!(app.game.character().skill_points() > 0);
        assert!(app.game.character().perk_picks() > 0);
        assert!(screen(&app, 100, 50).contains("Level up ("));
    }

    #[test]
    fn spending_a_skill_point_raises_the_skill_and_logs_it() {
        let mut app = with_levels_waiting(app());
        open_character(&mut app);
        let before = app.game.character().skill_points();
        app.update(Command::Pick(0));
        app.update(Command::Pick(3));
        assert_eq!(app.game.character().skill_points(), before - 1);
        let text = screen(&app, 100, 30);
        assert!(text.contains("Guns rises to"), "{text}");
    }

    #[test]
    fn a_skill_point_with_none_banked_is_refused_with_the_reason() {
        let mut app = app();
        open_character(&mut app);
        app.update(Command::Pick(0));
        app.update(Command::Pick(0));
        let text = screen(&app, 100, 30);
        assert!(text.contains("no skill points to spend"), "{text}");
        assert!(text.contains("(locked)"), "{text}");
    }

    #[test]
    fn taking_a_perk_spends_the_pick_and_logs_it() {
        let mut app = with_levels_waiting(app());
        open_character(&mut app);
        let picks = app.game.character().perk_picks();
        app.update(Command::Pick(1));
        let text = screen(&app, 100, 30);
        assert!(text.contains("Take a perk:"), "{text}");
        app.update(Command::Pick(0));
        assert_eq!(app.game.character().perk_picks(), picks - 1);
        assert_eq!(app.game.character().perks().count(), 1);
        assert!(screen(&app, 100, 30).contains("You take"));
    }

    #[test]
    fn a_perk_with_no_pick_waiting_is_refused_with_the_reason() {
        let mut app = with_levels_waiting(app());
        open_character(&mut app);
        app.update(Command::Pick(1));
        for _ in 0..app.game.character().perk_picks() {
            app.update(Command::Pick(0));
        }
        assert_eq!(app.game.character().perk_picks(), 0);
        app.update(Command::Pick(0));
        assert!(screen(&app, 100, 30).contains("no perk to choose yet"));
    }

    #[test]
    fn escape_steps_back_to_the_top_menu_and_then_to_the_scene() {
        let mut app = app();
        open_character(&mut app);
        app.update(Command::Pick(0));
        app.update(Command::Back);
        assert!(matches!(app.screen, Screen::Leveling(_)));
        assert!(screen(&app, 100, 30).contains("What do you do?"));
        app.update(Command::Back);
        assert!(matches!(app.screen, Screen::Exploring(_)));
        assert!(app.is_running());
    }

    #[test]
    fn the_character_screen_fits_the_smallest_terminal_with_every_skill_reachable() {
        let mut app = app();
        open_character(&mut app);
        app.update(Command::Pick(0));
        for _ in 0..12 {
            app.update(Command::Down);
        }
        let text = screen(&app, MIN_WIDTH, MIN_HEIGHT);
        assert!(text.contains("Unarmed"), "{text}");
    }

    #[test]
    fn a_perk_the_game_does_not_model_says_so_before_it_is_taken() {
        let mut app = with_levels_waiting(app());
        open_character(&mut app);
        app.update(Command::Pick(1));
        let unmodeled = app
            .game
            .perks()
            .open_to(app.game.character())
            .iter()
            .position(|perk| perk.modifiers.is_empty())
            .expect("an open perk with no modeled effect");
        for _ in 0..unmodeled {
            app.update(Command::Down);
        }
        assert!(screen(&app, 100, 40).contains("changes nothing"));
    }

    fn press_trade_row(app: &mut App, vendor: &str) {
        let index = app
            .shops_here()
            .iter()
            .position(|(id, _)| id.as_str() == vendor)
            .expect("the shop is in town");
        app.update(Command::Pick(app.people.len() + index));
    }

    #[test]
    fn the_scene_lists_the_goodsprings_shops_and_choosing_one_opens_the_trade_screen() {
        let mut app = app();
        let text = screen(&app, 100, 30);
        for expected in [
            "Trade at Chet's general store",
            "Trade at Trudy's saloon",
            "Trade at Doc Mitchell's medical supplies",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
        }
        press_trade_row(&mut app, "chet");
        assert!(matches!(app.screen, Screen::Trading { from: None, .. }));
        let text = screen(&app, 100, 30);
        assert!(text.contains("Your pack"), "{text}");
        assert!(text.contains("Enter trade"), "{text}");
        app.update(Command::Back);
        assert!(matches!(app.screen, Screen::Exploring(_)));
    }

    #[test]
    fn buying_in_a_shop_spends_caps_and_the_status_line_shows_it() {
        let mut app = app();
        apply_all(&[Effect::Caps { amount: 500.0 }], &mut app.game).expect("caps apply");
        press_trade_row(&mut app, "chet");
        // 12 gauge buckshot is the first line of Chet's stock; the stack asks how many.
        app.update(Command::Select);
        app.update(Command::Select);
        let text = screen(&app, 100, 30);
        assert!(text.contains("You buy 1"), "{text}");
        assert!(!text.contains("| 500 caps"), "{text}");
    }

    #[test]
    fn a_topic_that_opens_a_shop_returns_to_the_conversation_afterwards() {
        // Each try starts a fresh conversation, because choosing a topic changes the
        // menu (and "goodbye" ends it).
        let opened = (0..12).find_map(|pick| {
            let mut app = app();
            let doc = app
                .people
                .iter()
                .position(|person| person.name == "Doc Mitchell")
                .expect("Doc is here");
            app.update(Command::Pick(doc));
            app.update(Command::Pick(pick));
            matches!(app.screen, Screen::Trading { .. }).then_some(app)
        });
        let mut app = opened.expect("one of Doc's topics opens his shop");
        let Screen::Trading { from, .. } = &app.screen else {
            panic!("expected the shop");
        };
        assert!(from.is_some());
        app.update(Command::Back);
        assert!(
            matches!(app.screen, Screen::Talking(_)),
            "leaving the shop should resume the conversation"
        );
    }

    #[test]
    fn quit_stops_the_app_from_any_screen() {
        let mut app = app();
        app.update(Command::Pick(0));
        assert!(app.is_running());
        app.update(Command::Quit);
        assert!(!app.is_running());
    }

    #[test]
    fn escape_walks_away_from_a_conversation_without_quitting() {
        let mut app = app();
        app.update(Command::Pick(0));
        app.update(Command::Back);
        assert!(app.is_running());
        assert!(matches!(app.screen, Screen::Exploring(_)));
    }

    /// A folder of its own for one test, removed afterwards.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            Self(std::env::temp_dir().join(format!("fnv-tui-{name}-{}", std::process::id())))
        }

        /// The saves folder, inside a folder of the test's own so that the old save
        /// beside it is the test's too.
        fn dir(&self) -> PathBuf {
            self.0.join("saves")
        }

        fn file(&self, name: &str) -> PathBuf {
            self.dir().join(format!("{name}.json"))
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).ok();
        }
    }

    /// Types `text` into the open name field and presses Enter.
    fn type_name(app: &mut App, text: &str) {
        assert!(app.is_typing(), "a name field is open");
        for c in text.chars() {
            app.type_key(KeyCode::Char(c));
        }
        app.type_key(KeyCode::Enter);
    }

    /// Saves the game under `name` through the screens.
    fn save_as(app: &mut App, name: &str) {
        choose_row(app, "Save game");
        app.update(Command::Select);
        type_name(app, name);
    }

    #[test]
    fn saving_asks_for_a_name_writes_that_file_and_the_log_says_so() {
        let scratch = Scratch::new("save");
        let mut app = app().with_save_dir(scratch.dir());
        choose_row(&mut app, "Save game");
        assert!(matches!(app.screen, Screen::Saves(_)));
        assert!(!app.is_typing());
        let text = screen(&app, 120, 40);
        assert!(text.contains("New save..."), "{text}");
        app.update(Command::Select);
        assert!(app.is_typing());
        type_name(&mut app, "camp one");
        assert!(scratch.file("camp one").is_file());
        let text = screen(&app, 120, 40);
        assert!(text.contains("Game saved as \"camp one\""), "{text}");
        assert!(matches!(app.screen, Screen::Exploring(_)));
        assert!(!app.is_typing());
    }

    #[test]
    fn a_name_that_cannot_be_used_says_why_and_the_field_stays_open() {
        let scratch = Scratch::new("badname");
        let mut app = app().with_save_dir(scratch.dir());
        choose_row(&mut app, "Save game");
        app.update(Command::Select);
        app.type_key(KeyCode::Enter);
        assert!(app.is_typing());
        assert!(
            screen(&app, 120, 40).contains("needs a name"),
            "{}",
            screen(&app, 120, 40)
        );
        type_name(&mut app, "AutoSave");
        assert!(app.is_typing());
        assert!(screen(&app, 120, 40).contains("kept for the automatic save"));
        assert!(!scratch.file("AutoSave").exists() && !scratch.file("autosave").exists());
    }

    #[test]
    fn letters_typed_into_a_name_are_not_commands_and_other_characters_are_ignored() {
        let scratch = Scratch::new("letters");
        let mut app = app().with_save_dir(scratch.dir());
        choose_row(&mut app, "Save game");
        app.update(Command::Select);
        // 'q', 'j' and '1' are commands elsewhere; here they are part of a name, and
        // characters that could escape the folder are dropped.
        type_name(&mut app, "q../j/1\\x");
        assert!(
            scratch.file("qj1x").is_file(),
            "{:?}",
            std::fs::read_dir(scratch.dir()).map(|d| d.count())
        );
        assert!(app.is_running());
    }

    #[test]
    fn backspace_erases_and_escape_cancels_the_name_then_leaves_the_list() {
        let scratch = Scratch::new("escape");
        let mut app = app().with_save_dir(scratch.dir());
        choose_row(&mut app, "Save game");
        app.update(Command::Select);
        app.type_key(KeyCode::Char('a'));
        app.type_key(KeyCode::Char('b'));
        app.type_key(KeyCode::Backspace);
        assert!(screen(&app, 120, 40).contains("Name: a_"));
        app.type_key(KeyCode::Esc);
        assert!(!app.is_typing());
        assert!(matches!(app.screen, Screen::Saves(_)));
        app.update(Command::Back);
        assert!(matches!(app.screen, Screen::Exploring(_)));
        assert!(
            !scratch.dir().exists()
                || std::fs::read_dir(scratch.dir()).is_ok_and(|d| d.count() == 0)
        );
    }

    #[test]
    fn loading_a_named_save_brings_back_its_time_and_the_log_says_so() {
        let scratch = Scratch::new("load");
        let mut app = app().with_save_dir(scratch.dir());
        save_as(&mut app, "start");
        choose_row(&mut app, "Wait 8 hours");
        assert_eq!(app.game.clock().to_string(), "Day 1, 16:00");
        choose_row(&mut app, "Load game");
        let text = screen(&app, 120, 40);
        assert!(text.contains("start - Day 1, 08:00"), "{text}");
        app.update(Command::Select);
        assert_eq!(app.game.clock().to_string(), "Day 1, 08:00");
        let text = screen(&app, 120, 40);
        assert!(text.contains("Game loaded."), "{text}");
        assert!(matches!(app.screen, Screen::Exploring(_)));
    }

    #[test]
    fn several_saves_are_listed_and_each_loads_its_own_game() {
        let scratch = Scratch::new("several");
        let mut app = app().with_save_dir(scratch.dir());
        save_as(&mut app, "morning");
        choose_row(&mut app, "Wait 8 hours");
        save_as(&mut app, "evening");
        choose_row(&mut app, "Wait 8 hours");
        choose_row(&mut app, "Load game");
        let text = screen(&app, 120, 40);
        assert!(
            text.contains("morning") && text.contains("evening"),
            "{text}"
        );
        // The most recently written save comes first.
        app.update(Command::Pick(1));
        assert_eq!(app.game.clock().to_string(), "Day 1, 08:00");
        choose_row(&mut app, "Load game");
        app.update(Command::Pick(0));
        assert_eq!(app.game.clock().to_string(), "Day 1, 16:00");
    }

    #[test]
    fn saving_over_a_name_asks_first_and_keeps_the_old_save_on_no() {
        let scratch = Scratch::new("overwrite");
        let mut app = app().with_save_dir(scratch.dir());
        save_as(&mut app, "slot");
        let first = std::fs::read_to_string(scratch.file("slot")).expect("written");
        choose_row(&mut app, "Wait 8 hours");
        save_as(&mut app, "slot");
        assert!(matches!(app.screen, Screen::Saves(_)), "it asks first");
        assert!(screen(&app, 120, 40).contains("Write over \"slot\""));
        app.update(Command::Back);
        assert_eq!(
            std::fs::read_to_string(scratch.file("slot")).expect("kept"),
            first
        );
        // Choosing the existing save from the list asks the same question.
        app.update(Command::Pick(1));
        app.update(Command::Select);
        assert_ne!(
            std::fs::read_to_string(scratch.file("slot")).expect("written"),
            first
        );
        assert!(matches!(app.screen, Screen::Exploring(_)));
    }

    #[test]
    fn loading_puts_the_scene_back_where_the_save_was_made() {
        let scratch = Scratch::new("place");
        let mut app = app().with_save_dir(scratch.dir());
        save_as(&mut app, "home");
        let home = app.place_id.clone();
        let people_at_home = app.people.len();
        let away = app.game.routes()[0].to.clone();
        app.travel_to(&away);
        assert_eq!(app.place_id, away);
        choose_row(&mut app, "Load game");
        app.update(Command::Select);
        assert_eq!(app.place_id, home);
        assert_eq!(app.people.len(), people_at_home);
        assert_eq!(app.game.at(), Some(&home));
    }

    #[test]
    fn loading_with_no_saves_says_so_and_changes_nothing() {
        let scratch = Scratch::new("none");
        let mut app = app().with_save_dir(scratch.dir());
        choose_row(&mut app, "Wait an hour");
        let before = app.game.save();
        choose_row(&mut app, "Load game");
        assert!(screen(&app, 120, 40).contains("There are no saves yet."));
        app.update(Command::Select);
        assert_eq!(app.game.save(), before);
        app.update(Command::Back);
        assert!(matches!(app.screen, Screen::Exploring(_)));
    }

    #[test]
    fn a_damaged_save_is_listed_refused_and_the_game_goes_on() {
        let scratch = Scratch::new("damaged");
        std::fs::create_dir_all(scratch.dir()).expect("the folder is made");
        std::fs::write(scratch.file("broken"), "{ not a save").expect("the file is written");
        let mut app = app().with_save_dir(scratch.dir());
        let before = app.game.save();
        choose_row(&mut app, "Load game");
        let text = screen(&app, 120, 40);
        assert!(text.contains("broken - cannot be read"), "{text}");
        app.update(Command::Select);
        assert_eq!(app.game.save(), before);
        let text = screen(&app, 120, 40);
        assert!(text.contains("Could not load"), "{text}");
        assert!(matches!(app.screen, Screen::Exploring(_)));
    }

    #[test]
    fn nothing_is_saved_by_itself_unless_autosave_is_on() {
        let scratch = Scratch::new("noauto");
        let mut app = app().with_save_dir(scratch.dir());
        let away = app.game.routes()[0].to.clone();
        app.travel_to(&away);
        assert!(!scratch.file("autosave").exists());
    }

    #[test]
    fn walking_saves_by_itself_in_its_own_slot_and_leaves_named_saves_alone() {
        let scratch = Scratch::new("auto");
        let mut app = app().with_save_dir(scratch.dir()).with_autosave();
        save_as(&mut app, "mine");
        let mine = std::fs::read_to_string(scratch.file("mine")).expect("written");

        let away = app.game.routes()[0].to.clone();
        app.travel_to(&away);
        assert!(scratch.file("autosave").is_file());
        assert!(screen(&app, 120, 40).contains("Autosaved."));
        let after_walk = std::fs::read_to_string(scratch.file("autosave")).expect("written");

        let onward = app.game.routes()[0].to.clone();
        app.travel_to(&onward);
        assert_ne!(
            std::fs::read_to_string(scratch.file("autosave")).expect("written"),
            after_walk,
            "each safe moment writes it again"
        );
        assert_eq!(
            std::fs::read_to_string(scratch.file("mine")).expect("kept"),
            mine,
            "a named save is never touched"
        );
    }

    #[test]
    fn a_settled_fight_saves_by_itself_unless_the_player_is_down() {
        let scratch = Scratch::new("autofight");
        let mut app = app().with_save_dir(scratch.dir()).with_autosave();
        assert!(matches!(
            app.after_fight(Some(Outcome::Defeat)),
            Screen::Down
        ));
        assert!(!scratch.file("autosave").exists());
        assert!(matches!(
            app.after_fight(Some(Outcome::Fled)),
            Screen::Exploring(_)
        ));
        assert!(
            scratch.file("autosave").is_file(),
            "the player is alive, so it saves"
        );
    }

    #[test]
    fn the_autosave_is_listed_for_loading_but_never_offered_for_saving() {
        let scratch = Scratch::new("autolist");
        let mut app = app().with_save_dir(scratch.dir()).with_autosave();
        let away = app.game.routes()[0].to.clone();
        app.travel_to(&away);
        choose_row(&mut app, "Save game");
        assert!(!screen(&app, 120, 40).contains("Autosave (automatic)"));
        app.update(Command::Back);
        choose_row(&mut app, "Load game");
        assert!(screen(&app, 120, 40).contains("Autosave (automatic)"));
        app.update(Command::Select);
        assert!(screen(&app, 120, 40).contains("Game loaded."));
    }

    #[test]
    fn deleting_a_save_asks_first_then_removes_only_that_file() {
        let scratch = Scratch::new("delete");
        let mut app = app().with_save_dir(scratch.dir());
        save_as(&mut app, "keep");
        save_as(&mut app, "drop");
        choose_row(&mut app, "Load game");
        // The most recently written save is first: "drop".
        app.update(Command::Delete);
        assert!(screen(&app, 120, 40).contains("Delete \"drop\"?"));
        app.update(Command::Back);
        assert!(scratch.file("drop").is_file(), "Esc keeps it");
        app.update(Command::Delete);
        app.update(Command::Select);
        assert!(!scratch.file("drop").exists());
        assert!(scratch.file("keep").is_file());
        let text = screen(&app, 120, 40);
        assert!(text.contains("Deleted \"drop\"."), "{text}");
        assert!(
            text.contains("keep"),
            "the list shows the folder as it is now"
        );
        assert!(matches!(app.screen, Screen::Saves(_)));
    }

    #[test]
    fn renaming_a_save_moves_its_file_and_never_replaces_another() {
        let scratch = Scratch::new("rename");
        let mut app = app().with_save_dir(scratch.dir());
        save_as(&mut app, "first");
        save_as(&mut app, "second");
        choose_row(&mut app, "Load game");
        app.update(Command::Rename);
        assert!(app.is_typing(), "the new name is typed");
        for _ in 0.."second".len() {
            app.type_key(KeyCode::Backspace);
        }
        for c in "first".chars() {
            app.type_key(KeyCode::Char(c));
        }
        app.type_key(KeyCode::Enter);
        assert!(app.is_typing(), "a name that is taken is refused");
        assert!(screen(&app, 120, 40).contains("already exists"));
        assert!(scratch.file("second").is_file());
        for _ in 0.."first".len() {
            app.type_key(KeyCode::Backspace);
        }
        for c in "third".chars() {
            app.type_key(KeyCode::Char(c));
        }
        app.type_key(KeyCode::Enter);
        assert!(!scratch.file("second").exists());
        assert!(scratch.file("third").is_file());
        assert!(scratch.file("first").is_file());
        assert!(screen(&app, 120, 40).contains("Renamed \"second\" to \"third\"."));
    }

    #[test]
    fn the_autosave_keeps_the_one_before_it_and_both_can_be_loaded() {
        let scratch = Scratch::new("twoauto");
        let mut app = app().with_save_dir(scratch.dir()).with_autosave();
        let away = app.game.routes()[0].to.clone();
        app.travel_to(&away);
        let first = std::fs::read_to_string(scratch.file("autosave")).expect("written");
        let onward = app.game.routes()[0].to.clone();
        app.travel_to(&onward);
        assert_eq!(
            std::fs::read_to_string(scratch.file("autosave_previous")).expect("kept"),
            first
        );
        choose_row(&mut app, "Load game");
        let text = screen(&app, 120, 40);
        assert!(text.contains("Autosave (automatic)"), "{text}");
        assert!(text.contains("Previous autosave (automatic)"), "{text}");
    }

    #[test]
    fn the_old_single_save_moves_into_the_folder_as_a_named_save() {
        let scratch = Scratch::new("legacy");
        let mut app = app().with_save_dir(scratch.dir());
        std::fs::create_dir_all(&scratch.0).expect("the folder is made");
        app.game
            .save()
            .write(&scratch.0.join("save.json"))
            .expect("written");
        choose_row(&mut app, "Load game");
        assert!(screen(&app, 120, 40).contains("The old save was moved"));
        assert!(screen(&app, 120, 40).contains("Old save - "));
        assert!(!scratch.0.join("save.json").exists());
        app.update(Command::Select);
        assert!(screen(&app, 120, 40).contains("Game loaded."));
    }

    #[test]
    fn a_failed_autosave_is_said_and_the_game_goes_on() {
        let scratch = Scratch::new("autofail");
        // A file where the folder should be, so the folder cannot be made.
        std::fs::create_dir_all(&scratch.0).expect("the folder is made");
        let blocker = scratch.0.join("blocked");
        std::fs::write(&blocker, "x").expect("written");
        let mut app = app().with_save_dir(blocker).with_autosave();
        let away = app.game.routes()[0].to.clone();
        app.travel_to(&away);
        assert!(screen(&app, 120, 40).contains("Could not autosave"));
        assert_eq!(app.place_id, away);
    }

    fn give(app: &mut App, item: &str, count: u32) {
        let effect = Effect::GiveItem {
            item: ItemId::try_new(item).expect("valid id"),
            count: NonZeroU32::new(count).expect("a positive count"),
        };
        apply_all(&[effect], &mut app.game).expect("the item is given");
    }

    /// Wears the pistol down by `lost` points and puts it in the pack.
    fn with_a_worn_pistol(mut app: App, lost: u32) -> App {
        give(&mut app, "9mm_pistol", 1);
        let mut save = app.game.save();
        for weapon in &mut save.pack.weapons {
            weapon.lost = lost;
        }
        app.game = app.game.restore(&save).expect("the worn pistol loads");
        app.screen = app.exploring();
        app
    }

    #[test]
    fn the_scene_lists_the_pickups_the_campfire_and_the_repairer() {
        let text = screen(&app(), 100, 60);
        for expected in [
            "Pick up a broc flower",
            "Pick up a xander root",
            "Craft at the campfire at the Goodsprings source",
            "Craft at the workbench in the Goodsprings home",
            "repair a weapon",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
        }
    }

    #[test]
    fn picking_up_the_plants_and_making_powder_at_the_campfire_works_from_the_scene() {
        let mut app = app();
        choose_row(&mut app, "Pick up a broc flower");
        choose_row(&mut app, "Pick up a xander root");
        let text = screen(&app, 100, 60);
        assert!(
            !text.contains("Pick up a broc flower"),
            "a taken find is gone:\n{text}"
        );
        assert!(text.contains("You pick up a xander root"), "{text}");
        choose_row(&mut app, "Craft at the campfire");
        assert!(matches!(app.screen, Screen::Crafting(_)));
        let text = screen(&app, 100, 30);
        for expected in ["Healing powder", "[ready]", "Broc flower", "Uses up"] {
            assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
        }
        // Healing powder is the only campfire recipe the pack can make, but the
        // cursor starts on the first recipe in the list.
        while !selected_row(&app).contains("Healing powder") {
            app.update(Command::Down);
        }
        app.update(Command::Select);
        assert!(screen(&app, 100, 30).contains("You make Healing powder x1"));
        assert_eq!(
            app.game
                .inventory()
                .item_count(&ItemId::try_new("healing_powder").expect("valid id")),
            1
        );
        app.update(Command::Back);
        assert!(matches!(app.screen, Screen::Exploring(_)));
    }

    #[test]
    fn crafting_without_the_parts_says_what_is_missing_and_changes_nothing() {
        let mut app = app();
        choose_row(&mut app, "Craft at the campfire");
        while !selected_row(&app).contains("Healing powder") {
            app.update(Command::Down);
        }
        let before = app.game.save();
        app.update(Command::Select);
        assert_eq!(app.game.save(), before);
        assert!(
            screen(&app, 100, 30).contains("you carry 0 of"),
            "{}",
            screen(&app, 100, 30)
        );
    }

    #[test]
    fn the_repair_counter_quotes_a_price_and_mends_the_weapon() {
        let mut app = with_a_worn_pistol(app(), 140);
        apply_all(&[Effect::Caps { amount: 500.0 }], &mut app.game).expect("caps apply");
        choose_row(&mut app, "repair a weapon");
        assert!(matches!(app.screen, Screen::Repairing(_)));
        let text = screen(&app, 100, 30);
        assert!(text.contains("9mm pistol"), "{text}");
        assert!(text.contains("caps"), "{text}");
        let before = app.game.caps();
        app.update(Command::Select);
        assert!(app.game.caps() < before);
        assert!(screen(&app, 100, 30).contains("mends your 9mm pistol"));
        let pistol = ItemId::try_new("9mm_pistol").expect("valid id");
        let copy = app
            .game
            .inventory()
            .copies_of(&pistol)
            .next()
            .expect("the pistol is carried");
        assert!(
            app.game
                .copy_condition(copy)
                .is_some_and(|left| left.get() >= 50)
        );
        app.update(Command::Back);
        assert!(matches!(app.screen, Screen::Exploring(_)));
    }

    #[test]
    fn a_worn_weapon_and_a_kit_offer_mending_in_the_pack_and_two_copies_offer_a_merge() {
        let mut app = with_a_worn_pistol(app(), 100);
        give(&mut app, "weapon_repair_kit", 1);
        give(&mut app, "9mm_pistol", 1);
        choose_row(&mut app, "Pack");
        while !selected_row(&app).contains("9mm pistol") {
            app.update(Command::Down);
        }
        app.update(Command::Select);
        let text = screen(&app, 100, 40);
        for expected in ["Mend with a kit", "Merge another copy"] {
            assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
        }
    }
}
