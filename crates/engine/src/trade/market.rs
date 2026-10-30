//! What each merchant has now: stock and caps.

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::sync::Arc;

use super::book::VendorBook;
use super::error::TradeError;
use crate::shared::{ItemId, VendorId};

/// One vendor's stock and caps right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shop {
    caps: u32,
    stock: BTreeMap<ItemId, NonZeroU32>,
}

impl Shop {
    /// Caps the vendor holds.
    #[must_use]
    pub fn caps(&self) -> u32 {
        self.caps
    }

    /// How many of an item the vendor has.
    #[must_use]
    pub fn count(&self, item: &ItemId) -> u32 {
        self.stock.get(item).map_or(0, |count| count.get())
    }

    /// The stock in id order.
    pub fn stock(&self) -> impl Iterator<Item = (&ItemId, NonZeroU32)> {
        self.stock.iter().map(|(item, count)| (item, *count))
    }
}

/// Every vendor's shop. Cloning is cheap enough to try a trade on a copy: the
/// book is shared and a stock list is short.
#[derive(Debug, Clone, Default)]
pub struct Market {
    book: Arc<VendorBook>,
    shops: BTreeMap<VendorId, Shop>,
}

impl Market {
    /// A market where every vendor holds what the book says they start with.
    #[must_use]
    pub fn new(book: Arc<VendorBook>) -> Self {
        let shops = book
            .iter()
            .map(|(id, info)| {
                let shop = Shop {
                    caps: info.caps(),
                    stock: info
                        .stock()
                        .map(|(item, count)| (item.clone(), count))
                        .collect(),
                };
                (id.clone(), shop)
            })
            .collect();
        Self { book, shops }
    }

    /// The merchants that exist.
    #[must_use]
    pub fn book(&self) -> &VendorBook {
        &self.book
    }

    /// A vendor's shop now.
    #[must_use]
    pub fn shop(&self, vendor: &VendorId) -> Option<&Shop> {
        self.shops.get(vendor)
    }

    /// The vendor hands over `count` of an item and is paid `price` caps.
    ///
    /// # Errors
    ///
    /// [`TradeError::UnknownVendor`], or [`TradeError::OutOfStock`] when the vendor
    /// has fewer than `count`. Nothing changes on an error.
    pub fn sell_to_player(
        &mut self,
        vendor: &VendorId,
        item: &ItemId,
        count: NonZeroU32,
        price: u32,
    ) -> Result<(), TradeError> {
        let shop = self.shop_mut(vendor)?;
        let have = shop.count(item);
        let left = have
            .checked_sub(count.get())
            .ok_or_else(|| TradeError::OutOfStock {
                item: item.clone(),
                have,
                need: count.get(),
            })?;
        let caps = shop
            .caps
            .checked_add(price)
            .ok_or(TradeError::VendorShort {
                have: shop.caps,
                need: price,
            })?;
        match NonZeroU32::new(left) {
            Some(left) => shop.stock.insert(item.clone(), left),
            None => shop.stock.remove(item),
        };
        shop.caps = caps;
        Ok(())
    }

    /// The vendor takes `count` of an item and pays `price` caps.
    ///
    /// # Errors
    ///
    /// [`TradeError::UnknownVendor`], or [`TradeError::VendorShort`] when the
    /// vendor holds fewer than `price` caps. Nothing changes on an error.
    pub fn buy_from_player(
        &mut self,
        vendor: &VendorId,
        item: &ItemId,
        count: NonZeroU32,
        price: u32,
    ) -> Result<(), TradeError> {
        let shop = self.shop_mut(vendor)?;
        let caps = shop
            .caps
            .checked_sub(price)
            .ok_or(TradeError::VendorShort {
                have: shop.caps,
                need: price,
            })?;
        let held = shop.count(item);
        let total = held
            .checked_add(count.get())
            .and_then(NonZeroU32::new)
            .ok_or(TradeError::OutOfStock {
                item: item.clone(),
                have: held,
                need: count.get(),
            })?;
        shop.stock.insert(item.clone(), total);
        shop.caps = caps;
        Ok(())
    }

    fn shop_mut(&mut self, vendor: &VendorId) -> Result<&mut Shop, TradeError> {
        self.shops
            .get_mut(vendor)
            .ok_or_else(|| TradeError::UnknownVendor(vendor.clone()))
    }
}
