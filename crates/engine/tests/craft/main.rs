//! Integration tests for crafting.
//!
//! `book` builds recipe books from records and checks the partition law with random
//! recipes; `real_data` builds the book and the stations from `data/engine/`; the
//! composed `Game` (skill, ingredients, stations, story) is covered in
//! `tests/game/upkeep.rs`.

mod book;
mod real_data;
