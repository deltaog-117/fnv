//! Timed effects: the boosts in force now, and when each one ends.
//!
//! [`ActiveEffects`] holds the boosts a chem, a drink or a meal has started, each
//! with the minute it ends. It reads no clock of its own: the caller says what time
//! it is, so [`Game`](crate::game::Game) keeps the one clock the whole game uses.
//!
//! This feature owns these laws and nothing else:
//!
//! - once [`ActiveEffects::expire`] has run for a time, nothing in force ends at or
//!   before it;
//! - a new dose of an item replaces the last dose of the same item instead of
//!   stacking on it;
//! - the tally is exactly the sum of what is in force.
//!
//! Applying the tally to the character, keeping the player's hit points sensible
//! when a boost to maximum hit points ends, and spending the item belong to
//! [`Game`](crate::game::Game).

mod active;

pub use active::{Active, ActiveEffects};
