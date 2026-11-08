//! Poison: venom in the blood, and the hit points it takes as time passes.
//!
//! [`Poison`] holds one bounded number of points still to be paid in hit points.
//! Taking some in is cut by the player's poison resistance ([`taken_in`]); taking
//! some out is not. Time moves the points out of the blood and into the wounds:
//! [`Poison::elapse`] says how many came out, and
//! [`Game`](crate::game::Game) turns them into wounds.
//!
//! This feature owns these laws and nothing else:
//!
//! - the points never go above [`Poison::MAX`] or below none;
//! - resistance never makes more poison come in, and full resistance lets none in;
//! - time only ever lowers the points, and what it lowers them by is exactly what
//!   it reports, so no hit point is lost that was not first poison;
//! - an antidote ends the poison whatever its size.
//!
//! What resistance a player has, spending the antidote and keeping a poisoned
//! player alive belong to [`Game`](crate::game::Game).

mod state;

pub use state::{DRAIN_PER_MINUTE, Poison, taken_in};
