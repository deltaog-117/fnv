//! The `Modifier` vocabulary: what a perk or a trait changes about the character.
//!
//! A modifier is data, so a perk's effect is written in `data/perks/modifiers.json`
//! and not in code. It mirrors `$defs/modifier` in `schemas/common.schema.json`.
//! Amounts are per rank: a perk taken twice adds its modifiers twice.
//!
//! Some modifiers are conditional: they name the skill of the weapon in hand, the
//! kind of foe being fought or the item being used, and apply only then. Leaving
//! the condition out means always.

use std::num::NonZeroU16;

use serde::{Deserialize, Serialize};

use super::bounds::HealthPercent;
use super::condition::Condition;
use super::data::ArmorClass;
use super::error::ModelError;
use super::ids::ItemId;
use super::non_empty::NonEmpty;
use super::vocabulary::{FoeKind, Skill, Special};

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
    /// One S.P.E.C.I.A.L. attribute, kept inside the 1 to 10 scale.
    Special {
        /// Which attribute.
        stat: Special,
        /// Points; negative for a penalty.
        amount: i8,
    },
    /// Every S.P.E.C.I.A.L. attribute, each kept inside the 1 to 10 scale.
    AllSpecial {
        /// Points; negative for a penalty.
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
        /// Only while the body armor worn is of this class.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        armor: Option<ArmorClass>,
    },
    /// The chance that a hit on this character is critical, whoever strikes.
    CritChanceAgainst {
        /// Percent points added to the attacker's chance; negative makes this
        /// character harder to hurt critically, and -100 makes them immune.
        percent: i8,
        /// Only while the body armor worn is of this class.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        armor: Option<ArmorClass>,
    },
    /// A reward for a fight survived with little health: an attribute raised for a
    /// while once the fight is over.
    AfterFightBoost {
        /// The fight must end with less than this share of the hit points left.
        below_health: HealthPercent,
        /// The attribute raised.
        stat: Special,
        /// Points added; negative for a penalty.
        amount: i8,
        /// How long it lasts, in minutes.
        minutes: NonZeroU16,
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
    pub const ALL_TAGS: [&'static str; 22] = [
        "skill_points_per_level",
        "xp_percent",
        "carry_weight",
        "max_hp",
        "action_points",
        "all_skills",
        "special",
        "all_special",
        "skill",
        "damage_threshold",
        "damage_resistance",
        "ignore_threshold",
        "damage_percent",
        "crit_chance",
        "crit_chance_against",
        "after_fight_boost",
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
            Self::Special { .. } => "special",
            Self::AllSpecial { .. } => "all_special",
            Self::Skill { .. } => "skill",
            Self::DamageThreshold { .. } => "damage_threshold",
            Self::DamageResistance { .. } => "damage_resistance",
            Self::IgnoreThreshold { .. } => "ignore_threshold",
            Self::DamagePercent { .. } => "damage_percent",
            Self::CritChance { .. } => "crit_chance",
            Self::CritChanceAgainst { .. } => "crit_chance_against",
            Self::AfterFightBoost { .. } => "after_fight_boost",
            Self::CritDamagePercent { .. } => "crit_damage_percent",
            Self::ApCostPercent { .. } => "ap_cost_percent",
            Self::ReloadApPercent { .. } => "reload_ap_percent",
            Self::SpreadPercent { .. } => "spread_percent",
            Self::WeaponWearPercent { .. } => "weapon_wear_percent",
            Self::HealPercent { .. } => "heal_percent",
        }
    }
}

/// Modifiers that apply only while conditions about the character's situation hold.
///
/// A perk lists these beside its plain modifiers: Hot Blooded gives its damage and
/// its penalties only below half health, Early Bird its bonus only in the morning.
/// The conditions may ask about health and the time of day, joined with `all`,
/// `any` and `not`, and nothing else: anything that reads an attribute or a skill
/// could depend on the very modifiers it switches on, and a rule that can contradict
/// itself is refused when the data is read.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "ConditionalWire")]
pub struct ConditionalModifiers {
    when: NonEmpty<Condition>,
    modifiers: NonEmpty<Modifier>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConditionalWire {
    when: NonEmpty<Condition>,
    modifiers: NonEmpty<Modifier>,
}

impl ConditionalModifiers {
    /// Pairs the conditions that must all hold with what they switch on.
    ///
    /// # Errors
    ///
    /// [`ModelError::InvalidGate`] when a condition asks about something other than
    /// health or the time of day, or when the modifiers include one that could move
    /// the thing the conditions read: maximum hit points, or (under a health
    /// condition) any attribute that sets them.
    pub fn new(
        when: NonEmpty<Condition>,
        modifiers: NonEmpty<Modifier>,
    ) -> Result<Self, ModelError> {
        if !when.iter().all(reads_only_the_situation) {
            return Err(ModelError::InvalidGate(
                "conditions may only ask about health and the time of day",
            ));
        }
        let by_health = when.iter().any(reads_health);
        for modifier in modifiers.iter() {
            let moves_hit_points = match modifier {
                Modifier::MaxHp { .. } => true,
                Modifier::AllSpecial { .. } => by_health,
                Modifier::Special { stat, .. } => by_health && *stat == Special::Endurance,
                _ => false,
            };
            if moves_hit_points {
                return Err(ModelError::InvalidGate(
                    "modifiers may not change the maximum hit points a health condition reads",
                ));
            }
        }
        Ok(Self { when, modifiers })
    }

    /// The conditions, all of which must hold.
    #[must_use]
    pub fn when(&self) -> &[Condition] {
        self.when.as_slice()
    }

    /// What applies while they do.
    #[must_use]
    pub fn modifiers(&self) -> &[Modifier] {
        self.modifiers.as_slice()
    }
}

impl TryFrom<ConditionalWire> for ConditionalModifiers {
    type Error = ModelError;

    fn try_from(wire: ConditionalWire) -> Result<Self, Self::Error> {
        Self::new(wire.when, wire.modifiers)
    }
}

fn reads_only_the_situation(condition: &Condition) -> bool {
    match condition {
        Condition::Health { .. } | Condition::TimeOfDay(_) => true,
        Condition::All { of } | Condition::Any { of } => of.iter().all(reads_only_the_situation),
        Condition::Not { of } => reads_only_the_situation(of),
        _ => false,
    }
}

fn reads_health(condition: &Condition) -> bool {
    match condition {
        Condition::Health { .. } => true,
        Condition::All { of } | Condition::Any { of } => of.iter().any(reads_health),
        Condition::Not { of } => reads_health(of),
        _ => false,
    }
}
