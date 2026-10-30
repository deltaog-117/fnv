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
//! - a trade changes a vendor's stock and caps together or not at all.
//!
//! The player's side of a trade (caps, items, what is equipped) belongs to the
//! inventory, so [`Game`](crate::game::Game) runs both halves as one unit. Vendors
//! do not restock over time: that waits for the game clock.

mod book;
mod error;
mod market;
mod price;

pub use book::{VendorBook, VendorInfo};
pub use error::TradeError;
pub use market::{Market, Shop};
pub use price::{buy_multiplier, buy_total, sell_multiplier, sell_total};
