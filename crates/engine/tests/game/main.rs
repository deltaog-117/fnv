//! Integration tests for the game state.
//!
//! `state` checks that `Game` hands quest work to the quest log and refuses
//! effects aimed at features that are not built; `automation` covers stage effects
//! and rule firing; `real_data` runs a real quest's effects through `apply_all`.

mod automation;
mod real_data;
mod scenarios;
mod state;
