//! The player character: S.P.E.C.I.A.L., skills, leveling, perks and traits.
//!
//! So far this is the part everything else reads: the seven attributes, the XP
//! earned, and the numbers derived from them (level, hit points, action points,
//! carry weight and skill levels). [`Character`] implements the shared
//! `CharacterView` and `CharacterSink`, so conditions and effects run against it
//! unchanged.
//!
//! The character owns these laws and nothing else:
//!
//! - the level is never stored: it is the highest level whose XP total has been
//!   reached, so XP and level cannot disagree;
//! - XP only grows, and past the level cap it keeps counting without raising the
//!   level;
//! - every derived number comes from `data/engine/formulas.json`, and a real-data
//!   test fails if the formulas written here drift from the file.
//!
//! Skill points, tagged skills, perks, traits and character creation are not
//! built: a skill is its starting value from S.P.E.C.I.A.L. alone.

mod rules;
mod sheet;

pub use rules::{CharacterRules, Level};
pub use sheet::{Character, SpecialSet};
