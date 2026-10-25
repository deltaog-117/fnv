//! Integration tests for the shared vocabulary.
//!
//! These run against the public API only. `contract` keeps the Rust types and
//! `schemas/common.schema.json` in step; `ids` checks the id and bounds rules;
//! `loader` loads the real data and checks that broken data fails clearly;
//! `evaluator` checks what conditions and effects mean, using the stand-in states
//! in `memory`.

mod contract;
mod evaluator;
mod ids;
mod loader;
mod memory;
mod strategies;
