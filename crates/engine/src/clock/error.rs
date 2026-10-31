//! Why the clock refused to move.

use thiserror::Error;

/// Time could not pass. Nothing changes when one of these is returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ClockError {
    /// The new time would not fit in the clock's count of minutes.
    #[error("that much time cannot pass")]
    Overflow,
    /// The player is down, so no time passes by choice.
    #[error("you are down and cannot wait")]
    Down,
}
