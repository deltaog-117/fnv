//! Integration tests for the inventory.
//!
//! `pack` pins each rule with a small hand-built item book; `properties` checks the
//! laws over random sequences of effects against a plain model; `real_data` builds
//! the book from `data/engine/` and equips every weapon and piece of armor; `uses`
//! covers item uses and throwing items away.

mod pack;
mod properties;
mod real_data;
mod support;
mod uses;
