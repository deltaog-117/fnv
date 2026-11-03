//! Repairing worn weapons: how far a repair can lift a weapon, and what a
//! vendor charges for it.
//!
//! This feature owns these laws and nothing else:
//!
//! - a repair never makes a weapon worse, and never lifts it past what is left of
//!   the weapon when new;
//! - a repair lifts a weapon to the Repair skill's share of its condition, so a
//!   higher skill never gives a lower result;
//! - merging two copies never leaves the result worse than the better copy;
//! - a repair that changes nothing costs nothing, and a larger repair never costs
//!   less than a smaller one.
//!
//! A weapon's wear, the items a repair uses up and the caps a vendor is paid
//! belong to [`Game`](crate::game::Game), which runs each repair as one unit.
//! Every rate here is a stand-in: the wiki gives no repair formula, so the
//! numbers follow the shape the wiki describes (a skill ceiling, a bonus for
//! merging two copies) and wait for the game-data export.

mod error;
mod law;

pub use error::RepairError;
pub use law::{
    MERGE_BONUS_PERCENT, PRICE_SHARE_PERCENT, WEAPON_REPAIR_KIT, after_kit, after_merge, ceiling,
    vendor_price,
};
