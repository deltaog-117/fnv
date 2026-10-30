//! Integration tests for the game state.
//!
//! `state` checks that `Game` hands quest work to the quest log and refuses
//! effects aimed at features that are not built; `automation` covers stage effects
//! and rule firing; `pack` carries the character and the inventory through `Game`;
//! `real_data` runs a real quest's effects through `apply_all`; `items` uses and
//! drops pack items.

mod automation;
mod battle;
mod health;
mod items;
mod pack;
mod progress;
mod real_data;
mod scenarios;
mod state;
