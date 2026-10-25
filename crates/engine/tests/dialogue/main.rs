//! Integration tests for the dialogue runner.
//!
//! `conversation` pins each rule of the runner with a small hand-built dialogue;
//! `book` covers the checks made when the book is built; `properties` checks the
//! laws over random dialogue graphs; `real_data` and `scenarios` run the authored
//! Goodsprings dialogue from `data/engine/` and `tests/scenarios/`.

mod book;
mod conversation;
mod properties;
mod real_data;
mod scenarios;
mod support;
