//! The one state that `evaluate` and `apply_all` run against.

use std::collections::HashSet;
use std::num::NonZeroU32;
use std::sync::Arc;

use super::battle::{Arsenal, Battle, BattleError, Outcome, WEAR_PER_SHOT};
use crate::character::{Character, PerkBook, ProgressError};
use crate::clock::{ClockError, GameClock, Span};
use crate::combat::{Condition, Percent, Side, Status};
use crate::craft::{RecipeBook, StationBook};
use crate::effects::{Active, ActiveEffects};
use crate::gather::{FindBook, Gathered};
use crate::inventory::{CopyId, Inventory, InventoryError, ItemBook, Slot, WeaponCopy};
use crate::quests::{QuestBook, QuestLog};
use crate::reputation::{ReputationBook, Standing};
use crate::rest::{BedBook, RestError, restored};
use crate::shared::{
    Alignment, BedId, CharacterSink, CharacterView, CompanionId, CompanionSink, CompanionView,
    DiscountPercent, Effect, EffectError, Encounter, EncounterId, EndingId, FactionId, FlagId,
    HealthPercent, InventorySink, InventoryView, ItemId, ItemUse, LocationId, NpcId, NpcState,
    PerkId, QuestId, QuestSink, QuestStage, QuestStatus, QuestView, Renown, RenownTier,
    ReputationLevel, Skill, SkillLevel, Special, SpecialLevel, StandingSink, StandingView,
    VendorId, WorldSink, WorldView, apply, apply_all, evaluate,
};
use crate::trade::{
    Market, Shop, TradeError, VendorBook, buy_total, buy_total_with_discount, sell_total,
};
use crate::travel::{Route, TravelBook, TravelError};
use crate::world::{WorldBook, WorldState};

mod save;
mod upkeep;

pub use upkeep::{Crafted, RepairQuote, WornWeapon};

/// A quest effect that could not run because its feature is not built yet, kept
/// so the reward is not lost: it can be applied once the feature lands.
#[derive(Debug, Clone, PartialEq)]
pub struct Deferred {
    /// The quest whose stage or rule produced the effect.
    pub quest: QuestId,
    /// The effect that was postponed.
    pub effect: Effect,
    /// The missing feature.
    pub feature: &'static str,
}

/// Everything the player has done so far.
///
/// Cloning is cheap (the quest book is shared), so a caller can try a batch of
/// effects on a copy and keep it only if it works.
///
/// Quests are automated here, because stage effects and consequence rules need the
/// whole state: reaching a stage applies its `on_enter` effects, and every change
/// to a quest then fires that quest's rules whose conditions hold, once each.
#[derive(Debug, Clone)]
pub struct Game {
    book: Arc<QuestBook>,
    quests: QuestLog,
    world: WorldState,
    /// Fame, infamy and karma.
    standing: Standing,
    character: Character,
    inventory: Inventory,
    arsenal: Arc<Arsenal>,
    perks: Arc<PerkBook>,
    /// What each merchant has now; changes with every trade.
    market: Market,
    /// Where the player may sleep.
    beds: Arc<BedBook>,
    /// What can be made, and where the stations stand.
    recipes: Arc<RecipeBook>,
    stations: Arc<StationBook>,
    /// What lies at a place to be picked up once, and what has been picked up.
    finds: Arc<FindBook>,
    gathered: Gathered,
    /// What day and time it is; moves only through [`Game::wait`], [`Game::sleep`]
    /// and [`Game::travel`].
    clock: GameClock,
    /// Boosts from chems, drinks and meals, each with the minute it ends.
    effects: ActiveEffects,
    /// The places the player can walk between.
    map: Arc<TravelBook>,
    /// Where the player stands; `None` until the game is given a map.
    at: Option<LocationId>,
    /// Hit points lost and not yet healed. Stored as a loss, so hit points can
    /// never be above the maximum, and a level-up raises them with the maximum.
    wounds: u32,
    /// Encounters the player has won; each is offered once.
    won: HashSet<EncounterId>,
    /// Rules already fired, as (quest, index into its rules).
    fired: HashSet<(QuestId, usize)>,
    deferred: Vec<Deferred>,
}

impl Game {
    /// A new game: no quest started and no NPC in the world, so any effect aimed
    /// at an NPC is refused. Use [`Game::with_world`] for a populated world.
    #[must_use]
    pub fn new(book: Arc<QuestBook>) -> Self {
        Self::with_world(book, Arc::new(WorldBook::default()))
    }

    /// A new game over the quests and the NPCs that exist.
    #[must_use]
    pub fn with_world(book: Arc<QuestBook>, world: Arc<WorldBook>) -> Self {
        Self {
            quests: QuestLog::new(Arc::clone(&book)),
            book,
            world: WorldState::new(world),
            standing: Standing::new(Arc::default()),
            character: Character::default(),
            inventory: Inventory::new(Arc::default()),
            arsenal: Arc::default(),
            perks: Arc::default(),
            market: Market::default(),
            beds: Arc::default(),
            recipes: Arc::default(),
            stations: Arc::default(),
            finds: Arc::default(),
            gathered: Gathered::default(),
            clock: GameClock::START,
            effects: ActiveEffects::default(),
            map: Arc::default(),
            at: None,
            wounds: 0,
            won: HashSet::new(),
            fired: HashSet::new(),
            deferred: Vec::new(),
        }
    }

    /// The same game over the items that exist. Items are unknown until a book is
    /// given, so call this before play: it starts a fresh, empty pack.
    #[must_use]
    pub fn with_items(mut self, items: Arc<ItemBook>) -> Self {
        self.inventory = Inventory::new(items);
        self
    }

    /// The same game over the fights that exist. Without one there is nothing to
    /// fight, so call this before play.
    #[must_use]
    pub fn with_arsenal(mut self, arsenal: Arc<Arsenal>) -> Self {
        self.arsenal = arsenal;
        self
    }

    /// The same game over the perks and traits that exist. Without a book no perk
    /// can be taken, so call this before play.
    #[must_use]
    pub fn with_perks(mut self, perks: Arc<PerkBook>) -> Self {
        self.perks = perks;
        self
    }

    /// The same game over the beds that exist. Without a book there is nowhere to
    /// sleep, so call this before play.
    #[must_use]
    pub fn with_beds(mut self, beds: Arc<BedBook>) -> Self {
        self.beds = beds;
        self
    }

    /// The same game over the recipes that exist. Without a book nothing can be
    /// made, so call this before play.
    #[must_use]
    pub fn with_recipes(mut self, recipes: Arc<RecipeBook>) -> Self {
        self.recipes = recipes;
        self
    }

    /// The same game over the crafting stations that exist. Without a book there is
    /// nowhere to craft, so call this before play.
    #[must_use]
    pub fn with_stations(mut self, stations: Arc<StationBook>) -> Self {
        self.stations = stations;
        self
    }

    /// The same game over the finds that exist. Without a book there is nothing to
    /// pick up, so call this before play.
    #[must_use]
    pub fn with_finds(mut self, finds: Arc<FindBook>) -> Self {
        self.finds = finds;
        self
    }

    /// The same game over the merchants that exist, each with the stock and caps
    /// they start with. Set the items first: every stocked item must have a base
    /// value in the item book the game holds now.
    ///
    /// # Errors
    ///
    /// [`TradeError::NoValue`] when a vendor stocks an item the book does not know
    /// or gives no value.
    pub fn with_vendors(mut self, vendors: Arc<VendorBook>) -> Result<Self, TradeError> {
        for (id, info) in vendors.iter() {
            if let Some((item, _)) = info
                .stock()
                .find(|(item, _)| self.inventory.value(item).is_none())
            {
                return Err(TradeError::NoValue {
                    vendor: id.clone(),
                    item: item.clone(),
                });
            }
        }
        self.market = Market::new(vendors);
        Ok(self)
    }

    /// The same game over a map, with the player standing at `start`. Without a
    /// map the player has no place and cannot travel, so call this before play.
    ///
    /// # Errors
    ///
    /// [`TravelError::NotOnMap`] when `start` is not on the map.
    pub fn with_map(
        mut self,
        map: Arc<TravelBook>,
        start: &LocationId,
    ) -> Result<Self, TravelError> {
        if !map.contains(start) {
            return Err(TravelError::NotOnMap(start.clone()));
        }
        self.map = map;
        self.at = Some(start.clone());
        Ok(self)
    }

    /// The same game over the factions that track reputation. Without a book no
    /// faction has a standing, so call this before play: it starts every score at
    /// zero.
    #[must_use]
    pub fn with_reputation(mut self, book: Arc<ReputationBook>) -> Self {
        self.standing = Standing::new(book);
        self
    }

    /// The same game for a different character, for example one with another
    /// S.P.E.C.I.A.L. spread. Experience already earned goes with the old one.
    #[must_use]
    pub fn with_character(mut self, character: Character) -> Self {
        self.character = character;
        self
    }

    /// The same game with the player already wounded by `wounds` hit points, never
    /// more than the character's maximum. Set the character first: the cap is
    /// taken from the one the game holds now.
    #[must_use]
    pub fn with_wounds(mut self, wounds: u32) -> Self {
        self.wounds = wounds.min(self.character.max_hp());
        self
    }

    /// Fame, infamy and karma, for the reputation screen.
    #[must_use]
    pub fn standing(&self) -> &Standing {
        &self.standing
    }

    /// The quest log, for the journal.
    #[must_use]
    pub fn quests(&self) -> &QuestLog {
        &self.quests
    }

    /// The story flags and NPC fates.
    #[must_use]
    pub fn world(&self) -> &WorldState {
        &self.world
    }

    /// The player character.
    #[must_use]
    pub fn character(&self) -> &Character {
        &self.character
    }

    /// The perks and traits that exist, for the picker.
    #[must_use]
    pub fn perks(&self) -> &PerkBook {
        &self.perks
    }

    /// Spends a skill point the player has earned.
    ///
    /// # Errors
    ///
    /// [`ProgressError`] when no point is banked or the skill is at its maximum.
    pub fn spend_skill_point(&mut self, skill: Skill) -> Result<(), ProgressError> {
        self.character.spend_skill_point(skill)
    }

    /// Takes a perk with a pick the player has earned.
    ///
    /// # Errors
    ///
    /// [`ProgressError`] when no pick is waiting, the perk is unknown or fully
    /// taken, or a requirement is unmet.
    pub fn choose_perk(&mut self, perk: &PerkId) -> Result<(), ProgressError> {
        self.character.choose_perk(&self.perks, perk)
    }

    /// What the player carries and wears.
    #[must_use]
    pub fn inventory(&self) -> &Inventory {
        &self.inventory
    }

    /// Whether the load is over the carry weight; being over blocks nothing here,
    /// but travel and running will read it.
    #[must_use]
    pub fn is_encumbered(&self) -> bool {
        self.inventory.weight() > f64::from(self.character.carry_weight())
    }

    /// The player's hit points now.
    #[must_use]
    pub fn hp(&self) -> u32 {
        self.character.max_hp().saturating_sub(self.wounds)
    }

    /// Whether the player still has hit points.
    #[must_use]
    pub fn is_alive(&self) -> bool {
        self.hp() > 0
    }

    /// What day and time it is.
    #[must_use]
    pub fn clock(&self) -> GameClock {
        self.clock
    }

    /// Lets `span` of time pass.
    ///
    /// # Errors
    ///
    /// [`ClockError::Down`] while the player has no hit points, and
    /// [`ClockError::Overflow`] when the time does not fit; the clock is left as it
    /// was.
    pub fn wait(&mut self, span: Span) -> Result<(), ClockError> {
        if !self.is_alive() {
            return Err(ClockError::Down);
        }
        let before = self.clock.as_minutes();
        self.clock.advance(span)?;
        self.market.restock(before, self.clock.as_minutes());
        self.expire_effects();
        Ok(())
    }

    /// The boosts in force now: what each changes, by how much, and the minute it
    /// ends.
    pub fn active_effects(&self) -> impl Iterator<Item = &Active> {
        self.effects.iter()
    }

    /// Ends the boosts whose time has come. The wounds stay as they are, so hit
    /// points gained from a boost to the maximum go with it. A player who was alive
    /// is left with at least one hit point, because a boost to the maximum is a loan
    /// that cannot kill when it is repaid.
    fn expire_effects(&mut self) {
        if self.effects.expire(self.clock.as_minutes()) {
            self.character.set_boosts(self.effects.tally());
            self.wounds = self.wounds.min(self.character.max_hp().saturating_sub(1));
        }
    }

    /// Where the player stands, once the game has a map.
    #[must_use]
    pub fn at(&self) -> Option<&LocationId> {
        self.at.as_ref()
    }

    /// The ways out of where the player stands, nearest first. None while the
    /// player is down or the game has no map.
    #[must_use]
    pub fn routes(&self) -> Vec<Route> {
        match (&self.at, self.is_alive()) {
            (Some(here), true) => self.map.routes_from(here),
            _ => Vec::new(),
        }
    }

    /// Walks to a linked place, and returns how long it took. The player moves and
    /// the clock advances together or not at all.
    ///
    /// # Errors
    ///
    /// [`TravelError::NoMap`] when the player has no place, [`TravelError::NoRoute`]
    /// when `to` is not linked to where they stand, and [`TravelError::Clock`] when
    /// time cannot pass (the player is down, or the time does not fit).
    pub fn travel(&mut self, to: &LocationId) -> Result<Span, TravelError> {
        let from = self.at.clone().ok_or(TravelError::NoMap)?;
        let span = self
            .map
            .span_between(&from, to)
            .ok_or_else(|| TravelError::NoRoute {
                from: from.clone(),
                to: to.clone(),
            })?;
        self.wait(span)?;
        self.at = Some(to.clone());
        Ok(span)
    }

    /// The beds the player can sleep in at `place`. None while the player is down.
    pub fn beds_at<'a>(
        &'a self,
        place: &'a LocationId,
    ) -> impl Iterator<Item = (&'a BedId, &'a str)> {
        let open = self.is_alive();
        self.beds
            .at(place)
            .filter(move |_| open)
            .map(|(id, info)| (id, info.name()))
    }

    /// Sleeps for `span` in `bed`, and returns the hit points it restored.
    ///
    /// # Errors
    ///
    /// [`RestError::UnknownBed`] when there is no such bed, and
    /// [`RestError::Clock`] when time cannot pass (the player is down, or the time
    /// does not fit). Nothing changes in either case.
    pub fn sleep(&mut self, bed: &BedId, span: Span) -> Result<u32, RestError> {
        if self.beds.get(bed).is_none() {
            return Err(RestError::UnknownBed(bed.clone()));
        }
        // Worked out before the time passes; the wait is the only step that can fail.
        let amount = restored(self.character.max_hp(), span);
        self.wait(span)?;
        let healed = amount.min(self.wounds);
        self.heal(healed);
        Ok(healed)
    }

    /// Restores up to `amount` hit points, never past the maximum.
    pub fn heal(&mut self, amount: u32) {
        self.wounds = self.wounds.saturating_sub(amount);
    }

    /// The fights on offer at `place`: not yet won, and with every requirement met.
    /// None while the player is down.
    #[must_use]
    pub fn encounters_at(&self, place: &LocationId) -> Vec<&Encounter> {
        if !self.is_alive() {
            return Vec::new();
        }
        self.arsenal
            .encounters()
            .filter(|encounter| {
                &encounter.place == place
                    && !self.won.contains(&encounter.id)
                    && encounter
                        .requires
                        .iter()
                        .all(|condition| evaluate(condition, self))
            })
            .collect()
    }

    /// Starts a fight, with the player's equipped weapon and armor and the hit
    /// points they have now. The game itself is not changed until
    /// [`Game::finish_battle`].
    ///
    /// # Errors
    ///
    /// [`BattleError`] when the encounter is unknown or not on offer at its place,
    /// the player is down or has nothing to fight with.
    pub fn start_battle(&self, encounter: &EncounterId) -> Result<Battle, BattleError> {
        let definition = self
            .arsenal
            .encounter(encounter)
            .ok_or_else(|| BattleError::UnknownEncounter(encounter.clone()))?;
        let hp = NonZeroU32::new(self.hp()).ok_or(BattleError::PlayerDown)?;
        if !self.encounters_at(&definition.place).contains(&definition) {
            return Err(BattleError::NotOffered(encounter.clone()));
        }
        let worn: Vec<&ItemId> = [Slot::Outfit, Slot::Head]
            .into_iter()
            .filter_map(|slot| self.inventory.equipped(slot))
            .collect();
        let wielded = self.inventory.equipped(Slot::Weapon);
        let copy = self.inventory.wielded();
        let weapon_wear = copy
            .and_then(|id| self.inventory.weapon(id))
            .map_or(0, WeaponCopy::lost);
        let player = self.arsenal.player(
            &self.character,
            wielded,
            &worn,
            &self.inventory,
            weapon_wear,
        )?;
        let prepared = self
            .arsenal
            .foes(encounter)
            .ok_or_else(|| BattleError::UnknownEncounter(encounter.clone()))?;
        // The prepared fighters follow the record's foes one for one.
        let present: Vec<_> = definition
            .foes
            .iter()
            .zip(prepared)
            .filter(|(foe, _)| foe.when.iter().all(|condition| evaluate(condition, self)))
            .map(|(_, fighter)| fighter.clone())
            .collect();
        Battle::start(encounter.clone(), player, hp, &present, copy)
    }

    /// Settles a finished fight: the player keeps the wounds they ended with, and
    /// a win applies the encounter's effects and takes it off offer. If those
    /// effects fail, nothing changes.
    ///
    /// # Errors
    ///
    /// [`BattleError::Unfinished`] while both sides still stand, and
    /// [`BattleError::Effects`] when a winning effect is refused.
    pub fn finish_battle(&mut self, battle: &Battle) -> Result<Outcome, BattleError> {
        let combat = battle.combat();
        let status = combat.status();
        if status == Status::Ongoing {
            return Err(BattleError::Unfinished);
        }
        let hp = combat.hp(battle.player()).unwrap_or(0);
        let mut trial = self.clone();
        trial.wounds = trial.character.max_hp().saturating_sub(hp);
        trial.spend_rounds(battle)?;
        trial.wear_weapon(battle);
        let outcome = if status == Status::Fled {
            Outcome::Fled
        } else if status == Status::Won(Side::Player) {
            let effects = self
                .arsenal
                .encounter(battle.encounter())
                .map(|encounter| encounter.on_win.clone())
                .unwrap_or_default();
            apply_all(&effects, &mut trial)?;
            trial.won.insert(battle.encounter().clone());
            Outcome::Victory
        } else {
            Outcome::Defeat
        };
        *self = trial;
        Ok(outcome)
    }

    /// Wears the very weapon the player fought with by the shots they made.
    fn wear_weapon(&mut self, battle: &Battle) {
        let Some(copy) = battle.weapon() else {
            return;
        };
        let Some(held) = self.inventory.weapon(copy) else {
            return;
        };
        let Some(max) = self.arsenal.condition_max(held.item()) else {
            return;
        };
        let worn =
            Condition::new(max, held.lost()).worn(battle.shots().saturating_mul(WEAR_PER_SHOT));
        // The copy was found just above, so the pack cannot refuse.
        let _ = self.inventory.set_lost(copy, worn.lost());
    }

    /// How much of a carried weapon's condition is left, from 0 to 100 percent;
    /// `None` for a weapon that is not carried or does not wear.
    #[must_use]
    pub fn copy_condition(&self, copy: CopyId) -> Option<Percent> {
        let held = self.inventory.weapon(copy)?;
        let max = self.arsenal.condition_max(held.item())?;
        Some(Condition::new(max, held.lost()).percent())
    }

    /// Takes the rounds the player fired out of the pack. The rounds in the weapon
    /// at the end go back, since the game keeps no loaded weapon between fights.
    fn spend_rounds(&mut self, battle: &Battle) -> Result<(), EffectError> {
        let combat = battle.combat();
        let player = battle.player();
        let (Some(fighter), Some(rounds)) = (combat.combatant(player), combat.rounds(player))
        else {
            return Ok(());
        };
        let (Some(item), Some(start), Some(left)) = (
            fighter.ammo_item(),
            fighter.starting_stock(),
            rounds.total(),
        ) else {
            return Ok(());
        };
        match NonZeroU32::new(start.saturating_sub(left)) {
            Some(spent) => self.inventory.take_item(item, spent),
            None => Ok(()),
        }
    }

    /// Equips a carried item. Of several copies of a weapon, the least worn goes in
    /// hand.
    ///
    /// # Errors
    ///
    /// [`InventoryError`] when the item is not carried or cannot be equipped.
    pub fn equip(&mut self, item: &ItemId) -> Result<Slot, InventoryError> {
        self.inventory.equip(item)
    }

    /// Puts one particular carried weapon in hand.
    ///
    /// # Errors
    ///
    /// [`InventoryError::NoSuchCopy`] when no carried weapon has that id.
    pub fn equip_copy(&mut self, copy: CopyId) -> Result<Slot, InventoryError> {
        self.inventory.equip_copy(copy)
    }

    /// Throws one particular carried weapon away.
    ///
    /// # Errors
    ///
    /// [`InventoryError::NoSuchCopy`] when no carried weapon has that id and
    /// [`InventoryError::Equipped`] when it is the one in hand.
    pub fn drop_copy(&mut self, copy: CopyId) -> Result<(), InventoryError> {
        self.inventory.discard_copy(copy)
    }

    /// Uses one carried consumable, returning the hit points it restored.
    ///
    /// An item that heals is spent only when there is something to heal: a player at
    /// full health keeps it, so a stray key press does not waste a stimpak. An item
    /// that boosts starts its boosts, which end by themselves as the clock moves.
    ///
    /// # Errors
    ///
    /// [`InventoryError::Down`] when the player has no hit points,
    /// [`InventoryError::NotHeld`] when the item is not carried,
    /// [`InventoryError::NoEffect`] when the game does not model what it does, and
    /// [`InventoryError::AtFullHealth`] when there is nothing to heal.
    pub fn use_item(&mut self, item: &ItemId) -> Result<u32, InventoryError> {
        if !self.is_alive() {
            return Err(InventoryError::Down);
        }
        if self.inventory.item_count(item) == 0 {
            return Err(InventoryError::NotHeld(item.clone()));
        }
        let Some(effect) = self.inventory.item_use(item) else {
            return Err(InventoryError::NoEffect(item.clone()));
        };
        let (heal, boosts) = match &effect {
            ItemUse::Heal(hp) => (Some(*hp), &[][..]),
            ItemUse::Boost(boost) => (boost.heal, boost.effects.as_slice()),
        };
        // An item that heals is kept by a player with nothing to heal, so a stray key
        // press does not waste it; one that only boosts is always worth using.
        if heal.is_some() && self.wounds == 0 {
            return Err(InventoryError::AtFullHealth);
        }
        let healed = heal.map_or(0, |hp| hp.get().min(self.wounds));
        self.inventory.discard(item, NonZeroU32::MIN)?;
        if !boosts.is_empty() {
            self.effects.start(item, boosts, self.clock.as_minutes());
            self.character.set_boosts(self.effects.tally());
        }
        self.heal(healed);
        Ok(healed)
    }

    /// Throws `count` copies of a carried item away.
    ///
    /// # Errors
    ///
    /// [`InventoryError`] when too few are carried or the last copy is equipped.
    pub fn drop_item(&mut self, item: &ItemId, count: NonZeroU32) -> Result<(), InventoryError> {
        self.inventory.discard(item, count)
    }

    /// Empties an equipment slot, returning what was in it.
    pub fn unequip(&mut self, slot: Slot) -> Option<ItemId> {
        self.inventory.unequip(slot)
    }

    /// The merchants the player can trade with at `place`: those whose person is
    /// neither dead nor hostile. None while the player is down.
    pub fn vendors_at<'a>(
        &'a self,
        place: &'a LocationId,
    ) -> impl Iterator<Item = (&'a VendorId, &'a str)> {
        let open = self.is_alive();
        self.market
            .book()
            .at(place)
            .filter(move |(_, info)| open && self.vendor_will_trade(info.npc()))
            .map(|(id, info)| (id, info.name()))
    }

    /// A vendor's stock and caps now.
    #[must_use]
    pub fn shop(&self, vendor: &VendorId) -> Option<&Shop> {
        self.market.shop(vendor)
    }

    /// Caps the player pays to buy `count` of an item at the current Barter skill.
    ///
    /// # Errors
    ///
    /// [`TradeError::NoPrice`] when the item has no base value.
    pub fn buy_price(&self, item: &ItemId, count: NonZeroU32) -> Result<u32, TradeError> {
        let value = self
            .inventory
            .value(item)
            .ok_or_else(|| TradeError::NoPrice(item.clone()))?;
        Ok(buy_total(value, count, self.character.skill(Skill::Barter)))
    }

    /// The percent off the player's purchases at this vendor right now: the vendor's
    /// cut while the player's standing earns it, and `None` otherwise.
    #[must_use]
    pub fn discount_at(&self, vendor: &VendorId) -> Option<DiscountPercent> {
        let discount = self.market.book().get(vendor)?.discount()?;
        let earned =
            discount.when.is_empty() || discount.when.iter().any(|rule| evaluate(rule, self));
        earned.then_some(discount.percent)
    }

    /// Caps the player pays a vendor for `count` of an item: [`Game::buy_price`]
    /// with the vendor's cut, if the player has earned one.
    ///
    /// # Errors
    ///
    /// [`TradeError::UnknownVendor`] when there is no such vendor, and
    /// [`TradeError::NoPrice`] when the item has no base value.
    pub fn buy_price_from(
        &self,
        vendor: &VendorId,
        item: &ItemId,
        count: NonZeroU32,
    ) -> Result<u32, TradeError> {
        if self.market.book().get(vendor).is_none() {
            return Err(TradeError::UnknownVendor(vendor.clone()));
        }
        let Some(percent) = self.discount_at(vendor) else {
            return self.buy_price(item, count);
        };
        let value = self
            .inventory
            .value(item)
            .ok_or_else(|| TradeError::NoPrice(item.clone()))?;
        Ok(buy_total_with_discount(
            value,
            count,
            self.character.skill(Skill::Barter),
            percent,
        ))
    }

    /// Caps the player is paid for `count` of an item at the current Barter skill.
    ///
    /// # Errors
    ///
    /// [`TradeError::NoPrice`] when the item has no base value or the line is
    /// worth less than half a cap.
    pub fn sell_price(&self, item: &ItemId, count: NonZeroU32) -> Result<u32, TradeError> {
        let total = self
            .inventory
            .value(item)
            .map(|value| sell_total(value, count, self.character.skill(Skill::Barter)))
            .filter(|total| *total > 0);
        total.ok_or_else(|| TradeError::NoPrice(item.clone()))
    }

    /// Buys `count` of an item from a vendor, returning the caps paid. The vendor
    /// and the player change together or not at all.
    ///
    /// # Errors
    ///
    /// [`TradeError`] when the vendor is unknown or will not trade, has too few of
    /// the item, or the player cannot pay.
    pub fn buy(
        &mut self,
        vendor: &VendorId,
        item: &ItemId,
        count: NonZeroU32,
    ) -> Result<u32, TradeError> {
        self.check_vendor(vendor)?;
        let price = self.buy_price_from(vendor, item, count)?;
        let have = self.inventory.caps();
        if have < price {
            return Err(TradeError::PlayerShort { have, need: price });
        }
        let mut trial = self.clone();
        trial.market.sell_to_player(vendor, item, count, price)?;
        trial.inventory.add_caps(-f64::from(price))?;
        trial.inventory.give_item(item, count)?;
        *self = trial;
        Ok(price)
    }

    /// Sells `count` of a carried item to a vendor, returning the caps received.
    /// The vendor and the player change together or not at all.
    ///
    /// # Errors
    ///
    /// [`TradeError`] when the vendor is unknown or will not trade, the player
    /// carries too few, the sale would give up an equipped copy, the item has no
    /// price, or the vendor cannot pay.
    pub fn sell(
        &mut self,
        vendor: &VendorId,
        item: &ItemId,
        count: NonZeroU32,
    ) -> Result<u32, TradeError> {
        self.check_vendor(vendor)?;
        let have = self.inventory.item_count(item);
        if have < count.get() {
            return Err(TradeError::NotCarried {
                item: item.clone(),
                have,
                need: count.get(),
            });
        }
        if have == count.get() && self.inventory.is_equipped(item) {
            return Err(TradeError::Equipped(item.clone()));
        }
        let price = self.sell_price(item, count)?;
        let mut trial = self.clone();
        trial.market.buy_from_player(vendor, item, count, price)?;
        trial.inventory.take_item(item, count)?;
        trial.inventory.add_caps(f64::from(price))?;
        *self = trial;
        Ok(price)
    }

    /// Sells one particular carried weapon to a vendor, returning the caps received.
    /// The vendor and the player change together or not at all. The vendor's stock
    /// gains a weapon like any other: a shop keeps no record of wear.
    ///
    /// # Errors
    ///
    /// [`TradeError`] when the vendor is unknown or will not trade, the player does
    /// not carry that weapon, it is the one in hand, it has no price, or the vendor
    /// cannot pay.
    pub fn sell_copy(&mut self, vendor: &VendorId, copy: CopyId) -> Result<u32, TradeError> {
        self.check_vendor(vendor)?;
        let item = self
            .inventory
            .weapon(copy)
            .map(|held| held.item().clone())
            .ok_or(TradeError::NoSuchWeapon(copy.get()))?;
        if self.inventory.wielded() == Some(copy) {
            return Err(TradeError::Equipped(item));
        }
        let price = self.sell_price(&item, NonZeroU32::MIN)?;
        let mut trial = self.clone();
        trial
            .market
            .buy_from_player(vendor, &item, NonZeroU32::MIN, price)?;
        trial
            .inventory
            .discard_copy(copy)
            .map_err(|_| TradeError::NoSuchWeapon(copy.get()))?;
        trial.inventory.add_caps(f64::from(price))?;
        *self = trial;
        Ok(price)
    }

    /// Refuses a vendor that does not exist or whose person will not trade.
    fn check_vendor(&self, vendor: &VendorId) -> Result<(), TradeError> {
        let info = self
            .market
            .book()
            .get(vendor)
            .ok_or_else(|| TradeError::UnknownVendor(vendor.clone()))?;
        if !self.is_alive() || !self.vendor_will_trade(info.npc()) {
            return Err(TradeError::Unavailable(info.npc().clone()));
        }
        Ok(())
    }

    fn vendor_will_trade(&self, npc: &NpcId) -> bool {
        !self.world.npc_is(npc, NpcState::Dead) && !self.world.npc_is(npc, NpcState::Hostile)
    }

    /// Quest effects postponed because their feature is not built, oldest first.
    #[must_use]
    pub fn deferred(&self) -> &[Deferred] {
        &self.deferred
    }

    /// Records that the player has met the NPC.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the NPC is unknown, dead or removed.
    pub fn meet(&mut self, npc: &NpcId) -> Result<(), EffectError> {
        self.world.meet(npc)
    }
}

impl Game {
    /// Runs a change to one quest, then its automation, as a unit: if any effect
    /// the change sets off fails, the game is left as it was.
    fn quest_change(
        &mut self,
        quest: &QuestId,
        change: impl FnOnce(&mut QuestLog) -> Result<(), EffectError>,
    ) -> Result<(), EffectError> {
        if !self.book.is_automated(quest) {
            return change(&mut self.quests);
        }
        let before = self.quests.quest_stage(quest);
        let mut trial = self.clone();
        change(&mut trial.quests)?;
        trial.react(quest, before)?;
        *self = trial;
        Ok(())
    }

    /// Applies what a change to `quest` sets off: the effects of the stage it just
    /// reached (only when the stage moved, so repeating a stage pays nothing
    /// twice), then each consequence rule that now holds and has not fired.
    fn react(&mut self, quest: &QuestId, before: Option<QuestStage>) -> Result<(), EffectError> {
        let book = Arc::clone(&self.book);
        let after = self.quests.quest_stage(quest);
        if after != before
            && let Some(stage) = after.and_then(|stage| book.stage(quest, stage))
        {
            self.run(quest, &stage.on_enter)?;
        }
        for (index, rule) in book.rules(quest).iter().enumerate() {
            if rule.effects.is_empty() || self.fired.contains(&(quest.clone(), index)) {
                continue;
            }
            if rule
                .conditions
                .iter()
                .all(|condition| evaluate(condition, &*self))
            {
                // Marked before it runs, so a rule whose effects touch its own
                // quest cannot fire itself again.
                self.fired.insert((quest.clone(), index));
                self.run(quest, &rule.effects)?;
            }
        }
        Ok(())
    }

    /// Applies effects one by one. A feature that is not built yet postpones its
    /// effect; any other failure stops the run.
    fn run(&mut self, quest: &QuestId, effects: &[Effect]) -> Result<(), EffectError> {
        for effect in effects {
            match apply(effect, self) {
                Err(EffectError::NotBuilt { feature, .. }) => self.deferred.push(Deferred {
                    quest: quest.clone(),
                    effect: effect.clone(),
                    feature,
                }),
                result => result?,
            }
        }
        Ok(())
    }
}

/// The refusal for an effect whose feature has no state yet. Removing one of these
/// is the signal that the feature landed and `Game` gained a field for it.
fn not_built(effect: &'static str, feature: &'static str) -> EffectError {
    EffectError::NotBuilt { effect, feature }
}

impl QuestView for Game {
    fn quest_status(&self, quest: &QuestId) -> QuestStatus {
        self.quests.quest_status(quest)
    }

    fn quest_stage(&self, quest: &QuestId) -> Option<QuestStage> {
        self.quests.quest_stage(quest)
    }
}

impl QuestSink for Game {
    fn start_quest(&mut self, quest: &QuestId) -> Result<(), EffectError> {
        self.quest_change(quest, |log| log.start_quest(quest))
    }

    fn complete_quest(&mut self, quest: &QuestId) -> Result<(), EffectError> {
        self.quest_change(quest, |log| log.complete_quest(quest))
    }

    fn fail_quest(&mut self, quest: &QuestId) -> Result<(), EffectError> {
        self.quest_change(quest, |log| log.fail_quest(quest))
    }

    fn block_quest(&mut self, quest: &QuestId) -> Result<(), EffectError> {
        self.quest_change(quest, |log| log.block_quest(quest))
    }

    fn set_stage(&mut self, quest: &QuestId, stage: QuestStage) -> Result<(), EffectError> {
        self.quest_change(quest, |log| log.set_stage(quest, stage))
    }
}

impl StandingView for Game {
    fn reputation(&self, faction: &FactionId) -> ReputationLevel {
        self.standing.reputation(faction)
    }

    fn renown_tier(&self, faction: &FactionId, renown: Renown) -> RenownTier {
        self.standing.renown_tier(faction, renown)
    }

    fn renown_points(&self, faction: &FactionId, renown: Renown) -> f64 {
        self.standing.renown_points(faction, renown)
    }

    fn alignment(&self) -> Alignment {
        self.standing.alignment()
    }
}

// Companions keep the trait defaults: nobody has joined.
impl CompanionView for Game {}

impl CharacterView for Game {
    fn skill(&self, skill: Skill) -> SkillLevel {
        self.character.skill(skill)
    }

    fn special(&self, stat: Special) -> SpecialLevel {
        self.character.special(stat)
    }

    fn has_perk(&self, perk: &PerkId) -> bool {
        self.character.has_perk(perk)
    }

    fn health(&self) -> HealthPercent {
        HealthPercent::of(self.hp(), self.character.max_hp())
    }
}

impl InventoryView for Game {
    fn item_count(&self, item: &ItemId) -> u32 {
        self.inventory.item_count(item)
    }

    fn caps(&self) -> u32 {
        self.inventory.caps()
    }

    fn is_equipped(&self, item: &ItemId) -> bool {
        self.inventory.is_equipped(item)
    }

    fn disguises_as(&self, item: &ItemId, faction: &FactionId) -> bool {
        self.inventory.disguises_as(item, faction)
    }
}

impl WorldView for Game {
    fn flag(&self, flag: &FlagId) -> bool {
        self.world.flag(flag)
    }

    fn npc_is(&self, npc: &NpcId, state: NpcState) -> bool {
        self.world.npc_is(npc, state)
    }
}

impl StandingSink for Game {
    fn add_reputation(
        &mut self,
        faction: &FactionId,
        fame: f64,
        infamy: f64,
    ) -> Result<(), EffectError> {
        self.standing.add_reputation(faction, fame, infamy)
    }

    fn set_reputation(
        &mut self,
        faction: &FactionId,
        level: ReputationLevel,
    ) -> Result<(), EffectError> {
        self.standing.set_reputation(faction, level)
    }

    fn add_karma(&mut self, amount: f64) -> Result<(), EffectError> {
        self.standing.add_karma(amount)
    }
}

impl CharacterSink for Game {
    fn add_xp(&mut self, amount: f64) -> Result<(), EffectError> {
        self.character.add_xp(amount)
    }

    fn restore_health(&mut self) -> Result<(), EffectError> {
        self.heal(self.character.max_hp());
        Ok(())
    }
}

impl InventorySink for Game {
    fn add_caps(&mut self, amount: f64) -> Result<(), EffectError> {
        self.inventory.add_caps(amount)
    }

    fn give_item(&mut self, item: &ItemId, count: NonZeroU32) -> Result<(), EffectError> {
        self.inventory.give_item(item, count)
    }

    fn take_item(&mut self, item: &ItemId, count: NonZeroU32) -> Result<(), EffectError> {
        self.inventory.take_item(item, count)
    }
}

impl CompanionSink for Game {
    fn companion_join(&mut self, _: &CompanionId) -> Result<(), EffectError> {
        Err(not_built("companion_join", "companions"))
    }

    fn companion_leave(&mut self, _: &CompanionId) -> Result<(), EffectError> {
        Err(not_built("companion_leave", "companions"))
    }
}

impl WorldSink for Game {
    fn set_flag(&mut self, flag: &FlagId, value: bool) -> Result<(), EffectError> {
        self.world.set_flag(flag, value)
    }

    fn kill_npc(&mut self, npc: &NpcId) -> Result<(), EffectError> {
        self.world.kill_npc(npc)
    }

    fn set_hostile(&mut self, npc: &NpcId) -> Result<(), EffectError> {
        self.world.set_hostile(npc)
    }

    fn set_friendly(&mut self, npc: &NpcId) -> Result<(), EffectError> {
        self.world.set_friendly(npc)
    }

    fn remove_npc(&mut self, npc: &NpcId) -> Result<(), EffectError> {
        self.world.remove_npc(npc)
    }

    fn unlock_ending(&mut self, ending: &EndingId) -> Result<(), EffectError> {
        self.world.unlock_ending(ending)
    }

    fn lock_ending(&mut self, ending: &EndingId) -> Result<(), EffectError> {
        self.world.lock_ending(ending)
    }
}
