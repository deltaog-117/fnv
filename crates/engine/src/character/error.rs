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

/// A saved character that cannot be real. Nothing is built when one of these is
/// returned.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RestoreError {
    /// The attributes add up to more than a character can have.
    #[error("the attributes add up to {got}, more than the {max} a character can have")]
    SpecialTotal {
        /// The most a character can have.
        max: u32,
        /// The sum in the save.
        got: u32,
    },
    /// More skills are tagged than a character may tag.
    #[error("at most {max} tagged skills, not {got}")]
    TagCount {
        /// The most allowed.
        max: usize,
        /// How many the save has.
        got: usize,
    },
    /// A skill is tagged twice.
    #[error("{} is tagged twice", .0.name())]
    DuplicateTag(Skill),
    /// A skill has its points listed twice.
    #[error("{} is listed twice", .0.name())]
    DuplicateSkill(Skill),
    /// More traits than a character may have.
    #[error("at most {max} traits, not {got}")]
    TooManyTraits {
        /// The most allowed.
        max: usize,
        /// How many the save has.
        got: usize,
    },
    /// A trait is listed twice.
    #[error("{0} is listed twice")]
    DuplicateTrait(TraitId),
    /// The trait is not one the game offers.
    #[error("{0} is not a trait of this game")]
    UnknownTrait(TraitId),
    /// The perk is not one the game offers.
    #[error("{0} is not a perk of this game")]
    UnknownPerk(PerkId),
    /// A perk is listed twice.
    #[error("{0} is listed twice")]
    DuplicatePerk(PerkId),
    /// A perk is taken more times than it has ranks, or zero times.
    #[error("{perk} cannot be at rank {rank}; its highest rank is {max}")]
    PerkRank {
        /// The perk.
        perk: PerkId,
        /// The rank in the save.
        rank: u8,
        /// The highest rank.
        max: u8,
    },
    /// The perks taken and waiting do not match the levels reached.
    #[error("the levels reached give {expected} perks, the save accounts for {got}")]
    PerkPicks {
        /// Perks the levels reached give.
        expected: u32,
        /// Perks taken plus perks waiting in the save.
        got: u32,
    },
    /// A skill has more points spent than it could take.
    #[error("{} has more points spent than it could take", .0.name())]
    SkillOverMax(Skill),
}
