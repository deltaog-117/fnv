//! Types and contracts every feature builds on: ids, the `Condition` and `Effect`
//! vocabulary, the read-only view traits, the data loader and errors.
//!
//! Dependencies point inward: nothing here may import from a feature.
//!
//! The vocabulary mirrors `schemas/common.schema.json`. The schema stays the
//! language-neutral contract; contract tests in `tests/shared` fail when the two
//! drift apart.

mod bounds;
mod condition;
mod effect;
mod error;
mod ids;
mod non_empty;
mod redacted;
mod vocabulary;

pub use bounds::{SkillLevel, SpecialLevel};
pub use condition::{Condition, QuestCondition};
pub use effect::Effect;
pub use error::ModelError;
pub use ids::{CompanionId, EndingId, FactionId, FlagId, ItemId, NpcId, PerkId, QuestId};
pub use non_empty::NonEmpty;
pub use redacted::Redacted;
pub use vocabulary::{
    Alignment, Comparison, Dlc, NpcState, QuestStage, QuestState, ReputationLevel, Skill, Special,
};
