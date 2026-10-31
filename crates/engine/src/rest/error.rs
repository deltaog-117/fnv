//! Why a bed list could not be built, or a sleep was refused.

use thiserror::Error;

use crate::clock::ClockError;
use crate::shared::BedId;

/// A bed list could not be built, or a sleep was refused. Nothing changes when
/// one of these is returned.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RestError {
    /// Two beds share an id.
    #[error("bed id {0} appears twice")]
    DuplicateBed(BedId),
    /// No such bed.
    #[error("unknown bed {0}")]
    UnknownBed(BedId),
    /// Time could not pass.
    #[error(transparent)]
    Clock(#[from] ClockError),
}
