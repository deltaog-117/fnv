//! Locations, the travel graph and the places NPCs and creatures live in.
//!
//! So far this holds the story flags and the fate of each NPC. [`WorldBook`] is the
//! read-only roster of NPCs that exist; [`WorldState`] is what has happened to
//! them and which flags are set. It implements the shared `WorldView` and
//! `WorldSink` traits, so conditions and effects run against it unchanged.
//!
//! The state owns these laws and nothing else:
//!
//! - an NPC is alive until killed, and dead and removed are terminal;
//! - only a living NPC can turn hostile or friendly, so "dead and hostile" cannot
//!   be stored;
//! - an NPC missing from the roster is an error, never a silent no-op.

mod book;
mod state;

pub use book::WorldBook;
pub use state::WorldState;
