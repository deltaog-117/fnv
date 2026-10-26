//! Composition root of the front end: the one place that knows every screen.
//!
//! [`App`] owns the game and the persistent log, decides which screen is active,
//! and draws the persistent layout around it. It reads no terminal and writes none
//! (the caller feeds it commands and hands it a frame), so tests drive it headless.

use std::num::NonZeroU32;
use std::sync::Arc;

use anyhow::Context;
use engine::character::PerkBook;
use engine::dialogue::{Conversation, DialogueBook};
use engine::game::{Arsenal, Game, Outcome};
use engine::inventory::{ItemBook, Slot};
use engine::quests::QuestBook;
use engine::shared::{
    Effect, GameData, InventoryView, ItemId, LocationId, QuestId, QuestStage, apply_all,
};
use engine::world::WorldBook;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::character::{self, LevelUp};
use crate::combat::{self, Fight};
use crate::dialogue::{self, Talk};
use crate::exploration::{self, Exploration, Person};
use crate::quests;
use crate::shared::dice::Rng;
use crate::shared::input::Command;
use crate::shared::layout::{self, MIN_HEIGHT, MIN_WIDTH};
use crate::shared::log::{self, Entry, Log};
use crate::shared::theme;

/// The quest a new game starts in, and the stage that opens it.
const OPENING_QUEST: &str = "ain_t_that_a_kick_in_the_head";
const OPENING_STAGE: u32 = 10;
/// What the player wields and wears. The opening quest hands over the pistol late
/// in Doc's scene; there is no inventory screen yet, so it is taken in hand when a
/// fight starts. The armor is a stand-in worn from the start until there is a place
/// to find or buy it, and without it the Goodsprings fight is not fair.
const START_WEAPON: &str = "9mm_pistol";
const START_ARMOR: &str = "leather_armor";

enum Screen {
    Exploring(Exploration),
    Talking(Talk),
    Fighting(Fight),
    Leveling(LevelUp),
    /// The player fell in a fight; nothing more can be done but quit.
    Down,
}

/// The whole front end's state.
pub struct App {
    game: Game,
    dialogue: Arc<DialogueBook>,
    place: String,
    place_id: LocationId,
    people: Vec<Person>,
    screen: Screen,
    log: Log,
    dice: Rng,
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
        let mut game = Game::with_world(quests, world)
            .with_items(items)
            .with_arsenal(arsenal)
            .with_perks(Arc::new(PerkBook::base_game(data)));
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
        let here =
            |npc_place: &LocationId| npc_place == place || location.children.contains(npc_place);
        let mut people: Vec<Person> = data
            .npcs
            .iter()
            .filter(|npc| npc.location_ids.iter().any(here))
            .filter_map(|npc| {
                // Most dialogue files are transcripts without a graph; only the
                // authored ones open.
                let dialogue_id = npc
                    .dialogue
                    .iter()
                    .find(|id| Conversation::open(Arc::clone(&dialogue), id).is_ok())?;
                Some(Person {
                    npc: npc.id.clone(),
                    name: npc.name.clone(),
                    dialogue: dialogue_id.clone(),
                })
            })
            .collect();
        people.sort_by(|a, b| a.name.cmp(&b.name));

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
            people,
            log,
            dice,
            running: true,
        };
        app.screen = app.exploring();
        Ok(app)
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
            Screen::Exploring(scene) => match scene.update(command) {
                exploration::Next::Stay => {}
                exploration::Next::Quit => self.running = false,
                exploration::Next::Talk(index) => self.start_talk(index),
                exploration::Next::Fight(index) => self.start_fight(index),
                exploration::Next::Character => self.screen = Screen::Leveling(LevelUp::new()),
            },
            Screen::Leveling(sheet) => {
                if sheet.update(command, &mut self.game, &mut self.log) == character::Next::Leave {
                    self.screen = self.exploring();
                }
            }
            Screen::Talking(talk) => {
                if talk.update(command, &mut self.game, &mut self.log) == dialogue::Next::Leave {
                    self.screen = self.exploring();
                }
            }
            Screen::Fighting(fight) => {
                if let combat::Next::Done(outcome) =
                    fight.update(command, &mut self.game, &mut self.log, &mut self.dice)
                {
                    self.screen = match outcome {
                        Some(Outcome::Defeat) => Screen::Down,
                        _ => self.exploring(),
                    };
                }
            }
            Screen::Down => {
                if matches!(command, Command::Select | Command::Back) {
                    self.running = false;
                }
            }
        }
    }

    fn exploring(&self) -> Screen {
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
                .with_fights(fights)
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
                self.screen = Screen::Fighting(Fight::new(battle));
            }
            Err(error) => self.log.push(Entry::Note(error.to_string())),
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
        let active = self.game.quests().active().count();
        frame.render_widget(
            Paragraph::new(format!(
                " {} | Lv {} | HP {}/{} | {} caps | {active} active quest(s)",
                self.place,
                self.game.character().level().get(),
                self.game.hp(),
                self.game.character().max_hp(),
                self.game.caps(),
            ))
            .style(theme::heading()),
            regions.status,
        );
        let keys = match &self.screen {
            Screen::Exploring(scene) => {
                scene.render(frame, regions.scene);
                "Up/Down move | Enter choose | 1-9 pick | q quit"
            }
            Screen::Talking(talk) => {
                talk.render(frame, regions.scene);
                "Up/Down move | Enter say | 1-9 pick | Esc walk away | q quit"
            }
            Screen::Fighting(fight) => {
                fight.render(frame, regions.scene);
                "Up/Down move | Enter choose | 1-9 pick | q quit"
            }
            Screen::Leveling(sheet) => {
                sheet.render(frame, regions.scene, &self.game);
                "Up/Down move | Enter choose | 1-9 pick | Esc back | q quit"
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
        let steps = [
            stage(OPENING_QUEST, 50),
            stage("ghost_town_gunfight", 5),
            stage("ghost_town_gunfight", 10),
            stage("ghost_town_gunfight", 60),
            stage("ghost_town_gunfight", 65),
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
        let text = screen(&app(), 100, 30);
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
    fn the_status_line_shows_the_level_and_the_caps() {
        let text = screen(&app(), 100, 30);
        assert!(text.contains("Lv 1 | HP 200/200 | 0 caps"), "{text}");
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
        let fight = app.people.len();
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
        let fight = app.people.len();
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
        let fight = app.people.len();
        app.update(Command::Pick(fight));
        assert!(matches!(app.screen, Screen::Fighting(_)));
        let text = screen(&app, 100, 30);
        for expected in [
            "Round 1 | your turn | 80 AP left",
            "You",
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
        let fight = app.people.len();
        app.update(Command::Pick(fight));
        // Three foes: shoot is picks 0 to 2, then Reload, then End turn.
        app.update(Command::Pick(0));
        let text = screen(&app, 100, 30);
        assert!(text.contains("Ammo 12/17"), "{text}");
        app.update(Command::Pick(3));
        let text = screen(&app, 100, 30);
        assert!(text.contains("Ammo 13/16"), "{text}");
        assert!(text.contains("You reload: 13 rounds"), "{text}");
    }

    #[test]
    fn running_away_is_refused_during_a_fight() {
        let mut app = with_the_gunfight_open(app_with_seed(1));
        let fight = app.people.len();
        app.update(Command::Pick(fight));
        app.update(Command::Back);
        assert!(matches!(app.screen, Screen::Fighting(_)));
        assert!(screen(&app, 100, 30).contains("no running"));
    }

    #[test]
    fn a_fight_played_to_the_end_leaves_the_scene_or_the_game_over_screen() {
        for seed in 1..=6 {
            let mut app = with_the_gunfight_open(app_with_seed(seed));
            let fight = app.people.len();
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

    /// Opens the character screen from the scene, where its row follows the people
    /// and the fights.
    fn open_character(app: &mut App) {
        let row = app.people.len() + app.game.encounters_at(&app.place_id).len();
        app.update(Command::Pick(row));
        assert!(matches!(app.screen, Screen::Leveling(_)));
    }

    #[test]
    fn the_scene_offers_the_character_screen_and_says_when_points_are_waiting() {
        let app = app();
        assert!(screen(&app, 100, 30).contains("Character"));
        let app = with_levels_waiting(app);
        assert!(app.game.character().skill_points() > 0);
        assert!(app.game.character().perk_picks() > 0);
        assert!(screen(&app, 100, 30).contains("Level up ("));
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
}
