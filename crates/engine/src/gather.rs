//! Gathering: items lying at a place that the player can pick up once.
//!
//! [`FindBook`] is the read-only list of finds, each at a place. [`Gathered`] is
//! which of them the player has already taken.
//!
//! This feature owns these laws and nothing else:
//!
//! - a find is taken at most once, and nothing grows back;
//! - a find lists at least one item, and an item appears once in it;
//! - a find id appears once.
//!
//! Handing the items to the pack belongs to [`Game`](crate::game::Game), which
//! takes a find as one unit: the items arrive and the find is marked taken
//! together or not at all.

mod book;
mod error;
mod taken;

pub use book::{FindBook, FindInfo};
pub use error::GatherError;
pub use taken::Gathered;
