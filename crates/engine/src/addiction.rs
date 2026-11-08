//! Addiction: which chems the player is hooked on, and what withdrawal costs.
//!
//! [`AddictionBook`] reads once, at load, which consumables carry an addiction risk
//! and how likely it is. [`Addictions`] holds what the player is addicted to, each
//! with the minute of the last dose. Withdrawal is a [`BoostTally`], read by the
//! character like any other change, so this feature never touches the character
//! itself. It takes a roll as a plain number, so it needs no source of chance of
//! its own: [`Game`](crate::game::Game) rolls the dice.
//!
//! This feature owns these laws and nothing else:
//!
//! - a roll at or below the chance means addiction, so 0% never and 100% always;
//! - a dose of a chem the player is already addicted to never starts a second
//!   addiction, and only moves the time of the last dose forward;
//! - once [`Addictions::expire`] has run for a time, nothing held has gone without
//!   a dose for [`WITHDRAWAL_MINUTES`] or more;
//! - withdrawal only grows with the number of addictions.
//!
//! Rolling the dice, spending the item and curing at a doctor belong to
//! [`Game`](crate::game::Game).

mod book;
mod state;

pub use book::{AddictionBook, AddictionError, Risk};
pub use state::{AddictionKind, Addictions, KindError, WITHDRAWAL_MINUTES};
