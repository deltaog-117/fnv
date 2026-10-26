//! Integration tests for the player character.
//!
//! `sheet` pins each derived number and each refusal with small hand-built
//! characters; `progression` does the same for skill points, tags, traits and perks; `properties` checks the laws over random attributes and XP;
//! `real_data` checks the formulas and constants against
//! `data/engine/formulas.json` and `data/engine/levels.json`.

mod progression;
mod properties;
mod real_data;
mod sheet;
