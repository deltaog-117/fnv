//! The `Modifier` vocabulary: what a perk or a trait changes about the character.
//!
//! A modifier is data, so a perk's effect is written in `data/perks/modifiers.json`
//! and not in code. It mirrors `$defs/modifier` in `schemas/common.schema.json`.
//! Amounts are per rank: a perk taken twice adds its modifiers twice.
//!
//! Some modifiers are conditional: they name the skill of the weapon in hand, the
//! kind of foe being fought or the item being used, and apply only then. Leaving
//! the condition out means always.

use serde::{Deserialize, Serialize};

use super::ids::ItemId;
use super::vocabulary::{FoeKind, Skill};

/// One change a perk or trait makes to the numbers the character derives.
///
/// The integer widths are the allowed ranges, so an amount that is out of range
/// does not parse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Damage threshold, on top of the armor's. Never takes the total below zero.
    DamageThreshold {
        /// Points; negative for a penalty.
        amount: i8,
    },
    /// Damage resistance, in percent points on top of the armor's. Never takes the
    /// total below zero.
    DamageResistance {
        /// Percent points; negative for a penalty.
        percent: i8,
    },
    /// Points of a target's damage threshold that this character's attacks ignore.
    IgnoreThreshold {
        /// Points.
        amount: u8,
    },
    /// Weapon damage.
    DamagePercent {
        /// Percent added to the damage; negative for a penalty.
        percent: i8,
        /// Only with a weapon governed by this skill.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        skill: Option<Skill>,
        /// Only against this kind of foe.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        foe: Option<FoeKind>,
    },
    /// The chance that a hit is critical.
    CritChance {
        /// Percent points added; negative for a penalty.
        percent: i8,
        /// Only with a weapon governed by this skill.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        skill: Option<Skill>,
    },
    /// The damage a critical hit adds.
    CritDamagePercent {
        /// Percent added to the critical bonus; negative for a penalty.
        percent: i8,
        /// Only with a weapon governed by this skill.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        skill: Option<Skill>,
        /// Only against this kind of foe.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        foe: Option<FoeKind>,
    },
    /// What an attack costs in action points.
    ApCostPercent {
        /// Percent added to the cost; negative makes attacks cheaper.
        percent: i8,
        /// Only with a weapon governed by this skill.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        skill: Option<Skill>,
    },
    /// What a reload costs in action points.
    ReloadApPercent {
        /// Percent added to the cost; negative makes reloads cheaper.
        percent: i8,
    },
    /// How widely a weapon scatters its shots.
    SpreadPercent {
        /// Percent added to the spread; negative makes shots steadier.
        percent: i8,
        /// Only with a weapon governed by this skill.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        skill: Option<Skill>,
    },
    /// How fast the weapon in hand wears.
    WeaponWearPercent {
        /// Percent added to the wear; negative slows it.
        percent: i8,
    },
    /// Hit points restored by items.
    HealPercent {
        /// Percent added to what an item restores; negative for a penalty.
        percent: i8,
        /// Only for this item.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        item: Option<ItemId>,
    },
}

impl Modifier {
    /// Every `type` value a modifier can have, in schema order.
    pub const ALL_TAGS: [&'static str; 18] = [
        "skill_points_per_level",
        "xp_percent",
        "carry_weight",
        "max_hp",
        "action_points",
        "all_skills",
        "skill",
        "damage_threshold",
        "damage_resistance",
        "ignore_threshold",
        "damage_percent",
        "crit_chance",
        "crit_damage_percent",
        "ap_cost_percent",
        "reload_ap_percent",
        "spread_percent",
        "weapon_wear_percent",
        "heal_percent",
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
            Self::DamageThreshold { .. } => "damage_threshold",
            Self::DamageResistance { .. } => "damage_resistance",
            Self::IgnoreThreshold { .. } => "ignore_threshold",
            Self::DamagePercent { .. } => "damage_percent",
            Self::CritChance { .. } => "crit_chance",
            Self::CritDamagePercent { .. } => "crit_damage_percent",
            Self::ApCostPercent { .. } => "ap_cost_percent",
            Self::ReloadApPercent { .. } => "reload_ap_percent",
            Self::SpreadPercent { .. } => "spread_percent",
            Self::WeaponWearPercent { .. } => "weapon_wear_percent",
            Self::HealPercent { .. } => "heal_percent",
        }
    }
}
