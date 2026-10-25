//! The `Condition` vocabulary: predicates over game state.
//!
//! Quest rules, dialogue options, perk requirements and NPC availability all use
//! these. Evaluating one is the job of the evaluator, not of this module; here
//! they are only data that can be built, parsed and written back without ever
//! being invalid.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use super::bounds::{SkillLevel, SpecialLevel};
use super::error::ModelError;
use super::ids::{CompanionId, FactionId, FlagId, ItemId, NpcId, PerkId, QuestId};
use super::non_empty::NonEmpty;
use super::vocabulary::{
    Alignment, Comparison, NpcState, QuestStage, QuestState, ReputationLevel, Skill, Special,
};

fn default_count() -> NonZeroU32 {
    NonZeroU32::MIN
}

fn default_true() -> bool {
    true
}

/// A predicate over game state.
///
/// Serialized as an object with a `type` field, matching
/// `$defs/condition` in `schemas/common.schema.json`. Unknown fields are
/// rejected, as the schema rejects them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Condition {
    /// Every nested condition holds.
    All {
        /// The conditions that must all hold.
        of: NonEmpty<Condition>,
    },
    /// At least one nested condition holds.
    Any {
        /// The conditions of which one must hold.
        of: NonEmpty<Condition>,
    },
    /// The nested condition does not hold.
    Not {
        /// The condition to negate.
        of: Box<Condition>,
    },
    /// A quest is in a given state.
    Quest(QuestCondition),
    /// The player's standing with a faction compares to a level.
    Reputation {
        /// The faction or town.
        faction: FactionId,
        /// The standing to compare against.
        level: ReputationLevel,
        /// How to compare; defaults to "at least".
        #[serde(default)]
        op: Comparison,
    },
    /// The player's skill is at least a value.
    Skill {
        /// The skill.
        skill: Skill,
        /// The minimum level.
        min: SkillLevel,
    },
    /// The player's attribute is at least a value.
    Special {
        /// The attribute.
        stat: Special,
        /// The minimum value.
        min: SpecialLevel,
    },
    /// The player has a perk.
    Perk {
        /// The perk.
        perk: PerkId,
    },
    /// The player's karma has an alignment.
    Karma {
        /// The alignment.
        alignment: Alignment,
    },
    /// A companion is (or is not) travelling with the player.
    Companion {
        /// The companion.
        companion: CompanionId,
        /// Whether the companion must be active.
        active: bool,
    },
    /// The player carries at least some of an item.
    HasItem {
        /// The item.
        item: ItemId,
        /// How many; defaults to one.
        #[serde(default = "default_count")]
        count: NonZeroU32,
    },
    /// The player wears an item, or any gear of a faction.
    Wearing {
        /// The item that must be equipped.
        equipped: ItemId,
        /// A faction whose disguise the item must count as.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        faction: Option<FactionId>,
    },
    /// A story flag is set (or unset).
    Flag {
        /// The flag.
        flag: FlagId,
        /// The required value; defaults to set.
        #[serde(default = "default_true")]
        value: bool,
    },
    /// A non-player character is in a state.
    Npc {
        /// The NPC.
        npc: NpcId,
        /// The state asked about.
        state: NpcState,
    },
}

impl Condition {
    /// Every `type` value a condition can have, in schema order.
    pub const ALL_TAGS: [&'static str; 14] = [
        "all",
        "any",
        "not",
        "quest",
        "reputation",
        "skill",
        "special",
        "perk",
        "karma",
        "companion",
        "has_item",
        "wearing",
        "flag",
        "npc",
    ];

    /// The `type` value of this condition. The match is exhaustive, so adding a
    /// variant without naming it here does not compile.
    #[must_use]
    pub fn tag(&self) -> &'static str {
        match self {
            Self::All { .. } => "all",
            Self::Any { .. } => "any",
            Self::Not { .. } => "not",
            Self::Quest(_) => "quest",
            Self::Reputation { .. } => "reputation",
            Self::Skill { .. } => "skill",
            Self::Special { .. } => "special",
            Self::Perk { .. } => "perk",
            Self::Karma { .. } => "karma",
            Self::Companion { .. } => "companion",
            Self::HasItem { .. } => "has_item",
            Self::Wearing { .. } => "wearing",
            Self::Flag { .. } => "flag",
            Self::Npc { .. } => "npc",
        }
    }
}

/// A quest in a given state.
///
/// On the wire this is `{"quest": id, "state": name, "stage": n}` where `stage`
/// is present exactly when the state is `stage_at_least` or `stage_is`. The
/// stage-carrying variants of [`QuestState`] make any other combination
/// impossible to build.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "QuestConditionWire", into = "QuestConditionWire")]
pub struct QuestCondition {
    /// The quest.
    pub quest: QuestId,
    /// The state it must be in.
    pub state: QuestState,
}

impl QuestCondition {
    /// Pairs a quest with the state it must be in.
    #[must_use]
    pub fn new(quest: QuestId, state: QuestState) -> Self {
        Self { quest, state }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum QuestStateTag {
    NotStarted,
    Active,
    Completed,
    Failed,
    StageAtLeast,
    StageIs,
}

/// The shape on the wire: a state name plus an optional stage number.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct QuestConditionWire {
    quest: QuestId,
    state: QuestStateTag,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stage: Option<QuestStage>,
}

impl TryFrom<QuestConditionWire> for QuestCondition {
    type Error = ModelError;

    fn try_from(wire: QuestConditionWire) -> Result<Self, Self::Error> {
        let state = match (wire.state, wire.stage) {
            (QuestStateTag::NotStarted, None) => QuestState::NotStarted,
            (QuestStateTag::Active, None) => QuestState::Active,
            (QuestStateTag::Completed, None) => QuestState::Completed,
            (QuestStateTag::Failed, None) => QuestState::Failed,
            (QuestStateTag::StageAtLeast, Some(stage)) => QuestState::StageAtLeast(stage),
            (QuestStateTag::StageIs, Some(stage)) => QuestState::StageIs(stage),
            (QuestStateTag::StageAtLeast, None) => {
                return Err(ModelError::QuestStage {
                    state: "stage_at_least",
                    problem: "needs a stage number",
                });
            }
            (QuestStateTag::StageIs, None) => {
                return Err(ModelError::QuestStage {
                    state: "stage_is",
                    problem: "needs a stage number",
                });
            }
            (QuestStateTag::NotStarted, Some(_)) => return Err(unexpected_stage("not_started")),
            (QuestStateTag::Active, Some(_)) => return Err(unexpected_stage("active")),
            (QuestStateTag::Completed, Some(_)) => return Err(unexpected_stage("completed")),
            (QuestStateTag::Failed, Some(_)) => return Err(unexpected_stage("failed")),
        };
        Ok(Self {
            quest: wire.quest,
            state,
        })
    }
}

fn unexpected_stage(state: &'static str) -> ModelError {
    ModelError::QuestStage {
        state,
        problem: "must not have a stage number",
    }
}

impl From<QuestCondition> for QuestConditionWire {
    fn from(condition: QuestCondition) -> Self {
        let (state, stage) = match condition.state {
            QuestState::NotStarted => (QuestStateTag::NotStarted, None),
            QuestState::Active => (QuestStateTag::Active, None),
            QuestState::Completed => (QuestStateTag::Completed, None),
            QuestState::Failed => (QuestStateTag::Failed, None),
            QuestState::StageAtLeast(stage) => (QuestStateTag::StageAtLeast, Some(stage)),
            QuestState::StageIs(stage) => (QuestStateTag::StageIs, Some(stage)),
        };
        Self {
            quest: condition.quest,
            state,
            stage,
        }
    }
}
