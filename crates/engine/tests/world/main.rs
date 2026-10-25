//! Integration tests for the world state.
//!
//! `fates` pins each NPC, flag and ending rule with a small hand-built roster;
//! `properties` checks the laws over random effect sequences; `through_game`
//! runs world effects and conditions through the composed `Game`.

mod fates;
mod properties;
mod support;
mod through_game;
