//! Weapons the player carries, one object per copy.
//!
//! A stack of three pistols would share one wear number; here each pistol is its
//! own record with its own id and the condition points it has lost. Everything else
//! the pack holds (ammo, armor, chems) stays a counted stack. A copy also keeps the
//! weapon mods fitted to it.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::shared::ItemId;

/// The identity of one carried weapon. Ids are never reused within a pack, so a
/// stale id can only miss, never name another weapon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CopyId(u32);

impl CopyId {
    /// The id as the number a save stores.
    #[must_use]
    pub fn get(self) -> u32 {
        self.0
    }

    /// The id a save stored. Whether it names a weapon is for the pack to say.
    #[must_use]
    pub fn from_saved(number: u32) -> Self {
        Self(number)
    }

    /// The id after this one, or `None` at the end of the numbers.
    pub(super) fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }

    /// The first id of an empty pack.
    pub(super) const FIRST: Self = Self(0);
}

impl fmt::Display for CopyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// One carried weapon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeaponCopy {
    pub(super) item: ItemId,
    pub(super) lost: u32,
    /// The mods fitted, in id order, each at most once.
    pub(super) mods: Vec<ItemId>,
}

impl WeaponCopy {
    /// What kind of weapon it is.
    #[must_use]
    pub fn item(&self) -> &ItemId {
        &self.item
    }

    /// Condition points this copy has lost. The pack does not know a weapon's
    /// maximum; the game clamps the number when it reads it.
    #[must_use]
    pub fn lost(&self) -> u32 {
        self.lost
    }

    /// The mods fitted to this copy, in id order.
    #[must_use]
    pub fn mods(&self) -> &[ItemId] {
        &self.mods
    }
}
