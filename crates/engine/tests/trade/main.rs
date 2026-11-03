//! Integration tests for vendors and barter.
//!
//! `discount` checks a vendor's standing-earned price cut; `prices` pins the Barter formulas; `trading` runs buys and sells through `Game`
//! over a small hand-built market; `properties` checks that no random sequence of
//! trades creates or loses caps or items; `real_data` loads the Goodsprings vendors
//! from `data/engine/` and trades with each; `restock` checks that shops refill with the
//! clock.

mod discount;
mod prices;
mod properties;
mod real_data;
mod restock;
mod support;
mod trading;
