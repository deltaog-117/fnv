//! Station records: a crafting station standing at a place.

use serde::Deserialize;

use crate::shared::{LocationId, StationId, StationKind};

/// A crafting station the player can use.
///
/// The wiki lists no stations, so every record is authored by hand until the
/// game-data export exists. A place with no station record offers no crafting.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Station {
    /// The station's unique id.
    pub id: StationId,
    /// What the player sees when choosing to craft.
    pub name: String,
    /// The kind of station.
    pub kind: StationKind,
    /// The place where it stands.
    pub place: LocationId,
}
