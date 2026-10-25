//! Integration tests for turn-based combat.
//!
//! `damage` and `fight` pin each rule with small hand-built fights; `properties`
//! checks the laws over random fights; `scenarios` runs the language-neutral JSON
//! in `tests/scenarios/combat.json`; `real_data` checks the rules against
//! `data/engine/formulas.json`.

mod damage;
mod fight;
mod properties;
mod real_data;
mod scenarios;
mod support;
