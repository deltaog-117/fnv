//! Vendor records: a merchant, what they stock and how many caps they hold.

use std::num::NonZeroU32;

use serde::Deserialize;

use crate::shared::{ItemId, LocationId, NpcId, VendorId};

/// One line of a vendor's stock.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StockLine {
    /// The item for sale.
    pub item: ItemId,
    /// How many the vendor starts with.
    pub count: NonZeroU32,
}

/// A merchant the player can trade with.
///
/// The wiki records no vendor stock, so every record is authored by hand until the
/// game-data export exists.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vendor {
    /// The vendor's unique id.
    pub id: VendorId,
    /// What the player sees when choosing to trade.
    pub name: String,
    /// The person who trades; a dead or hostile vendor does not.
    pub npc: NpcId,
    /// The place where the vendor can be found.
    pub place: LocationId,
    /// Caps the vendor can pay with, and starts with.
    pub caps: u32,
    /// What the vendor starts with; an item appears once.
    #[serde(default)]
    pub stock: Vec<StockLine>,
}
