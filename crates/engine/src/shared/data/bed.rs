//! Bed records: a place where the player may sleep.

use serde::Deserialize;

use crate::shared::{BedId, LocationId};

/// A bed the player can sleep in.
///
/// The wiki lists no beds, so every record is authored by hand until the
/// game-data export exists. A place with no bed record offers no sleep.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bed {
    /// The bed's unique id.
    pub id: BedId,
    /// What the player sees when choosing to sleep.
    pub name: String,
    /// The place where the bed stands.
    pub place: LocationId,
}
