//! Which mods exist, which weapons they fit, and what they change.

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use super::fitting::Fitting;
use crate::inventory::InventoryError;
use crate::shared::{GameData, ItemId, ModEffect};

/// Why the book could not be built.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum WeaponModError {
    /// A mod's effect has a number out of its range.
    #[error("{item}: {reason}")]
    BadEffect {
        /// The mod.
        item: ItemId,
        /// What is wrong with it.
        reason: String,
    },
    /// A mod names a weapon that is not in the data.
    #[error("{item}: fits unknown weapon {weapon}")]
    UnknownWeapon {
        /// The mod.
        item: ItemId,
        /// The weapon it names.
        weapon: ItemId,
    },
}

/// Why a mod could not be fitted or taken off.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum FitError {
    /// The player has no hit points left.
    #[error("the player is down")]
    Down,
    /// The mod does not fit this weapon.
    #[error("{item} does not fit {weapon}")]
    DoesNotFit {
        /// The mod.
        item: ItemId,
        /// The weapon.
        weapon: ItemId,
    },
    /// The pack refused: no such weapon, mod not carried, already fitted and so on.
    #[error(transparent)]
    Pack(#[from] InventoryError),
}

#[derive(Debug, Clone, PartialEq)]
struct Entry {
    fits: BTreeSet<ItemId>,
    effects: Vec<ModEffect>,
}

/// Every weapon mod, checked once at load.
#[derive(Debug, Clone, Default)]
pub struct WeaponModBook {
    mods: BTreeMap<ItemId, Entry>,
}

impl WeaponModBook {
    /// Reads every mod in the loaded data.
    ///
    /// # Errors
    ///
    /// [`WeaponModError`] naming the mod at fault.
    pub fn from_data(data: &GameData) -> Result<Self, WeaponModError> {
        let mut mods = BTreeMap::new();
        for record in data.weapon_mods.iter() {
            for effect in &record.effects {
                check(&record.id, effect)?;
            }
            let mut fits = BTreeSet::new();
            for weapon in &record.modifies {
                if !data.weapons.contains(weapon) {
                    return Err(WeaponModError::UnknownWeapon {
                        item: record.id.clone(),
                        weapon: weapon.clone(),
                    });
                }
                fits.insert(weapon.clone());
            }
            mods.insert(
                record.id.clone(),
                Entry {
                    fits,
                    effects: record.effects.clone(),
                },
            );
        }
        Ok(Self { mods })
    }

    /// Whether the mod exists.
    #[must_use]
    pub fn contains(&self, item: &ItemId) -> bool {
        self.mods.contains_key(item)
    }

    /// Whether the mod fits the weapon.
    #[must_use]
    pub fn fits(&self, item: &ItemId, weapon: &ItemId) -> bool {
        self.mods
            .get(item)
            .is_some_and(|entry| entry.fits.contains(weapon))
    }

    /// The mods that fit a weapon, in id order.
    pub fn mods_for(&self, weapon: &ItemId) -> Vec<&ItemId> {
        self.mods
            .iter()
            .filter(|(_, entry)| entry.fits.contains(weapon))
            .map(|(item, _)| item)
            .collect()
    }

    /// What one mod changes; empty for a mod the game cannot act on, and for one that
    /// does not exist.
    #[must_use]
    pub fn effects(&self, item: &ItemId) -> &[ModEffect] {
        self.mods
            .get(item)
            .map_or(&[], |entry| entry.effects.as_slice())
    }

    /// What the fitted mods change together. A mod the book does not know changes
    /// nothing.
    #[must_use]
    pub fn fitting(&self, fitted: &[ItemId]) -> Fitting {
        let mut total = Fitting::default();
        for item in fitted {
            total.add(self.effects(item));
        }
        total
    }

    /// How many mods the book holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.mods.len()
    }

    /// Whether the book is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.mods.is_empty()
    }
}

/// Refuses a number that would break a law: nothing finite and nothing that takes
/// damage or condition away.
fn check(item: &ItemId, effect: &ModEffect) -> Result<(), WeaponModError> {
    let (name, amount, ok) = match *effect {
        ModEffect::Damage { amount } => ("damage", amount, amount >= 0.0),
        ModEffect::ConditionHp { amount } => ("condition points", amount, amount >= 0.0),
        ModEffect::ConditionPercent { amount } => ("condition percent", amount, amount >= 0.0),
        ModEffect::ClipSize { amount } => ("rounds", f64::from(amount), true),
        ModEffect::Spread { amount } => ("spread", amount, amount <= 0.0),
    };
    if amount.is_finite() && ok {
        return Ok(());
    }
    Err(WeaponModError::BadEffect {
        item: item.clone(),
        reason: format!("{name} change of {amount} is not allowed"),
    })
}
