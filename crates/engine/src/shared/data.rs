//! Plain records read from `data/engine/`.
//!
//! These types hold data and no game state. Every struct rejects unknown
//! fields, so a field added to a schema without a matching change here fails the
//! full-data load test instead of being dropped.

mod dialogue;
mod faction;
mod item;
mod location;
mod npc;
mod quest;

pub use dialogue::{Dialogue, Response, Topic, TopicSection};
pub use faction::{Faction, FactionKind, InfoValue, ReputationModel, ReputationNote};
pub use item::{
    Addiction, Ammo, Armor, ArmorClass, ArmorSlot, Consumable, ItemKind, StatEffect, Weapon,
    WeaponClass,
};
pub use location::{Location, LocationSize, MapKind, Neighbor, Position};
pub use npc::{Npc, NpcSpecial};
pub use quest::{Quest, QuestKind, Rule, RuleKind, RuleTargetType, Stage, WalkthroughNode};
