//! What a conditional perk or trait reads that the sheet does not hold: how hurt
//! the character is and what time it is.

use crate::shared::{
    CharacterView, CompanionView, HealthPercent, InventoryView, MinuteOfDay, QuestView,
    StandingView, WorldView,
};

/// The two facts a conditional modifier may depend on. The game sets them each
/// time wounds or the clock change; the character only reads them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Situation {
    /// The share of their hit points the character has left.
    pub health: HealthPercent,
    /// The time of day.
    pub time: MinuteOfDay,
}

impl Default for Situation {
    /// Unhurt, at 08:00, where a new game starts.
    fn default() -> Self {
        Self {
            health: HealthPercent::FULL,
            time: MinuteOfDay::new(8, 0).unwrap_or(MinuteOfDay::MIDNIGHT),
        }
    }
}

// A situation answers only for health and the time; every other question gets the
// default, and the gates are refused at load if they ask one.
impl QuestView for Situation {}
impl StandingView for Situation {}
impl InventoryView for Situation {}
impl CompanionView for Situation {}

impl CharacterView for Situation {
    fn health(&self) -> HealthPercent {
        self.health
    }
}

impl WorldView for Situation {
    fn minute_of_day(&self) -> MinuteOfDay {
        self.time
    }
}
