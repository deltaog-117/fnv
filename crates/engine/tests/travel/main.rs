//! Integration tests for travel.
//!
//! `map` pins how the book is built from location records; `journey` runs trips
//! through `Game`; `properties` checks that no random walk breaks the laws;
//! `real_data` builds the map from `data/engine/` and walks out of Goodsprings.

mod journey;
mod map;
mod properties;
mod real_data;
mod support;
