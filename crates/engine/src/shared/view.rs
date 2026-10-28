//! Read-only views of game state, the questions a [`Condition`] can ask.
//!
//! Each feature owns its state and answers only for its own domain, so the
//! evaluator never imports a feature. Every method has a default answer ("not
//! started", "no", "zero"), which lets a test implement one trait and leave the
//! rest empty. A default is an answer for something the state does not know about;
//! a feature that owns the domain overrides it.
//!
//! [`Condition`]: super::Condition

use super::bounds::{HealthPercent, SkillLevel, SpecialLevel};
use super::ids::{CompanionId, FactionId, FlagId, ItemId, NpcId, PerkId, QuestId};
use super::vocabulary::{Alignment, NpcState, QuestStage, ReputationLevel, Skill, Special};

/// Where a quest stands, apart from its stage number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuestStatus {
    /// The quest has not begun.
    NotStarted,
    /// The quest is in progress.
    Active,
    /// The quest finished successfully.
    Completed,
    /// The quest failed or was locked out.
    Failed,
}

/// Quest progress.
pub trait QuestView {
    /// Where the quest stands; unknown quests have not started.
    fn quest_status(&self, _quest: &QuestId) -> QuestStatus {
        QuestStatus::NotStarted
    }

    /// The stage the quest last reached, if it reached any. A finished quest keeps
    /// its last stage, so stage conditions still answer after completion.
    fn quest_stage(&self, _quest: &QuestId) -> Option<QuestStage> {
        None
    }
}

/// Reputation and karma.
pub trait StandingView {
    /// The player's standing with a faction; unknown factions are neutral.
    fn reputation(&self, _faction: &FactionId) -> ReputationLevel {
        ReputationLevel::Neutral
    }

    /// The player's overall moral alignment.
    fn alignment(&self) -> Alignment {
        Alignment::Neutral
    }
}

/// The player character's skills, attributes and perks.
pub trait CharacterView {
    /// The skill level; the lowest level by default.
    fn skill(&self, _skill: Skill) -> SkillLevel {
        SkillLevel::default()
    }

    /// The attribute value; the lowest value by default.
    fn special(&self, _stat: Special) -> SpecialLevel {
        SpecialLevel::default()
    }

    /// Whether the player has the perk.
    fn has_perk(&self, _perk: &PerkId) -> bool {
        false
    }

    /// The share of their hit points the player has left; full by default, since a
    /// state that tracks no wounds has nobody hurt.
    fn health(&self) -> HealthPercent {
        HealthPercent::FULL
    }
}

/// What the player carries and wears.
pub trait InventoryView {
    /// How many of an item the player carries.
    fn item_count(&self, _item: &ItemId) -> u32 {
        0
    }

    /// How many bottle caps the player carries.
    fn caps(&self) -> u32 {
        0
    }

    /// Whether the item is equipped.
    fn is_equipped(&self, _item: &ItemId) -> bool {
        false
    }

    /// Whether the item counts as a disguise for the faction.
    fn disguises_as(&self, _item: &ItemId, _faction: &FactionId) -> bool {
        false
    }
}

/// Who travels with the player.
pub trait CompanionView {
    /// Whether the companion is travelling with the player.
    fn companion_active(&self, _companion: &CompanionId) -> bool {
        false
    }
}

/// Story flags and the non-player characters.
pub trait WorldView {
    /// Whether the story flag is set.
    fn flag(&self, _flag: &FlagId) -> bool {
        false
    }

    /// Whether the NPC is in the state. An NPC the state knows nothing about is in
    /// no state, `Alive` included.
    fn npc_is(&self, _npc: &NpcId, _state: NpcState) -> bool {
        false
    }
}

/// Everything a condition can ask about. Implemented for any type that
/// implements the six domain views, so a state never implements it by hand.
pub trait StateView:
    QuestView + StandingView + CharacterView + InventoryView + CompanionView + WorldView
{
}

impl<T> StateView for T where
    T: QuestView
        + StandingView
        + CharacterView
        + InventoryView
        + CompanionView
        + WorldView
        + ?Sized
{
}

/// A world that answers every question with the default: nothing started, nothing
/// owned, nobody met. Useful as a baseline in tests.
#[derive(Debug, Clone, Copy, Default)]
pub struct EmptyWorld;

impl QuestView for EmptyWorld {}
impl StandingView for EmptyWorld {}
impl CharacterView for EmptyWorld {}
impl InventoryView for EmptyWorld {}
impl CompanionView for EmptyWorld {}
impl WorldView for EmptyWorld {}
