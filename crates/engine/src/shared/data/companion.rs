//! Companion records: a person who can join the player's party.

use serde::Deserialize;

use super::Foe;
use crate::shared::{CompanionId, Condition, PerkId};

/// A companion the player can recruit.
///
/// The wiki gives no loadouts for companions, so every `loadout` is a placeholder
/// until the game-data export exists. `loadout.when` must stay empty: a companion
/// is in a fight because they are in the party, not because a condition holds.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Companion {
    /// The companion's unique id.
    pub id: CompanionId,
    /// What the player sees.
    pub name: String,
    /// The NPC they are and what they fight with.
    pub loadout: Foe,
    /// A perk whose modifiers the player has while the companion is in the party.
    pub perk: Option<PerkId>,
    /// The companion will not join, and leaves, while any of these holds (their
    /// opinion of the player's reputation).
    #[serde(default)]
    pub refuses_when: Vec<Condition>,
}
