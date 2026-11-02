//! Why a map could not be built, or a trip was refused.

use thiserror::Error;

use crate::clock::ClockError;
use crate::shared::LocationId;

/// A map could not be built, or a trip was refused. Nothing changes when one of
/// these is returned.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum TravelError {
    /// Two location records share an id.
    #[error("location id {0} appears twice")]
    DuplicatePlace(LocationId),
    /// A link names a place that is not on the map.
    #[error("{from} links to {to}, which is not on the map")]
    UnknownPlace {
        /// The place that has the link.
        from: LocationId,
        /// The missing place.
        to: LocationId,
    },
    /// A link joins a place to itself.
    #[error("{0} links to itself")]
    SelfLink(LocationId),
    /// A link's distance is not a finite number above zero.
    #[error("the link from {from} to {to} has the distance {distance}")]
    BadDistance {
        /// One end.
        from: LocationId,
        /// The other end.
        to: LocationId,
        /// The distance as written.
        distance: f64,
    },
    /// The game was given no map, or the player has no place on it.
    #[error("there is no map to travel on")]
    NoMap,
    /// There is no place on the map with this id.
    #[error("{0} is not on the map")]
    NotOnMap(LocationId),
    /// The two places are not linked.
    #[error("there is no way from {from} to {to}")]
    NoRoute {
        /// Where the player is.
        from: LocationId,
        /// Where they asked to go.
        to: LocationId,
    },
    /// Time could not pass, so the trip could not be made.
    #[error(transparent)]
    Clock(#[from] ClockError),
}
