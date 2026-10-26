//! The one state that `evaluate` and `apply_all` run against.

use std::collections::HashSet;
use std::num::NonZeroU32;
use std::sync::Arc;

use super::battle::{Arsenal, Battle, BattleError, Outcome};
use crate::character::{Character, PerkBook, ProgressError};
use crate::combat::{Side, Status};
use crate::inventory::{Inventory, InventoryError, ItemBook, Slot};
use crate::quests::{QuestBook, QuestLog};
use crate::shared::{
    CharacterSink, CharacterView, CompanionId, CompanionSink, CompanionView, Effect, EffectError,
    Encounter, EncounterId, EndingId, FactionId, FlagId, InventorySink, InventoryView, ItemId,
    LocationId, NpcId, NpcState, PerkId, QuestId, QuestSink, QuestStage, QuestStatus, QuestView,
    ReputationLevel, Skill, SkillLevel, Special, SpecialLevel, StandingSink, StandingView,
    WorldSink, WorldView, apply, apply_all, evaluate,
};
use crate::world::{WorldBook, WorldState};

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
    character: Character,
    inventory: Inventory,
    arsenal: Arc<Arsenal>,
    perks: Arc<PerkBook>,
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
            character: Character::default(),
            inventory: Inventory::new(Arc::default()),
            arsenal: Arc::default(),
            perks: Arc::default(),
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

    /// The same game for a different character, for example one with another
    /// S.P.E.C.I.A.L. spread. Experience already earned goes with the old one.
    #[must_use]
    pub fn with_character(mut self, character: Character) -> Self {
        self.character = character;
        self
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
        let player = self.arsenal.player(
            &self.character,
            self.inventory.equipped(Slot::Weapon),
            &worn,
            &self.inventory,
        )?;
        let foes = self
            .arsenal
            .foes(encounter)
            .ok_or_else(|| BattleError::UnknownEncounter(encounter.clone()))?;
        Battle::start(encounter.clone(), player, hp, foes)
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
        let outcome = if status == Status::Won(Side::Player) {
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

    /// Equips a carried item.
    ///
    /// # Errors
    ///
    /// [`InventoryError`] when the item is not carried or cannot be equipped.
    pub fn equip(&mut self, item: &ItemId) -> Result<Slot, InventoryError> {
        self.inventory.equip(item)
    }

    /// Empties an equipment slot, returning what was in it.
    pub fn unequip(&mut self, slot: Slot) -> Option<ItemId> {
        self.inventory.unequip(slot)
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

// The two views below keep the trait defaults: no standing, nobody met.
impl StandingView for Game {}
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
    fn add_reputation(&mut self, _: &FactionId, _: f64, _: f64) -> Result<(), EffectError> {
        Err(not_built("reputation", "reputation"))
    }

    fn set_reputation(&mut self, _: &FactionId, _: ReputationLevel) -> Result<(), EffectError> {
        Err(not_built("set_reputation", "reputation"))
    }

    fn add_karma(&mut self, _: f64) -> Result<(), EffectError> {
        Err(not_built("karma", "reputation"))
    }
}

impl CharacterSink for Game {
    fn add_xp(&mut self, amount: f64) -> Result<(), EffectError> {
        self.character.add_xp(amount)
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
