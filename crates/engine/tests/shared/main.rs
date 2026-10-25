//! Integration tests for the shared vocabulary.
//!
//! These run against the public API only. `contract` keeps the Rust types and
//! `schemas/common.schema.json` in step; `ids` checks the id and bounds rules.

mod contract;
mod ids;
mod strategies;
