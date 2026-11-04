//! Encounter records: a fight the player can start at a place.

use std::num::NonZeroU32;

use serde::Deserialize;

use crate::shared::{Condition, Effect, EncounterId, FoeKind, ItemId, LocationId, NonEmpty, NpcId};

fn default_kind() -> FoeKind {
    FoeKind::Human
}

/// One enemy of an encounter. The NPC record gives its attributes and level; the
/// wiki has no loadouts, so the weapon and the armor are authored here.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Foe {
    /// The NPC this enemy is.
    pub npc: NpcId,
    /// The weapon in hand.
    pub weapon: ItemId,
    /// The armor worn, if any.
    pub armor: Option<ItemId>,
    /// What sort of creature this is; a person when absent.
    #[serde(default = "default_kind")]
    pub kind: FoeKind,
    /// Hit points, when the NPC record gives none and the formula for the player
    /// would make a footsoldier tougher than the player.
    pub hp: Option<NonZeroU32>,
    /// Every condition must hold for this enemy to be in the fight; none means it
    /// always is. The game has no allies yet, so a defender who joined the town's
    /// side is written as an enemy that is absent.
    #[serde(default)]
    pub when: Vec<Condition>,
}

/// A fight the player can start, and what winning it does.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Encounter {
    /// The encounter's unique id.
    pub id: EncounterId,
    /// What the player sees when choosing it.
    pub name: String,
    /// The place where it can start.
    pub place: LocationId,
    /// Every condition must hold for the fight to be offered.
    #[serde(default)]
    pub requires: Vec<Condition>,
    /// The enemies, in the order they act after the player.
    pub foes: NonEmpty<Foe>,
    /// Applied, in order, when the player wins.
    #[serde(default)]
    pub on_win: Vec<Effect>,
}
