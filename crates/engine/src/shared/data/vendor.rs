//! Vendor records: a merchant, what they stock and how many caps they hold.

use std::num::NonZeroU32;

use serde::Deserialize;

use crate::shared::{Condition, DiscountPercent, ItemId, LocationId, NpcId, SkillLevel, VendorId};

/// A cut in the price the player pays, while the player's standing earns it.
///
/// The percentages and the standings come from the wiki's reputation notes, so
/// they are authored per vendor and read with the same conditions rules use.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Discount {
    /// How much is taken off what the player pays.
    pub percent: DiscountPercent,
    /// At least one must hold for the cut to apply; empty means always.
    #[serde(default)]
    pub when: Vec<Condition>,
}

/// A repair service: the vendor mends worn weapons for caps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepairService {
    /// The vendor's own Repair skill: a weapon is mended to at most this percent.
    pub skill: SkillLevel,
}

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
    /// A price cut for a player in good standing, if the vendor gives one.
    pub discount: Option<Discount>,
    /// Mends worn weapons for caps, if the vendor offers it.
    pub repair: Option<RepairService>,
}
