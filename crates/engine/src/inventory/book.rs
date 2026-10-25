//! The items that exist, read-only.

use std::collections::HashMap;

use super::state::InventoryError;
use crate::shared::{ArmorSlot, GameData, ItemId};

/// What an item is, as far as carrying and wearing it goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemClass {
    /// Wielded.
    Weapon,
    /// Worn in the slot its armor record names.
    Armor(ArmorSlot),
    /// Ammunition.
    Ammo,
    /// Food, drink, chems and medicine.
    Consumable,
}

#[derive(Debug, Clone, Copy)]
struct Info {
    class: ItemClass,
    weight: f64,
}

/// Every item that can be carried, with its class and weight.
///
/// The book is what lets the inventory refuse an item that does not exist, instead
/// of keeping a stack under a typo. Weapon mods, skill books and crafting
/// components are not in the loaded data yet, so they cannot be carried.
#[derive(Debug, Default)]
pub struct ItemBook {
    items: HashMap<ItemId, Info>,
}

impl ItemBook {
    /// Builds the book from `(id, class, weight in pounds)` entries.
    ///
    /// # Errors
    ///
    /// [`InventoryError::BadWeight`] for a negative or non-finite weight, and
    /// [`InventoryError::DuplicateItem`] when an id appears twice.
    pub fn new(
        entries: impl IntoIterator<Item = (ItemId, ItemClass, f64)>,
    ) -> Result<Self, InventoryError> {
        let mut items = HashMap::new();
        for (id, class, weight) in entries {
            if !weight.is_finite() || weight < 0.0 {
                return Err(InventoryError::BadWeight { item: id, weight });
            }
            if items.insert(id.clone(), Info { class, weight }).is_some() {
                return Err(InventoryError::DuplicateItem(id));
            }
        }
        Ok(Self { items })
    }

    /// Every weapon, armor, ammo and consumable in the loaded data.
    ///
    /// # Errors
    ///
    /// The same as [`ItemBook::new`].
    pub fn from_data(data: &GameData) -> Result<Self, InventoryError> {
        let weapons = data
            .weapons
            .iter()
            .map(|item| (item.id.clone(), ItemClass::Weapon, item.weight));
        let armor = data
            .armor
            .iter()
            .map(|item| (item.id.clone(), ItemClass::Armor(item.slot), item.weight));
        let ammo = data
            .ammo
            .iter()
            .map(|item| (item.id.clone(), ItemClass::Ammo, item.weight));
        let consumables = data
            .consumables
            .iter()
            .map(|item| (item.id.clone(), ItemClass::Consumable, item.weight));
        Self::new(weapons.chain(armor).chain(ammo).chain(consumables))
    }

    /// Whether the item exists.
    #[must_use]
    pub fn contains(&self, item: &ItemId) -> bool {
        self.items.contains_key(item)
    }

    /// The item's class, if it exists.
    #[must_use]
    pub fn class(&self, item: &ItemId) -> Option<ItemClass> {
        self.items.get(item).map(|info| info.class)
    }

    /// The item's weight in pounds, if it exists.
    #[must_use]
    pub fn weight(&self, item: &ItemId) -> Option<f64> {
        self.items.get(item).map(|info| info.weight)
    }

    /// How many items the book holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the book is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
