//! Errors raised while building the shared data model.

use thiserror::Error;

/// A value was rejected while constructing a model type.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ModelError {
    /// An id did not match `^[a-z0-9][a-z0-9_]*$`.
    #[error(
        "invalid {kind} id {value:?}: use lowercase letters, digits and underscores, \
         and start with a letter or digit"
    )]
    InvalidId {
        /// Which kind of id was being built (for example `"quest"`).
        kind: &'static str,
        /// The rejected text.
        value: String,
    },
    /// A bounded number was outside its allowed range.
    #[error("{what} must be between {min} and {max}, got {got}")]
    OutOfRange {
        /// The quantity being checked (for example `"skill level"`).
        what: &'static str,
        /// Smallest allowed value.
        min: u32,
        /// Largest allowed value.
        max: u32,
        /// The rejected value.
        got: u32,
    },
    /// A list that must hold at least one element was empty.
    #[error("{what} must not be empty")]
    Empty {
        /// The list being checked (for example `"all"`).
        what: &'static str,
    },
    /// A quest state and its stage number disagree.
    #[error("quest state {state:?} {problem}")]
    QuestStage {
        /// The wire name of the state.
        state: &'static str,
        /// What is wrong, for example `"needs a stage number"`.
        problem: &'static str,
    },
}
