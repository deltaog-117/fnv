//! Items and equipment, crafting and barter.
//!
//! So far this holds what the player carries: item stacks, bottle caps and the
//! three equipment slots. [`ItemBook`] is the read-only list of items that exist;
//! [`Inventory`] is what the player has. It implements the shared `InventoryView`
//! and `InventorySink`, so conditions and effects run against it unchanged.
//!
//! The inventory owns these laws and nothing else:
//!
//! - only items in the book can be held, and a stack is never empty;
//! - caps never go below zero, and amounts are whole numbers;
//! - an equipped item is always held, sits in a slot it fits, and stops being
//!   equipped when its last copy leaves;
//! - weight never blocks picking something up: being over the carry limit is for
//!   the game to read, not for the pack to refuse.
//!
//! Using consumables, repair, crafting and barter are not built.

mod book;
mod state;

pub use book::{ItemBook, ItemClass};
pub use state::{Inventory, InventoryError, Slot};
