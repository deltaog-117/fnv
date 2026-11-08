//! Radiation: how much the player has absorbed, and the sickness it brings.
//!
//! [`Radiation`] holds one bounded number of rads. Taking some in is cut by the
//! player's radiation resistance ([`absorbed`]); taking some out is not. The
//! sickness is a [`BoostTally`], read by the character like any other change, so
//! this feature never touches the character itself.
//!
//! This feature owns these laws and nothing else:
//!
//! - the rads never go above [`Rads::MAX`] or below none;
//! - resistance never makes more radiation come in, and full resistance lets none in;
//! - a sicker player is never better off: the sickness only grows with the rads.
//!
//! What resistance a player has, spending the item and curing at a doctor belong to
//! [`Game`](crate::game::Game).

mod state;

pub use state::{RADS_PER_TIER, Radiation, absorbed};
