//! Integration tests for the game state.
//!
//! `state` checks that `Game` hands quest work to the quest log and refuses
//! effects aimed at features that are not built; `automation` covers stage effects
//! and rule firing; `pack` carries the character and the inventory through `Game`;
//! `real_data` runs a real quest's effects through `apply_all`; `items` uses and
//! drops pack items; `clock` checks that only waiting moves the time; `rest` sleeps
//! in a bed; `boosts` runs chems and time together; `reputation` runs fame, infamy and karma through the composed game; `upkeep` repairs, crafts and gathers; `situation` runs the perks that depend on wounds and the hour.

mod ailments;
mod automation;
mod battle;
mod boosts;
mod clock;
mod companions;
mod health;
mod items;
mod pack;
mod poison;
mod progress;
mod real_data;
mod reputation;
mod rest;
mod scenarios;
mod situation;
mod state;
mod upkeep;
