//! Timed boosts: the temporary changes a chem, a drink or a meal makes.
//!
//! [`Boost`] is what a record says an item does and for how long; [`BoostTally`] is
//! the sum of the boosts in force, which the character reads. Both are plain data
//! with no clock: the feature that owns time decides when a boost starts and ends.

use std::num::{NonZeroI16, NonZeroU32};

use serde::{Deserialize, Serialize};
use strum::EnumCount;

use super::vocabulary::{Skill, Special};

/// A number that is neither an attribute nor a skill, which a boost can change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DerivedStat {
    /// Maximum hit points.
    MaxHp,
    /// Action points, past the usual cap.
    ActionPoints,
    /// Pounds that can be carried.
    CarryWeight,
    /// Damage resistance, in percent points before the cap.
    DamageResistance,
    /// Radiation resistance, in percent points before the cap.
    RadiationResistance,
}

/// What a boost changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BoostStat {
    /// One S.P.E.C.I.A.L. attribute.
    Special(Special),
    /// One skill.
    Skill(Skill),
    /// A number the character derives.
    Derived(DerivedStat),
}

impl BoostStat {
    /// The name the player sees.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Special(stat) => stat.name(),
            Self::Skill(skill) => skill.name(),
            Self::Derived(DerivedStat::MaxHp) => "Maximum hit points",
            Self::Derived(DerivedStat::ActionPoints) => "Action points",
            Self::Derived(DerivedStat::CarryWeight) => "Carry weight",
            Self::Derived(DerivedStat::DamageResistance) => "Damage resistance",
            Self::Derived(DerivedStat::RadiationResistance) => "Radiation resistance",
        }
    }
}

/// One change an item makes for a while.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Boost {
    /// What it changes.
    pub stat: BoostStat,
    /// By how much; negative for a penalty, and never zero.
    pub amount: NonZeroI16,
    /// For how many minutes. The clock counts whole minutes, so an effect the wiki
    /// gives in seconds is rounded up to one.
    pub minutes: NonZeroU32,
}

/// The sum of the boosts in force, by stat. Sums saturate, so no pile of doses can
/// wrap a number.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BoostTally {
    special: [i32; Special::COUNT],
    skills: [i32; Skill::COUNT],
    max_hp: i32,
    action_points: i32,
    carry_weight: i32,
    damage_resistance: i32,
    radiation_resistance: i32,
}

impl BoostTally {
    /// Adds `amount` to a stat.
    pub fn add(&mut self, stat: BoostStat, amount: i16) {
        let amount = i32::from(amount);
        let slot = match stat {
            BoostStat::Special(stat) => &mut self.special[stat as usize],
            BoostStat::Skill(skill) => &mut self.skills[skill as usize],
            BoostStat::Derived(DerivedStat::MaxHp) => &mut self.max_hp,
            BoostStat::Derived(DerivedStat::ActionPoints) => &mut self.action_points,
            BoostStat::Derived(DerivedStat::CarryWeight) => &mut self.carry_weight,
            BoostStat::Derived(DerivedStat::DamageResistance) => &mut self.damage_resistance,
            BoostStat::Derived(DerivedStat::RadiationResistance) => &mut self.radiation_resistance,
        };
        *slot = slot.saturating_add(amount);
    }

    /// Adds everything `other` changes, stat by stat, so changes from different
    /// sources (a chem, radiation sickness, withdrawal) read as one.
    pub fn merge(&mut self, other: &Self) {
        let add = |mine: &mut i32, theirs: i32| *mine = mine.saturating_add(theirs);
        self.special
            .iter_mut()
            .zip(other.special)
            .for_each(|(mine, theirs)| add(mine, theirs));
        self.skills
            .iter_mut()
            .zip(other.skills)
            .for_each(|(mine, theirs)| add(mine, theirs));
        add(&mut self.max_hp, other.max_hp);
        add(&mut self.action_points, other.action_points);
        add(&mut self.carry_weight, other.carry_weight);
        add(&mut self.damage_resistance, other.damage_resistance);
        add(&mut self.radiation_resistance, other.radiation_resistance);
    }

    /// The change to an attribute.
    #[must_use]
    pub fn special(&self, stat: Special) -> i32 {
        self.special[stat as usize]
    }

    /// The change to a skill.
    #[must_use]
    pub fn skill(&self, skill: Skill) -> i32 {
        self.skills[skill as usize]
    }

    /// The change to maximum hit points.
    #[must_use]
    pub fn max_hp(&self) -> i32 {
        self.max_hp
    }

    /// The change to action points.
    #[must_use]
    pub fn action_points(&self) -> i32 {
        self.action_points
    }

    /// The change to carry weight, in pounds.
    #[must_use]
    pub fn carry_weight(&self) -> i32 {
        self.carry_weight
    }

    /// The change to damage resistance, in percent points.
    #[must_use]
    pub fn damage_resistance(&self) -> i32 {
        self.damage_resistance
    }

    /// The change to radiation resistance, in percent points.
    #[must_use]
    pub fn radiation_resistance(&self) -> i32 {
        self.radiation_resistance
    }

    /// Whether nothing is changed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> Boost {
        serde_json::from_str(json).expect("the literal is a valid boost")
    }

    #[test]
    fn a_boost_names_an_attribute_a_skill_or_a_derived_number() {
        assert_eq!(
            parse(r#"{"stat": "strength", "amount": 2, "minutes": 4}"#).stat,
            BoostStat::Special(Special::Strength)
        );
        assert_eq!(
            parse(r#"{"stat": "melee_weapons", "amount": -3, "minutes": 1}"#).stat,
            BoostStat::Skill(Skill::MeleeWeapons)
        );
        assert_eq!(
            parse(r#"{"stat": "max_hp", "amount": 60, "minutes": 4}"#).stat,
            BoostStat::Derived(DerivedStat::MaxHp)
        );
    }

    #[test]
    fn a_boost_of_nothing_or_for_no_time_does_not_parse() {
        for bad in [
            r#"{"stat": "luck", "amount": 0, "minutes": 4}"#,
            r#"{"stat": "luck", "amount": 1, "minutes": 0}"#,
            r#"{"stat": "wisdom", "amount": 1, "minutes": 4}"#,
            r#"{"stat": "luck", "amount": 1, "minutes": 4, "extra": true}"#,
        ] {
            assert!(serde_json::from_str::<Boost>(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_tally_adds_and_each_stat_has_its_own_slot() {
        let mut tally = BoostTally::default();
        assert!(tally.is_empty());
        tally.add(BoostStat::Special(Special::Luck), 2);
        tally.add(BoostStat::Special(Special::Luck), -1);
        tally.add(BoostStat::Skill(Skill::Guns), 5);
        tally.add(BoostStat::Derived(DerivedStat::MaxHp), 60);
        assert_eq!(tally.special(Special::Luck), 1);
        assert_eq!(tally.special(Special::Strength), 0);
        assert_eq!(tally.skill(Skill::Guns), 5);
        assert_eq!(tally.skill(Skill::Barter), 0);
        assert_eq!(tally.max_hp(), 60);
        assert!(!tally.is_empty());
    }

    #[test]
    fn merging_adds_stat_by_stat() {
        let (mut a, mut b) = (BoostTally::default(), BoostTally::default());
        a.add(BoostStat::Special(Special::Endurance), -1);
        b.add(BoostStat::Special(Special::Endurance), 3);
        b.add(BoostStat::Derived(DerivedStat::RadiationResistance), 25);
        a.merge(&b);
        assert_eq!(a.special(Special::Endurance), 2);
        assert_eq!(a.radiation_resistance(), 25);
        assert_eq!(a.special(Special::Luck), 0);
    }

    #[test]
    fn the_tally_saturates_instead_of_wrapping() {
        let mut tally = BoostTally::default();
        for _ in 0..=u16::MAX {
            tally.add(BoostStat::Derived(DerivedStat::MaxHp), i16::MAX);
        }
        assert!(tally.max_hp() > 0);
    }
}
