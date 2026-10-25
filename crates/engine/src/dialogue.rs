//! The dialogue runner: topics, skill-check gates and their effects.
//!
//! [`DialogueBook`] is the read-only set of speakers' topics, checked once when it
//! is built. [`Conversation`] is one talk with one speaker; it holds only where the
//! talk stands, and runs conditions and effects against whatever state the caller
//! passes in, so the player's state stays owned by `game`.
//!
//! The runner owns these laws and nothing else:
//!
//! - a conversation opens at the speaker's `start` topics, and a topic with no
//!   `next` sends it back there;
//! - only a topic that is offered (all its `conditions` hold) can be chosen, and a
//!   topic whose `check` fails is listed but cannot be chosen;
//! - a topic's effects apply as a unit: if one fails, the state is left as it was
//!   and the conversation does not move;
//! - a topic with `ends` closes the conversation, and nothing can be chosen after.

mod book;
mod conversation;

pub use book::{DialogueBook, DialogueBookError};
pub use conversation::{Choice, Conversation, DialogueError, Reply};
