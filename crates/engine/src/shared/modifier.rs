//! The `Modifier` vocabulary: what a perk or a trait changes about the character.
//!
//! A modifier is data, so a perk's effect is written in `data/perks/modifiers.json`
//! and not in code. It mirrors `$defs/modifier` in `schemas/common.schema.json`.
//! Amounts are per rank: a perk taken twice adds its modifiers twice.

use serde::{Deserialize, Serialize};

use super::vocabulary::Skill;

/// One change a perk or trait makes to the numbers the character derives.
///
/// The integer widths are the allowed ranges, so an amount that is out of range
/// does not parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Modifier {
    /// Skill points gained at every level-up.
    SkillPointsPerLevel {
        /// Whole points.
        amount: u8,
    },
    /// Experience earned, as a percentage added to or taken from every award.
    XpPercent {
        /// Percent points; negative for a penalty.
        percent: i8,
    },
    /// Pounds that can be carried.
    CarryWeight {
        /// Pounds.
        pounds: u16,
    },
    /// Maximum hit points.
    MaxHp {
        /// Hit points.
        amount: u16,
    },
    /// Action points, on top of the base cap.
    ActionPoints {
        /// Action points.
        amount: u8,
    },
    /// Every skill.
    AllSkills {
        /// Skill points; negative for a penalty.
        amount: i8,
    },
    /// One skill.
    Skill {
        /// Which skill.
        skill: Skill,
        /// Skill points; negative for a penalty.
        amount: i8,
    },
}

impl Modifier {
    /// Every `type` value a modifier can have, in schema order.
    pub const ALL_TAGS: [&'static str; 7] = [
        "skill_points_per_level",
        "xp_percent",
        "carry_weight",
        "max_hp",
        "action_points",
        "all_skills",
        "skill",
    ];

    /// The `type` value of this modifier. The match is exhaustive, so adding a
    /// variant without naming it here does not compile.
    #[must_use]
    pub fn tag(&self) -> &'static str {
        match self {
            Self::SkillPointsPerLevel { .. } => "skill_points_per_level",
            Self::XpPercent { .. } => "xp_percent",
            Self::CarryWeight { .. } => "carry_weight",
            Self::MaxHp { .. } => "max_hp",
            Self::ActionPoints { .. } => "action_points",
            Self::AllSkills { .. } => "all_skills",
            Self::Skill { .. } => "skill",
        }
    }
}
