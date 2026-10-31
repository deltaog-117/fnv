//! Beds and sleep: where the player may rest, and how much a night restores.
//!
//! [`BedBook`] is the read-only list of beds, each at a place. [`restored`] turns
//! the length of a sleep into hit points.
//!
//! This feature owns these laws and nothing else:
//!
//! - a sleep of the full rest length restores every hit point, and a shorter one
//!   restores a proportional share, rounded up so any sleep helps a wounded player;
//! - a longer sleep never restores more than the maximum;
//! - a bed id appears once.
//!
//! Hit points, the clock and the choice of bed belong to
//! [`Game`](crate::game::Game), which runs a sleep as one unit: the time passes and
//! the wounds close together or not at all. Waiting without a bed restores
//! nothing. Hardcore mode, campfires and owned beds wait for their own features.

mod book;
mod error;
mod recovery;

pub use book::{BedBook, BedInfo};
pub use error::RestError;
pub use recovery::{FULL_REST_HOURS, restored};
