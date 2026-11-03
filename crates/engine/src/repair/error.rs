//! Why a repair was refused.

use thiserror::Error;

use crate::shared::{EffectError, ItemId, NpcId, VendorId};

/// A repair was refused. Nothing changes when one of these is returned.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum RepairError {
    /// The player has no hit points left.
    #[error("the player is down")]
    Down,
    /// The item is not a weapon that wears.
    #[error("{0} does not wear, so it cannot be repaired")]
    DoesNotWear(ItemId),
    /// The player carries fewer of an item than the repair needs.
    #[error("you carry {have} of {item}, and the repair needs {need}")]
    NotCarried {
        /// The item.
        item: ItemId,
        /// How many the player carries.
        have: u32,
        /// How many the repair needs.
        need: u32,
    },
    /// The repair would leave the weapon as it is.
    #[error("{item} is already as good as Repair {skill} can make it")]
    NothingToGain {
        /// The weapon.
        item: ItemId,
        /// The Repair skill that sets the ceiling.
        skill: u8,
    },
    /// The weapon has no base value, so a vendor cannot price the repair.
    #[error("{0} has no price")]
    NoPrice(ItemId),
    /// No such vendor.
    #[error("unknown vendor {0}")]
    UnknownVendor(VendorId),
    /// The vendor does not mend weapons.
    #[error("{0} does not repair weapons")]
    NoService(VendorId),
    /// The vendor's person is dead or hostile.
    #[error("{0} will not trade")]
    Unavailable(NpcId),
    /// The player cannot pay.
    #[error("you hold {have} caps and the repair costs {need}")]
    PlayerShort {
        /// Caps the player holds.
        have: u32,
        /// Caps the repair costs.
        need: u32,
    },
    /// The player's side of the repair failed.
    #[error(transparent)]
    Effect(#[from] EffectError),
}
