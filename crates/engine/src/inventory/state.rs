//! What the player carries and wears.

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::sync::Arc;

use thiserror::Error;

use super::book::{ItemBook, ItemClass};
use super::copies::{CopyId, WeaponCopy};
use crate::shared::{
    ArmorSlot, EffectError, FactionId, InventorySink, InventoryView, ItemId, ItemUse, whole_amount,
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
            ItemClass::Ammo | ItemClass::Consumable | ItemClass::Misc => None,
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
    /// An item's base value was negative, NaN or infinite.
    #[error("item {item} has an impossible value: {value}")]
    BadValue {
        /// The item.
        item: ItemId,
        /// The rejected value.
        value: f64,
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
    /// A use was given for an item that is not a consumable.
    #[error("item {0} is not a consumable, so it has no use")]
    NotConsumable(ItemId),
    /// The item has no effect the game models yet.
    #[error("{0} has no effect the game models yet")]
    NoEffect(ItemId),
    /// Fewer copies are carried than the action needs.
    #[error("{have} of {item} carried, {need} needed")]
    NotEnough {
        /// The item.
        item: ItemId,
        /// How many are carried.
        have: u32,
        /// How many the action needs.
        need: u32,
    },
    /// The last copy of the item is equipped.
    #[error("{0} is equipped; unequip it first")]
    Equipped(ItemId),
    /// No carried weapon has this id.
    #[error("no carried weapon is {0}")]
    NoSuchCopy(CopyId),
    /// A weapon id was listed twice, or the numbers ran out.
    #[error("weapon {0} cannot be added")]
    CopyTaken(CopyId),
    /// The player is already at full health.
    #[error("already at full health")]
    AtFullHealth,
    /// The item only takes radiation out, and the player has none.
    #[error("no radiation to take out")]
    NotIrradiated,
    /// The player has no hit points left.
    #[error("the player is down")]
    Down,
}

/// Everything the player carries: stacks, weapons, caps and what is equipped.
///
/// Weapons are kept one by one (see [`WeaponCopy`]); every other item is a counted
/// stack. Cloning is cheap enough to try a batch on a copy: the book is shared and
/// the lists are short.
#[derive(Debug, Clone)]
pub struct Inventory {
    book: Arc<ItemBook>,
    stacks: BTreeMap<ItemId, NonZeroU32>,
    weapons: BTreeMap<CopyId, WeaponCopy>,
    next_copy: Option<CopyId>,
    caps: u32,
    /// Armor and clothing; the weapon slot is `wielded`.
    equipped: BTreeMap<Slot, ItemId>,
    wielded: Option<CopyId>,
}

impl Inventory {
    /// An empty pack over the items that exist.
    #[must_use]
    pub fn new(book: Arc<ItemBook>) -> Self {
        Self {
            book,
            stacks: BTreeMap::new(),
            weapons: BTreeMap::new(),
            next_copy: Some(CopyId::FIRST),
            caps: 0,
            equipped: BTreeMap::new(),
            wielded: None,
        }
    }

    /// An empty pack over the same items, for loading a save into.
    #[must_use]
    pub fn emptied(&self) -> Self {
        Self::new(Arc::clone(&self.book))
    }

    /// The carried stacks in id order. Weapons are not stacks: see
    /// [`Inventory::weapons`].
    pub fn stacks(&self) -> impl Iterator<Item = (&ItemId, NonZeroU32)> {
        self.stacks.iter().map(|(item, count)| (item, *count))
    }

    /// The carried weapons in id order.
    pub fn weapons(&self) -> impl Iterator<Item = (CopyId, &WeaponCopy)> {
        self.weapons.iter().map(|(id, copy)| (*id, copy))
    }

    /// The carried copies of one kind of weapon, in id order.
    pub fn copies_of(&self, item: &ItemId) -> impl Iterator<Item = CopyId> + '_ {
        let item = item.clone();
        self.weapons()
            .filter(move |(_, copy)| copy.item() == &item)
            .map(|(id, _)| id)
    }

    /// One carried weapon.
    #[must_use]
    pub fn weapon(&self, id: CopyId) -> Option<&WeaponCopy> {
        self.weapons.get(&id)
    }

    /// The weapon in hand.
    #[must_use]
    pub fn wielded(&self) -> Option<CopyId> {
        self.wielded
    }

    /// The item in a slot. For the weapon slot it is the kind of the weapon in hand.
    #[must_use]
    pub fn equipped(&self, slot: Slot) -> Option<&ItemId> {
        match slot {
            Slot::Weapon => self
                .wielded
                .and_then(|id| self.weapons.get(&id))
                .map(|copy| &copy.item),
            Slot::Outfit | Slot::Head => self.equipped.get(&slot),
        }
    }

    /// The item's base value in caps, if the book gives one.
    #[must_use]
    pub fn value(&self, item: &ItemId) -> Option<f64> {
        self.book.value(item)
    }

    /// The slot the item goes in when equipped, if it fits one at all.
    #[must_use]
    pub fn slot_for(&self, item: &ItemId) -> Option<Slot> {
        self.book.class(item).and_then(Slot::for_class)
    }

    /// What using the item does, if the game models it.
    #[must_use]
    pub fn item_use(&self, item: &ItemId) -> Option<ItemUse> {
        self.book.item_use(item)
    }

    /// Total weight carried, in pounds.
    #[must_use]
    pub fn weight(&self) -> f64 {
        let stacks: f64 = self
            .stacks
            .iter()
            .filter_map(|(item, count)| {
                self.book
                    .weight(item)
                    .map(|weight| weight * f64::from(count.get()))
            })
            .sum();
        let weapons: f64 = self
            .weapons
            .values()
            .filter_map(|copy| self.book.weight(&copy.item))
            .sum();
        stacks + weapons
    }

    /// Equips a carried item in the slot it fits, replacing what was there. Of
    /// several copies of a weapon, the least worn goes in hand.
    ///
    /// # Errors
    ///
    /// [`InventoryError::NotHeld`] when the item is not carried and
    /// [`InventoryError::NotEquippable`] when it has no slot.
    pub fn equip(&mut self, item: &ItemId) -> Result<Slot, InventoryError> {
        if self.is_weapon(item) {
            let best = self
                .weapons
                .iter()
                .filter(|(_, copy)| &copy.item == item)
                .min_by_key(|(id, copy)| (copy.lost, **id))
                .map(|(id, _)| *id)
                .ok_or_else(|| InventoryError::NotHeld(item.clone()))?;
            return self.equip_copy(best);
        }
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

    /// Puts one particular carried weapon in hand.
    ///
    /// # Errors
    ///
    /// [`InventoryError::NoSuchCopy`] when no carried weapon has that id.
    pub fn equip_copy(&mut self, id: CopyId) -> Result<Slot, InventoryError> {
        if !self.weapons.contains_key(&id) {
            return Err(InventoryError::NoSuchCopy(id));
        }
        self.wielded = Some(id);
        Ok(Slot::Weapon)
    }

    /// Records the condition points a carried weapon has lost.
    ///
    /// # Errors
    ///
    /// [`InventoryError::NoSuchCopy`] when no carried weapon has that id.
    pub fn set_lost(&mut self, id: CopyId, lost: u32) -> Result<(), InventoryError> {
        self.weapons
            .get_mut(&id)
            .map(|copy| copy.lost = lost)
            .ok_or(InventoryError::NoSuchCopy(id))
    }

    /// Puts a weapon back into the pack under the id a save gave it.
    ///
    /// # Errors
    ///
    /// [`InventoryError::Unknown`] when the item is not a weapon in the book and
    /// [`InventoryError::CopyTaken`] when the id is already in use.
    pub fn restore_weapon(
        &mut self,
        id: CopyId,
        item: &ItemId,
        lost: u32,
    ) -> Result<(), InventoryError> {
        if !self.is_weapon(item) {
            return Err(InventoryError::Unknown(item.clone()));
        }
        if self.weapons.contains_key(&id) {
            return Err(InventoryError::CopyTaken(id));
        }
        self.weapons.insert(
            id,
            WeaponCopy {
                item: item.clone(),
                lost,
            },
        );
        // Later weapons must never reuse an id a save already handed out.
        if self.next_copy.is_some_and(|next| next <= id) {
            self.next_copy = id.next();
        }
        Ok(())
    }

    /// Throws one weapon away.
    ///
    /// # Errors
    ///
    /// [`InventoryError::NoSuchCopy`] when no carried weapon has that id and
    /// [`InventoryError::Equipped`] when it is the one in hand.
    pub fn discard_copy(&mut self, id: CopyId) -> Result<(), InventoryError> {
        let copy = self
            .weapons
            .get(&id)
            .ok_or(InventoryError::NoSuchCopy(id))?;
        if self.wielded == Some(id) {
            return Err(InventoryError::Equipped(copy.item.clone()));
        }
        self.weapons.remove(&id);
        Ok(())
    }

    /// Throws `count` copies away. The last copy of an equipped item stays, so
    /// the pack never keeps a slot filled with something it no longer holds. Of
    /// weapons the most worn go first and the one in hand last.
    ///
    /// # Errors
    ///
    /// [`InventoryError::NotHeld`] when none is carried,
    /// [`InventoryError::NotEnough`] when fewer than `count` are, and
    /// [`InventoryError::Equipped`] when the last copy is equipped.
    pub fn discard(&mut self, item: &ItemId, count: NonZeroU32) -> Result<(), InventoryError> {
        let have = self.item_count(item);
        if have == 0 {
            return Err(InventoryError::NotHeld(item.clone()));
        }
        let Some(left) = have.checked_sub(count.get()) else {
            return Err(InventoryError::NotEnough {
                item: item.clone(),
                have,
                need: count.get(),
            });
        };
        if self.is_weapon(item) {
            if left == 0 && self.is_equipped(item) {
                return Err(InventoryError::Equipped(item.clone()));
            }
            self.remove_weapons(item, count.get());
            return Ok(());
        }
        match NonZeroU32::new(left) {
            Some(left) => {
                self.stacks.insert(item.clone(), left);
            }
            None if self.is_equipped(item) => return Err(InventoryError::Equipped(item.clone())),
            None => {
                self.stacks.remove(item);
            }
        }
        Ok(())
    }

    /// Empties a slot, returning what was in it.
    pub fn unequip(&mut self, slot: Slot) -> Option<ItemId> {
        match slot {
            Slot::Weapon => {
                let id = self.wielded.take()?;
                self.weapons.get(&id).map(|copy| copy.item.clone())
            }
            Slot::Outfit | Slot::Head => self.equipped.remove(&slot),
        }
    }

    /// Merges the weapon `gone` into `keep`: `gone` leaves the pack and `keep`
    /// ends with `lost` points lost. If `gone` was in hand, `keep` takes its place.
    ///
    /// # Errors
    ///
    /// [`InventoryError::NoSuchCopy`] when either id is unknown or they are the
    /// same, and [`InventoryError::NotEquippable`] when they are different kinds.
    pub fn merge_copies(
        &mut self,
        keep: CopyId,
        gone: CopyId,
        lost: u32,
    ) -> Result<(), InventoryError> {
        let kind = |id: CopyId| self.weapons.get(&id).map(|copy| copy.item.clone());
        let (Some(kept), Some(other)) = (kind(keep), kind(gone)) else {
            let missing = if self.weapons.contains_key(&keep) {
                gone
            } else {
                keep
            };
            return Err(InventoryError::NoSuchCopy(missing));
        };
        if keep == gone {
            return Err(InventoryError::NoSuchCopy(gone));
        }
        if kept != other {
            return Err(InventoryError::NotEquippable(other));
        }
        self.weapons.remove(&gone);
        if self.wielded == Some(gone) {
            self.wielded = Some(keep);
        }
        self.set_lost(keep, lost)
    }

    fn is_weapon(&self, item: &ItemId) -> bool {
        self.book.class(item) == Some(ItemClass::Weapon)
    }

    /// Removes `count` copies of a weapon: unwielded ones first, the most worn
    /// first, the one in hand last. The caller has checked there are enough.
    fn remove_weapons(&mut self, item: &ItemId, count: u32) {
        let mut copies: Vec<(bool, std::cmp::Reverse<u32>, CopyId)> = self
            .weapons
            .iter()
            .filter(|(_, copy)| &copy.item == item)
            .map(|(id, copy)| (self.wielded == Some(*id), std::cmp::Reverse(copy.lost), *id))
            .collect();
        copies.sort();
        for (_, _, id) in copies.into_iter().take(count as usize) {
            self.weapons.remove(&id);
            if self.wielded == Some(id) {
                self.wielded = None;
            }
        }
    }
}

impl InventoryView for Inventory {
    fn item_count(&self, item: &ItemId) -> u32 {
        if self.is_weapon(item) {
            let held = self
                .weapons
                .values()
                .filter(|copy| &copy.item == item)
                .count();
            return u32::try_from(held).unwrap_or(u32::MAX);
        }
        self.stacks.get(item).map_or(0, |count| count.get())
    }

    fn caps(&self) -> u32 {
        self.caps
    }

    fn is_equipped(&self, item: &ItemId) -> bool {
        self.equipped.values().any(|worn| worn == item)
            || self
                .wielded
                .and_then(|id| self.weapons.get(&id))
                .is_some_and(|copy| &copy.item == item)
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
        if self.is_weapon(item) {
            return self.add_weapons(item, count.get());
        }
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
        if self.is_weapon(item) {
            self.remove_weapons(item, count.get());
            return Ok(());
        }
        if let Some(left) = NonZeroU32::new(remaining) {
            self.stacks.insert(item.clone(), left);
        } else {
            self.stacks.remove(item);
            self.equipped.retain(|_, worn| worn != item);
        }
        Ok(())
    }
}

impl Inventory {
    /// Adds `count` new weapons, all or none: the ids must not run out.
    fn add_weapons(&mut self, item: &ItemId, count: u32) -> Result<(), EffectError> {
        let mut next = self.next_copy;
        let mut ids = Vec::new();
        for _ in 0..count {
            let id = next.ok_or_else(|| EffectError::Refused {
                effect: "give_item",
                reason: "too many weapons have been carried".to_owned(),
            })?;
            ids.push(id);
            next = id.next();
        }
        for id in ids {
            self.weapons.insert(
                id,
                WeaponCopy {
                    item: item.clone(),
                    lost: 0,
                },
            );
        }
        self.next_copy = next;
        Ok(())
    }
}
