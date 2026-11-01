//! Integration tests for reputation and karma.
//!
//! `model` pins the tiers, the seven standings and the book with small hand-built
//! factions; `effects` pins each effect and its refusals; `properties` checks the
//! laws over random effects; `real_data` checks the 13 modeled factions in
//! `data/engine/`. The composed `Game` is covered in `tests/game/reputation.rs`.

mod effects;
mod model;
mod properties;
mod real_data;
mod support;
