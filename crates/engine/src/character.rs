//! The player character: S.P.E.C.I.A.L., skills, leveling, perks and traits.
//!
//! The part everything else reads: the seven attributes, the XP earned, and the
//! numbers derived from them (level, hit points, action points, carry weight and
//! skill levels). [`Character`] implements the shared `CharacterView` and
//! `CharacterSink`, so conditions and effects run against it unchanged.
//!
//! The character owns these laws and nothing else:
//!
//! - the level is never stored: it is the highest level whose XP total has been
//!   reached, so XP and level cannot disagree;
//! - XP only grows, and past the level cap it keeps counting without raising the
//!   level;
//! - every level pays out once: skill points at each level-up (`10 + 0.5 *
//!   intelligence`, half points carried over) and a perk to choose every second
//!   level, and nothing is spent that was not paid out;
//! - a skill is its starting value plus the points spent, 15 if tagged and what
//!   perks and traits add, always between 0 and 100;
//! - a perk is taken only when a pick is waiting and its requirements hold;
//!   what a perk or trait changes comes from its `modifiers` in the data, summed
//!   once when it is taken;
//! - every derived number comes from `data/engine/formulas.json`, and a real-data
//!   test fails if the formulas written here drift from the file.
//!
//! [`PerkBook`] says which perks and traits exist and who may take them. Only the
//! perks and traits with `modifiers` change a number; the rest can be taken and
//! recorded, and change nothing yet.

mod bonus;
mod book;
mod error;
mod rules;
mod sheet;

pub use book::PerkBook;
pub use error::{CreationError, ProgressError, Unmet};
pub use rules::{
    BASE_SKILL_HALVES, CharacterRules, Level, MAX_TRAITS, START_SPECIAL_POINTS, TAG_BONUS,
    TAG_COUNT,
};
pub use sheet::{Character, SpecialSet};
