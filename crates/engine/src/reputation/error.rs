//! Why a reputation book could not be built.

use thiserror::Error;

use crate::shared::FactionId;

/// A reputation book could not be built. Nothing is built when one of these is
/// returned.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReputationError {
    /// Two factions share an id.
    #[error("faction id {0} appears twice")]
    DuplicateFaction(FactionId),
    /// A faction has a reputation record without both thresholds and titles.
    #[error("faction {0} has a reputation model without thresholds or titles")]
    Incomplete(FactionId),
    /// The thresholds do not start at zero and rise.
    #[error("faction {faction}: tier thresholds {thresholds:?} must start at 0 and rise")]
    BadThresholds {
        /// The faction.
        faction: FactionId,
        /// The rejected thresholds.
        thresholds: [u32; 4],
    },
}
