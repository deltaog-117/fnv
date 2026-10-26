//! Why a character cannot be made, or cannot grow in the way asked.

use thiserror::Error;

use crate::shared::{PerkId, Skill, Special, TraitId};

/// What a perk needs that the character does not have yet.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Unmet {
    /// The character is not high enough a level.
    #[error("level {need}")]
    Level {
        /// The level needed.
        need: u32,
    },
    /// An attribute is too low.
    #[error("{} {need}", stat.name())]
    Special {
        /// The attribute.
        stat: Special,
        /// The value needed.
        need: u8,
    },
    /// A skill is too low.
    #[error("{} {need}", skill.name())]
    Skill {
        /// The skill.
        skill: Skill,
        /// The value needed.
        need: u8,
    },
    /// A requirement the game cannot check, in the wiki's words.
    #[error("{0}")]
    Other(String),
}

/// A refusal to grow the character.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProgressError {
    /// No whole skill point is banked.
    #[error("no skill points to spend")]
    NoSkillPoints,
    /// The skill cannot go higher.
    #[error("{} is already at its maximum", .0.name())]
    SkillAtMaximum(Skill),
    /// No perk is waiting to be chosen.
    #[error("no perk to choose yet")]
    NoPerkPick,
    /// The perk is not one the game offers.
    #[error("{0} is not a perk of this game")]
    UnknownPerk(PerkId),
    /// Every rank of the perk is already taken.
    #[error("{0} is already at its highest rank")]
    PerkMaxed(PerkId),
    /// The character does not meet a requirement of the perk.
    #[error("{perk} needs {unmet}")]
    PerkLocked {
        /// The perk.
        perk: PerkId,
        /// The first requirement that is not met.
        unmet: Unmet,
    },
}

/// A refusal to make the character.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CreationError {
    /// The attributes do not add up to the points a new character has.
    #[error("the attributes must add up to {expected}, not {got}")]
    SpecialTotal {
        /// The points a new character has.
        expected: u32,
        /// The sum given.
        got: u32,
    },
    /// The wrong number of skills was tagged.
    #[error("tag exactly {expected} skills, not {got}")]
    TagCount {
        /// How many to tag.
        expected: usize,
        /// How many were given.
        got: usize,
    },
    /// A skill was tagged twice.
    #[error("{} is tagged twice", .0.name())]
    DuplicateTag(Skill),
    /// More traits than a new character may have.
    #[error("at most {max} traits, not {got}")]
    TooManyTraits {
        /// The most allowed.
        max: usize,
        /// How many were given.
        got: usize,
    },
    /// A trait was chosen twice.
    #[error("{0} is chosen twice")]
    DuplicateTrait(TraitId),
    /// The trait is not one the game offers.
    #[error("{0} is not a trait of this game")]
    UnknownTrait(TraitId),
}
