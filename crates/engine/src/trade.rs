//! Vendors and barter: what a merchant stocks, holds and charges.
//!
//! [`VendorBook`] is the read-only list of merchants; [`Market`] is what they have
//! now, their stock and caps, which change with every trade. Prices come from
//! [`buy_total`] and [`sell_total`], which turn an item's base value and the
//! player's Barter skill into caps.
//!
//! This feature owns these laws and nothing else:
//!
//! - a vendor never sells more than it has, and never pays more caps than it holds;
//! - a stock line is never empty, and an item appears once in a vendor's stock;
//! - a trade changes a vendor's stock and caps together or not at all;
//! - a restock depends on the clock alone: waiting in one step or in many gives the
//!   same shops.
//!
//! The player's side of a trade (caps, items, what is equipped) belongs to the
//! inventory, so [`Game`](crate::game::Game) runs both halves as one unit.
//!
//! Vendors restock with the game clock: on every multiple of their interval
//! (3 days unless the record says) since the game began, a vendor goes back to the
//! stock and caps of the book. What the player sold them is gone with the refill.

mod book;
mod error;
mod market;
mod price;

pub use book::{DEFAULT_RESTOCK_DAYS, VendorBook, VendorInfo};
pub use error::TradeError;
pub use market::{Market, Shop};
pub use price::{
    buy_multiplier, buy_total, buy_total_with_discount, flat_buy_total, flat_sell_total,
    sell_multiplier, sell_total,
};
