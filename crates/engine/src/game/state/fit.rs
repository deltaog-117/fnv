//! Fitting weapon mods to the weapons the player carries.

use super::Game;
use crate::inventory::{CopyId, InventoryError, WeaponCopy};
use crate::shared::ItemId;
use crate::weapon_mods::{FitError, Fitting};

impl Game {
    /// What the mods fitted to a carried weapon change together.
    pub(super) fn fitting_of(&self, copy: &WeaponCopy) -> Fitting {
        self.weapon_mods.fitting(copy.mods())
    }

    /// The mods that fit a kind of weapon, in id order.
    #[must_use]
    pub fn mods_for(&self, weapon: &ItemId) -> Vec<&ItemId> {
        self.weapon_mods.mods_for(weapon)
    }

    /// The mods fitted to a carried weapon, in id order; none for a weapon that is
    /// not carried.
    #[must_use]
    pub fn fitted_mods(&self, copy: CopyId) -> &[ItemId] {
        self.inventory.weapon(copy).map_or(&[], WeaponCopy::mods)
    }

    /// What the mods fitted to a carried weapon change together.
    #[must_use]
    pub fn fitting(&self, copy: CopyId) -> Fitting {
        self.inventory
            .weapon(copy)
            .map(|held| self.fitting_of(held))
            .unwrap_or_default()
    }

    /// Fits a carried mod to a carried weapon. The mod leaves the pack and stays on
    /// the weapon until it is taken off. A mod whose effect the game cannot act on
    /// yet (a silencer, a scope) fits all the same and changes nothing.
    ///
    /// # Errors
    ///
    /// [`FitError::Down`] while the player has no hit points,
    /// [`FitError::DoesNotFit`] when the mod is not listed for that weapon, and
    /// [`FitError::Pack`] when the weapon or the mod is not carried or the mod is
    /// already fitted.
    pub fn fit_mod(&mut self, copy: CopyId, item: &ItemId) -> Result<(), FitError> {
        if !self.is_alive() {
            return Err(FitError::Down);
        }
        let weapon = self
            .inventory
            .weapon(copy)
            .ok_or(InventoryError::NoSuchCopy(copy))?
            .item()
            .clone();
        if !self.weapon_mods.fits(item, &weapon) {
            return Err(FitError::DoesNotFit {
                item: item.clone(),
                weapon,
            });
        }
        self.inventory.fit_mod(copy, item)?;
        Ok(())
    }

    /// Takes a mod off a carried weapon and back into the pack. A weapon that lost
    /// more condition than it can hold without the mod is left at its new limit.
    ///
    /// # Errors
    ///
    /// [`FitError::Down`] while the player has no hit points and
    /// [`FitError::Pack`] when the weapon is not carried or lacks the mod.
    pub fn remove_mod(&mut self, copy: CopyId, item: &ItemId) -> Result<(), FitError> {
        if !self.is_alive() {
            return Err(FitError::Down);
        }
        self.inventory.remove_mod(copy, item)?;
        if let Some(held) = self.inventory.weapon(copy)
            && let Some(max) = self
                .arsenal
                .condition_max(held.item(), &self.fitting_of(held))
        {
            let lost = held.lost().min(max.get());
            // The weapon was found just above, so the pack cannot refuse.
            let _ = self.inventory.set_lost(copy, lost);
        }
        Ok(())
    }
}
