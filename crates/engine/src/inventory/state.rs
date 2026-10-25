//! What the player carries and wears.

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::sync::Arc;

use thiserror::Error;

use super::book::{ItemBook, ItemClass};
use crate::shared::{
    ArmorSlot, EffectError, FactionId, InventorySink, InventoryView, ItemId, whole_amount,
};

/// A place on the body or in the hand where one item can be equipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Slot {
    /// The wielded weapon.
    Weapon,
    /// Body armor or clothing.
    Outfit,
    /// A helmet or hat.
    Head,
}

impl Slot {
    /// The slot an item of this class goes in, if it can be equipped at all.
    fn for_class(class: ItemClass) -> Option<Self> {
        match class {
            ItemClass::Weapon => Some(Self::Weapon),
            ItemClass::Armor(ArmorSlot::Body | ArmorSlot::Clothing) => Some(Self::Outfit),
            ItemClass::Armor(ArmorSlot::Head) => Some(Self::Head),
            ItemClass::Ammo | ItemClass::Consumable => None,
        }
    }
}

/// Why an item book could not be built, or an item could not be equipped.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum InventoryError {
    /// An item's weight was negative, NaN or infinite.
    #[error("item {item} has an impossible weight: {weight}")]
    BadWeight {
        /// The item.
        item: ItemId,
        /// The rejected weight.
        weight: f64,
    },
    /// Two items share an id.
    #[error("item id {0} appears twice")]
    DuplicateItem(ItemId),
    /// The item is not in the book.
    #[error("unknown item {0}")]
    Unknown(ItemId),
    /// The player does not carry the item.
    #[error("{0} is not carried")]
    NotHeld(ItemId),
    /// The item is not something that can be equipped.
    #[error("{0} cannot be equipped")]
    NotEquippable(ItemId),
}

/// Everything the player carries: stacks, caps and what is equipped.
///
/// Cloning is cheap enough to try a batch on a copy: the book is shared and a
/// stack list is short.
#[derive(Debug, Clone)]
pub struct Inventory {
    book: Arc<ItemBook>,
    stacks: BTreeMap<ItemId, NonZeroU32>,
    caps: u32,
    equipped: BTreeMap<Slot, ItemId>,
}

impl Inventory {
    /// An empty pack over the items that exist.
    #[must_use]
    pub fn new(book: Arc<ItemBook>) -> Self {
        Self {
            book,
            stacks: BTreeMap::new(),
            caps: 0,
            equipped: BTreeMap::new(),
        }
    }

    /// The carried stacks in id order.
    pub fn stacks(&self) -> impl Iterator<Item = (&ItemId, NonZeroU32)> {
        self.stacks.iter().map(|(item, count)| (item, *count))
    }

    /// The item in a slot.
    #[must_use]
    pub fn equipped(&self, slot: Slot) -> Option<&ItemId> {
        self.equipped.get(&slot)
    }

    /// Total weight carried, in pounds.
    #[must_use]
    pub fn weight(&self) -> f64 {
        self.stacks
            .iter()
            .filter_map(|(item, count)| {
                self.book
                    .weight(item)
                    .map(|weight| weight * f64::from(count.get()))
            })
            .sum()
    }

    /// Equips a carried item in the slot it fits, replacing what was there.
    ///
    /// # Errors
    ///
    /// [`InventoryError::NotHeld`] when the item is not carried and
    /// [`InventoryError::NotEquippable`] when it has no slot.
    pub fn equip(&mut self, item: &ItemId) -> Result<Slot, InventoryError> {
        if !self.stacks.contains_key(item) {
            return Err(InventoryError::NotHeld(item.clone()));
        }
        let slot = self
            .book
            .class(item)
            .and_then(Slot::for_class)
            .ok_or_else(|| InventoryError::NotEquippable(item.clone()))?;
        self.equipped.insert(slot, item.clone());
        Ok(slot)
    }

    /// Empties a slot, returning what was in it.
    pub fn unequip(&mut self, slot: Slot) -> Option<ItemId> {
        self.equipped.remove(&slot)
    }
}

impl InventoryView for Inventory {
    fn item_count(&self, item: &ItemId) -> u32 {
        self.stacks.get(item).map_or(0, |count| count.get())
    }

    fn caps(&self) -> u32 {
        self.caps
    }

    fn is_equipped(&self, item: &ItemId) -> bool {
        self.equipped.values().any(|worn| worn == item)
    }

    fn disguises_as(&self, _item: &ItemId, _faction: &FactionId) -> bool {
        // Disguises are a reputation rule that is not built yet.
        false
    }
}

impl InventorySink for Inventory {
    fn add_caps(&mut self, amount: f64) -> Result<(), EffectError> {
        let change = whole_amount("caps", amount)?;
        let target = i64::from(self.caps) + change;
        if target < 0 {
            return Err(EffectError::NotEnoughCaps {
                have: self.caps,
                change: amount,
            });
        }
        self.caps = u32::try_from(target).map_err(|_| EffectError::Refused {
            effect: "caps",
            reason: format!("{target} caps is more than can be carried"),
        })?;
        Ok(())
    }

    fn give_item(&mut self, item: &ItemId, count: NonZeroU32) -> Result<(), EffectError> {
        if !self.book.contains(item) {
            return Err(EffectError::Unknown {
                kind: "item",
                id: item.as_str().to_owned(),
            });
        }
        let held = self.item_count(item);
        let total = held
            .checked_add(count.get())
            .and_then(NonZeroU32::new)
            .ok_or_else(|| EffectError::Refused {
                effect: "give_item",
                reason: format!("{held} + {count} of {item} is more than can be carried"),
            })?;
        self.stacks.insert(item.clone(), total);
        Ok(())
    }

    fn take_item(&mut self, item: &ItemId, count: NonZeroU32) -> Result<(), EffectError> {
        let have = self.item_count(item);
        let remaining =
            have.checked_sub(count.get())
                .ok_or_else(|| EffectError::NotEnoughItems {
                    item: item.clone(),
                    have,
                    need: count.get(),
                })?;
        if let Some(left) = NonZeroU32::new(remaining) {
            self.stacks.insert(item.clone(), left);
        } else {
            self.stacks.remove(item);
            self.equipped.retain(|_, worn| worn != item);
        }
        Ok(())
    }
}
