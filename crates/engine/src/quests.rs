//! The quest log: stages, branching and the lock/fail/unlock consequence rules.
//!
//! [`QuestBook`] is the read-only definition of every quest (its stages and
//! consequence rules), checked once when it is built. [`QuestLog`] is the
//! player's progress through those quests; it implements the shared `QuestView`
//! and `QuestSink` traits, so conditions and effects run against it unchanged.
//!
//! The log owns the lifecycle laws and nothing else:
//!
//! - a quest moves `not started -> active -> completed | failed`;
//! - completed and failed are terminal, so no later effect reopens them;
//! - stages only move forward, and reaching a `finish` stage completes the quest.
//!
//! Firing the consequence rules needs the whole game state, so it lives in
//! `game`; the book only hands the rules out.

mod book;
mod log;

pub use book::{QuestBook, QuestBookError};
pub use log::QuestLog;
