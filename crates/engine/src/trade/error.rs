//! Why a trade or a vendor list was refused.

use thiserror::Error;

use crate::shared::{EffectError, ItemId, NpcId, VendorId};

/// A vendor list could not be built, or a trade was refused. Nothing changes when
/// one of these is returned.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum TradeError {
    /// Two vendors share an id.
    #[error("vendor id {0} appears twice")]
    DuplicateVendor(VendorId),
    /// A vendor lists the same item twice.
    #[error("vendor {vendor} lists {item} twice")]
    DuplicateStock {
        /// The vendor.
        vendor: VendorId,
        /// The repeated item.
        item: ItemId,
    },
    /// A vendor stocks an item the game does not have, or one with no value.
    #[error("vendor {vendor} stocks {item}, which has no base value")]
    NoValue {
        /// The vendor.
        vendor: VendorId,
        /// The item.
        item: ItemId,
    },
    /// No such vendor.
    #[error("unknown vendor {0}")]
    UnknownVendor(VendorId),
    /// The vendor's NPC is dead or hostile.
    #[error("{0} will not trade")]
    Unavailable(NpcId),
    /// The item has no base value, or is worth less than a cap at this price.
    #[error("{0} has no price")]
    NoPrice(ItemId),
    /// The vendor holds fewer of the item than asked.
    #[error("the vendor has {have} of {item}, not {need}")]
    OutOfStock {
        /// The item.
        item: ItemId,
        /// How many the vendor has.
        have: u32,
        /// How many were asked for.
        need: u32,
    },
    /// The vendor cannot pay.
    #[error("the vendor holds {have} caps and cannot pay {need}")]
    VendorShort {
        /// Caps the vendor holds.
        have: u32,
        /// Caps the trade needs.
        need: u32,
    },
    /// The player cannot pay.
    #[error("you hold {have} caps and cannot pay {need}")]
    PlayerShort {
        /// Caps the player holds.
        have: u32,
        /// Caps the trade needs.
        need: u32,
    },
    /// The player carries fewer of the item than asked.
    #[error("you carry {have} of {item}, not {need}")]
    NotCarried {
        /// The item.
        item: ItemId,
        /// How many the player carries.
        have: u32,
        /// How many were asked for.
        need: u32,
    },
    /// The item is worn or wielded and the sale would give up that copy.
    #[error("{0} is equipped; unequip it first")]
    Equipped(ItemId),
    /// The player's side of the trade failed.
    #[error(transparent)]
    Effect(#[from] EffectError),
}
