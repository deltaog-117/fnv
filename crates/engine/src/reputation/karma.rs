//! Karma: the player's overall morals as one bounded number.

use crate::shared::Alignment;

/// Karma, from [`Karma::MIN`] (as evil as it gets) to [`Karma::MAX`].
///
/// The value is always finite and inside the range: a change that would leave it
/// stops at the edge.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Karma(f64);

impl Karma {
    /// The lowest karma.
    pub const MIN: f64 = -1000.0;
    /// The highest karma.
    pub const MAX: f64 = 1000.0;
    /// Karma at or above this is good, and at or below its negative is evil. A
    /// stand-in: the wiki's karma page is not imported yet, so this is the game's
    /// commonly quoted value, not a checked one.
    pub const THRESHOLD: f64 = 250.0;

    /// The karma after a change, stopping at the edge of the range. A change that
    /// is not a finite number leaves karma as it was.
    #[must_use]
    pub fn shifted(self, amount: f64) -> Self {
        if amount.is_finite() {
            Self((self.0 + amount).clamp(Self::MIN, Self::MAX))
        } else {
            self
        }
    }

    /// The karma as a plain number.
    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }

    /// What the karma amounts to.
    #[must_use]
    pub fn alignment(self) -> Alignment {
        if self.0 >= Self::THRESHOLD {
            Alignment::Good
        } else if self.0 <= -Self::THRESHOLD {
            Alignment::Evil
        } else {
            Alignment::Neutral
        }
    }
}
