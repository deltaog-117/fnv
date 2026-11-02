//! Travel: the places the player can walk between, and how long each walk takes.
//!
//! [`TravelBook`] is the read-only map: every location, and the links between
//! neighboring ones, built once from the location records. [`minutes_for`] turns a
//! distance on the map into time.
//!
//! This feature owns these laws and nothing else:
//!
//! - a link is two-way: if the player can walk from A to B they can walk back, and
//!   the walk takes the same time;
//! - a walk always takes time, never zero;
//! - a link only joins two places that exist, over a distance that is finite and
//!   greater than zero.
//!
//! Where the player stands, and the clock a walk advances, belong to
//! [`Game`](crate::game::Game), which runs a trip as one unit: the player moves and
//! the time passes together or not at all. The derived links are an approximation of
//! the game's roads, not the roads themselves, and the walking pace is a stand-in.

mod book;
mod error;
mod pace;

pub use book::{Route, TravelBook};
pub use error::TravelError;
pub use pace::{MINUTES_PER_PIXEL, minutes_for};
