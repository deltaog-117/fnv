//! The game clock: what day and hour it is, and how much time an action takes.
//!
//! [`GameClock`] counts whole minutes from the start of day 1. Time only moves
//! when the player does something that takes time, by [`Span`]: a length of time
//! that cannot be zero, so "wait for nothing" is not an action.
//!
//! This feature owns these laws and nothing else:
//!
//! - time never runs backwards: advancing only adds minutes;
//! - the hour is always 0 to 23, the minute 0 to 59, and the day starts at 1;
//! - advancing past the largest representable time is refused and leaves the
//!   clock as it was.
//!
//! What time does to the world (resting, restocking, timed effects) belongs to the
//! features that care. [`Game`](crate::game::Game) holds the clock and decides
//! when it may move. Conditions and effects cannot mention time yet.

mod error;
mod time;

pub use error::ClockError;
pub use time::{GameClock, Span};
