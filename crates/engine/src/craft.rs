//! Crafting: what can be made, from what, at which station.
//!
//! [`RecipeBook`] turns the imported recipes into [`Formula`]s the game can make,
//! and says why each of the others cannot be made yet. [`StationBook`] lists the
//! stations, each of a kind and at a place.
//!
//! This feature owns these laws and nothing else:
//!
//! - a formula always has a station kind, an output and every ingredient resolved
//!   to an item, with at least one of each ingredient;
//! - a recipe is either a formula or has a stated reason it is not one, never both
//!   and never neither;
//! - a recipe id appears once, and a station id appears once.
//!
//! The pack, the skill check and the story belong to
//! [`Game`](crate::game::Game), which runs each craft as one unit: the
//! ingredients leave, the product arrives and the recipe's reactions run together
//! or not at all. The item the recipe names must exist; `Game` knows the item book,
//! so the caller of [`RecipeBook::from_data`] passes the test for it.

mod book;
mod error;
mod stations;

pub use book::{Formula, RecipeBook, Unavailable};
pub use error::CraftError;
pub use stations::{StationBook, StationInfo};
