//! The merchants that exist, read-only.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use super::error::TradeError;
use crate::shared::{
    Condition, Discount, GameData, ItemId, LocationId, NpcId, SkillLevel, Vendor, VendorId,
};

/// A merchant: who trades, where, and what they start with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorInfo {
    name: String,
    npc: NpcId,
    place: LocationId,
    caps: u32,
    stock: Vec<(ItemId, NonZeroU32)>,
    discount: Option<Discount>,
    flat_prices: Vec<Condition>,
    repair: Option<SkillLevel>,
    restock_days: NonZeroU32,
}

/// Days between restocks when a vendor does not say. The game's stand-in: the wiki
/// gives no restock interval.
pub const DEFAULT_RESTOCK_DAYS: NonZeroU32 = NonZeroU32::new(3).unwrap();

impl VendorInfo {
    /// Days between restocks.
    #[must_use]
    pub fn restock_days(&self) -> NonZeroU32 {
        self.restock_days
    }

    /// The Repair skill the vendor mends weapons with, if they offer repairs.
    #[must_use]
    pub fn repair_skill(&self) -> Option<SkillLevel> {
        self.repair
    }

    /// The price cut this vendor gives a player in good standing.
    #[must_use]
    pub fn discount(&self) -> Option<&Discount> {
        self.discount.as_ref()
    }

    /// The conditions under which this vendor deals at base value both ways; any one
    /// of them is enough, and none means never.
    #[must_use]
    pub fn flat_prices(&self) -> &[Condition] {
        &self.flat_prices
    }

    /// What the player sees when choosing to trade.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The person who trades.
    #[must_use]
    pub fn npc(&self) -> &NpcId {
        &self.npc
    }

    /// Where the vendor can be found.
    #[must_use]
    pub fn place(&self) -> &LocationId {
        &self.place
    }

    /// Caps held at the start.
    #[must_use]
    pub fn caps(&self) -> u32 {
        self.caps
    }

    /// The starting stock, each item once.
    pub fn stock(&self) -> impl Iterator<Item = (&ItemId, NonZeroU32)> {
        self.stock.iter().map(|(item, count)| (item, *count))
    }
}

/// Every merchant, checked once at load.
#[derive(Debug, Clone, Default)]
pub struct VendorBook {
    vendors: BTreeMap<VendorId, VendorInfo>,
}

impl VendorBook {
    /// Builds the book from the loaded data.
    ///
    /// # Errors
    ///
    /// The same as [`VendorBook::new`].
    pub fn from_data(data: &GameData) -> Result<Self, TradeError> {
        Self::new(data.vendors.iter())
    }

    /// Builds the book from vendor records.
    ///
    /// # Errors
    ///
    /// [`TradeError::DuplicateVendor`] or [`TradeError::DuplicateStock`] when an
    /// id or an item repeats.
    pub fn new<'a>(records: impl IntoIterator<Item = &'a Vendor>) -> Result<Self, TradeError> {
        let mut vendors = BTreeMap::new();
        for record in records {
            let mut stock: Vec<(ItemId, NonZeroU32)> = Vec::with_capacity(record.stock.len());
            for line in &record.stock {
                if stock.iter().any(|(item, _)| item == &line.item) {
                    return Err(TradeError::DuplicateStock {
                        vendor: record.id.clone(),
                        item: line.item.clone(),
                    });
                }
                stock.push((line.item.clone(), line.count));
            }
            let info = VendorInfo {
                name: record.name.clone(),
                npc: record.npc.clone(),
                place: record.place.clone(),
                caps: record.caps,
                stock,
                discount: record.discount.clone(),
                flat_prices: record.flat_prices.clone(),
                repair: record.repair.map(|service| service.skill),
                restock_days: record.restock_days.unwrap_or(DEFAULT_RESTOCK_DAYS),
            };
            if vendors.insert(record.id.clone(), info).is_some() {
                return Err(TradeError::DuplicateVendor(record.id.clone()));
            }
        }
        Ok(Self { vendors })
    }

    /// The vendor with this id.
    #[must_use]
    pub fn get(&self, vendor: &VendorId) -> Option<&VendorInfo> {
        self.vendors.get(vendor)
    }

    /// Every vendor, in id order.
    pub fn iter(&self) -> impl Iterator<Item = (&VendorId, &VendorInfo)> {
        self.vendors.iter()
    }

    /// The vendors found at a place.
    pub fn at<'a>(
        &'a self,
        place: &'a LocationId,
    ) -> impl Iterator<Item = (&'a VendorId, &'a VendorInfo)> {
        self.iter().filter(move |(_, info)| &info.place == place)
    }

    /// How many vendors there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.vendors.len()
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vendors.is_empty()
    }
}
