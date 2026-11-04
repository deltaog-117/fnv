//! Integration tests for the player character.
//!
//! `sheet` pins each derived number and each refusal with small hand-built
//! characters; `progression` does the same for skill points, tags, traits and perks; `properties` checks the laws over random attributes and XP;
//! `edge` checks what perks and traits add to a fight and to healing; `real_data` checks the formulas and constants against
//! `data/engine/formulas.json` and `data/engine/levels.json`.

mod edge;
mod progression;
mod properties;
mod real_data;
mod sheet;
