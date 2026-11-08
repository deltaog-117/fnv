//! Weapon mods: which mod fits which weapon, and what a set of fitted mods changes.
//!
//! [`WeaponModBook`] reads once, at load, which weapons each mod fits and the
//! numbers it changes. [`Fitting`] adds up what the mods on one weapon change, and
//! applies it to the weapon's combat numbers. The weapon copy itself remembers its
//! mods (see [`WeaponCopy`](crate::inventory::WeaponCopy)); this feature only
//! reads them, so it never touches the pack.
//!
//! This feature owns these laws and nothing else:
//!
//! - a mod fits only the weapons its record names, and a weapon never carries the
//!   same mod twice;
//! - what a weapon's mods change is the sum of what each changes, whatever the
//!   order they were fitted in;
//! - a mod whose text names nothing the game can act on changes nothing, and can
//!   still be fitted;
//! - a weapon's condition when new never falls to zero and never below the weapon's
//!   own, because no modeled effect takes condition away.
//!
//! Spending the mod from the pack, refusing a weapon that is not carried and
//! clamping the wear when a mod comes off belong to
//! [`Game`](crate::game::Game).

mod book;
mod fitting;

pub use book::{FitError, WeaponModBook, WeaponModError};
pub use fitting::Fitting;
