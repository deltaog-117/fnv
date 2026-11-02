//! Why a save could not be written, read or loaded.

use thiserror::Error;

/// A save was refused. Nothing in the game changes when one of these is returned.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SaveError {
    /// The text is not a save: bad JSON, a missing or unknown field, or an id that is
    /// not valid.
    #[error("not a valid save: {0}")]
    Format(String),
    /// The save was written by a newer game.
    #[error("the save is version {found}, and this game reads up to version {supported}")]
    TooNew {
        /// The version in the file.
        found: u64,
        /// The newest version this game reads.
        supported: u32,
    },
    /// No such version ever existed.
    #[error("the save claims version {0}, which never existed")]
    UnknownVersion(u64),
    /// The file could not be read or written.
    #[error("{0}")]
    Io(String),
    /// The save does not fit the game it is loaded into.
    #[error("{part}: {reason}")]
    Invalid {
        /// The part of the save at fault, such as `quests` or `pack`.
        part: &'static str,
        /// What is wrong with it.
        reason: String,
    },
}

impl SaveError {
    /// A part of the save that cannot be loaded, with the reason in words.
    pub fn invalid(part: &'static str, reason: impl std::fmt::Display) -> Self {
        Self::Invalid {
            part,
            reason: reason.to_string(),
        }
    }
}
