//! Types and contracts every feature builds on: ids, the `Condition` and `Effect`
//! vocabulary, the read-only view traits and write-side sink traits with the
//! evaluator that connects them, the data loader and errors.
//!
//! Dependencies point inward: nothing here may import from a feature.
//!
//! The vocabulary mirrors `schemas/common.schema.json`. The schema stays the
//! language-neutral contract; contract tests in `tests/shared` fail when the two
//! drift apart.

mod boost;
mod bounds;
mod condition;
mod data;
mod effect;
mod error;
mod eval;
mod ids;
mod load;
mod modifier;
mod non_empty;
mod redacted;
mod sink;
mod view;
mod vocabulary;

pub use boost::{Boost, BoostStat, BoostTally, DerivedStat};
pub use bounds::{DiscountPercent, HealthPercent, Rads, RenownTier, SkillLevel, SpecialLevel};
pub use condition::{Condition, QuestCondition};
pub use data::{
    Addiction, Ammo, Armor, ArmorClass, ArmorSlot, Bed, BoostUse, Consumable, CraftReaction,
    Dialogue, Discount, Encounter, Faction, FactionKind, Find, FindLine, Foe, InfoValue,
    Ingredient, ItemKind, ItemUse, Location, LocationSize, MapKind, Misc, Neighbor, Npc,
    NpcSpecial, Perk, PerkRequirements, Position, Quest, QuestKind, Recipe, RecipeMode,
    RecipeSkill, RepairService, ReputationModel, ReputationNote, Response, Rule, RuleKind,
    RuleTargetType, Stage, StatEffect, Station, StationKind, StockLine, Topic, TopicSection, Trait,
    Vendor, WalkthroughNode, Weapon, WeaponClass,
};
pub use effect::Effect;
pub use error::ModelError;
pub use eval::{apply, apply_all, evaluate};
pub use ids::{
    AmmoFamilyId, BedId, CompanionId, DialogueId, EncounterId, EndingId, FactionId, FindId, FlagId,
    ItemId, LocationId, NpcId, PerkId, QuestId, RecipeId, StationId, TopicKey, TraitId, VendorId,
};
pub use load::{Catalog, GameData, LoadError, Record, Scope};
pub use modifier::Modifier;
pub use non_empty::NonEmpty;
pub use redacted::Redacted;
pub use sink::{
    BatchError, CharacterSink, CompanionSink, EffectError, InventorySink, QuestSink, StandingSink,
    State, StateSink, WorldSink, whole_amount,
};
pub use view::{
    CharacterView, CompanionView, EmptyWorld, InventoryView, QuestStatus, QuestView, StandingView,
    StateView, WorldView,
};
pub use vocabulary::{
    Ailment, Alignment, Comparison, Dlc, FoeKind, NpcState, QuestStage, QuestState, Renown,
    ReputationLevel, Skill, Special,
};
