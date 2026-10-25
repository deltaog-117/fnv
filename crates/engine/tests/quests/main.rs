//! Integration tests for the quest log.
//!
//! `lifecycle` pins each rule of the quest lifecycle with a small hand-built
//! quest; `properties` checks the laws over random effect sequences; `real_data`
//! walks every quest in `data/engine/`.

mod lifecycle;
mod properties;
mod real_data;
mod support;
