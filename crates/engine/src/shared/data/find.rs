//! Find records: items the player can pick up, once, at a place.

use std::num::NonZeroU32;

use serde::Deserialize;

use crate::shared::{FindId, ItemId, LocationId, NonEmpty};

/// One line of a find.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FindLine {
    /// The item.
    pub item: ItemId,
    /// How many.
    pub count: NonZeroU32,
}

/// Something to pick up at a place. Taking it is a game action and it does not
/// come back.
///
/// The wiki lists no pickups, so every record is authored by hand until the
/// game-data export exists.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Find {
    /// The find's unique id.
    pub id: FindId,
    /// What the player sees when choosing to take it.
    pub name: String,
    /// The place where it lies.
    pub place: LocationId,
    /// What is taken; an item appears once.
    pub items: NonEmpty<FindLine>,
}
