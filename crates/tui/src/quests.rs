//! The quest journal: the persistent panel of active quests, and the journal screen
//! with every quest the player has started.

mod journal;
mod panel;

pub use journal::{Journal, Next};
pub use panel::render;
