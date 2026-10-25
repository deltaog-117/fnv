//! The one state that `evaluate` and `apply_all` run against.

use std::num::NonZeroU32;
use std::sync::Arc;

use crate::quests::{QuestBook, QuestLog};
use crate::shared::{
    CharacterSink, CharacterView, CompanionId, CompanionSink, CompanionView, EffectError, EndingId,
    FactionId, FlagId, InventorySink, InventoryView, ItemId, NpcId, QuestId, QuestSink, QuestStage,
    QuestStatus, QuestView, ReputationLevel, StandingSink, StandingView, WorldSink, WorldView,
};

/// Everything the player has done so far.
///
/// Cloning is cheap (the quest book is shared), so a caller can try a batch of
/// effects on a copy and keep it only if it works.
#[derive(Debug, Clone)]
pub struct Game {
    quests: QuestLog,
}

impl Game {
    /// A new game: no quest started.
    #[must_use]
    pub fn new(book: Arc<QuestBook>) -> Self {
        Self {
            quests: QuestLog::new(book),
        }
    }

    /// The quest log, for the journal.
    #[must_use]
    pub fn quests(&self) -> &QuestLog {
        &self.quests
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

// The five views below keep the trait defaults: nothing owned, nobody met.
impl StandingView for Game {}
impl CharacterView for Game {}
impl InventoryView for Game {}
impl CompanionView for Game {}
impl WorldView for Game {}

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
    fn set_flag(&mut self, _: &FlagId, _: bool) -> Result<(), EffectError> {
        Err(not_built("set_flag", "world"))
    }

    fn kill_npc(&mut self, _: &NpcId) -> Result<(), EffectError> {
        Err(not_built("kill_npc", "world"))
    }

    fn set_hostile(&mut self, _: &NpcId) -> Result<(), EffectError> {
        Err(not_built("set_hostile", "world"))
    }

    fn set_friendly(&mut self, _: &NpcId) -> Result<(), EffectError> {
        Err(not_built("set_friendly", "world"))
    }

    fn remove_npc(&mut self, _: &NpcId) -> Result<(), EffectError> {
        Err(not_built("remove_npc", "world"))
    }

    fn unlock_ending(&mut self, _: &EndingId) -> Result<(), EffectError> {
        Err(not_built("unlock_ending", "world"))
    }

    fn lock_ending(&mut self, _: &EndingId) -> Result<(), EffectError> {
        Err(not_built("lock_ending", "world"))
    }
}
