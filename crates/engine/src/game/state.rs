//! The one state that `evaluate` and `apply_all` run against.

use std::num::NonZeroU32;
use std::sync::Arc;

use crate::quests::{QuestBook, QuestLog};
use crate::shared::{
    CharacterSink, CharacterView, CompanionId, CompanionSink, CompanionView, EffectError, EndingId,
    FactionId, FlagId, InventorySink, InventoryView, ItemId, NpcId, NpcState, QuestId, QuestSink,
    QuestStage, QuestStatus, QuestView, ReputationLevel, StandingSink, StandingView, WorldSink,
    WorldView,
};
use crate::world::{WorldBook, WorldState};

/// Everything the player has done so far.
///
/// Cloning is cheap (the quest book is shared), so a caller can try a batch of
/// effects on a copy and keep it only if it works.
#[derive(Debug, Clone)]
pub struct Game {
    quests: QuestLog,
    world: WorldState,
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
            quests: QuestLog::new(book),
            world: WorldState::new(world),
        }
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

    /// Records that the player has met the NPC.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the NPC is unknown, dead or removed.
    pub fn meet(&mut self, npc: &NpcId) -> Result<(), EffectError> {
        self.world.meet(npc)
    }
}

/// The refusal for an effect whose feature has no state yet. Removing one of these
/// is the signal that the feature landed and `Game` gained a field for it.
fn not_built(effect: &'static str, feature: &'static str) -> EffectError {
    EffectError::Refused {
        effect,
        reason: format!("the {feature} feature is not built yet"),
    }
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
        self.quests.start_quest(quest)
    }

    fn complete_quest(&mut self, quest: &QuestId) -> Result<(), EffectError> {
        self.quests.complete_quest(quest)
    }

    fn fail_quest(&mut self, quest: &QuestId) -> Result<(), EffectError> {
        self.quests.fail_quest(quest)
    }

    fn block_quest(&mut self, quest: &QuestId) -> Result<(), EffectError> {
        self.quests.block_quest(quest)
    }

    fn set_stage(&mut self, quest: &QuestId, stage: QuestStage) -> Result<(), EffectError> {
        self.quests.set_stage(quest, stage)
    }
}

// The four views below keep the trait defaults: nothing owned, nobody met.
impl StandingView for Game {}
impl CharacterView for Game {}
impl InventoryView for Game {}
impl CompanionView for Game {}

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
    fn add_xp(&mut self, _: f64) -> Result<(), EffectError> {
        Err(not_built("xp", "character"))
    }
}

impl InventorySink for Game {
    fn add_caps(&mut self, _: f64) -> Result<(), EffectError> {
        Err(not_built("caps", "inventory"))
    }

    fn give_item(&mut self, _: &ItemId, _: NonZeroU32) -> Result<(), EffectError> {
        Err(not_built("give_item", "inventory"))
    }

    fn take_item(&mut self, _: &ItemId, _: NonZeroU32) -> Result<(), EffectError> {
        Err(not_built("take_item", "inventory"))
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
