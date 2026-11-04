//! The damage pipeline, from `data/rules/formulas.json`: `weapon_damage`, then
//! `damage_after_dr`, then `damage_after_dt`.

use super::fight::AttackProfile;
use super::stats::{Points, Power, StatError};

/// What an attacker's training adds against one kind of foe, in percent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Against {
    /// Added to the weapon's damage.
    pub damage_percent: i32,
    /// Added to the damage a critical hit brings.
    pub crit_damage_percent: i32,
}

impl Against {
    /// No difference.
    pub const NONE: Self = Self {
        damage_percent: 0,
        crit_damage_percent: 0,
    };

    /// The attack with this difference built in. A penalty never takes damage below
    /// zero.
    #[must_use]
    pub fn applied(self, profile: &AttackProfile) -> AttackProfile {
        let scale = |points: Points, percent: i32| {
            Points::try_new(points.get() * (1.0 + f64::from(percent) / 100.0).max(0.0))
                .unwrap_or(points)
        };
        AttackProfile {
            base_damage: scale(profile.base_damage, self.damage_percent),
            crit_damage: scale(profile.crit_damage, self.crit_damage_percent),
            ..*profile
        }
    }
}

/// The constants of the damage steps.
///
/// They mirror `constants.dr_cap` and `constants.min_damage_fraction` in
/// `formulas.json`; a real-data test fails if [`Rules::standard`] drifts from the file.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rules {
    dr_cap: f64,
    min_damage_fraction: f64,
}

impl Rules {
    /// Builds the rules, rejecting a cap outside 0 to 100 and a floor outside (0, 1].
    ///
    /// # Errors
    ///
    /// Returns [`StatError::OutOfRange`] when a constant is outside its range or NaN.
    pub fn new(dr_cap: f64, min_damage_fraction: f64) -> Result<Self, StatError> {
        if !(0.0..=100.0).contains(&dr_cap) {
            return Err(StatError::OutOfRange {
                what: "resistance cap",
                min: 0.0,
                max: 100.0,
                got: dr_cap,
            });
        }
        if !(min_damage_fraction > 0.0 && min_damage_fraction <= 1.0) {
            return Err(StatError::OutOfRange {
                what: "minimum damage fraction",
                min: 0.0,
                max: 1.0,
                got: min_damage_fraction,
            });
        }
        Ok(Self {
            dr_cap,
            min_damage_fraction,
        })
    }

    /// The base game's constants: resistance caps at 85 and at least 20% always
    /// gets through.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            dr_cap: 85.0,
            min_damage_fraction: 0.2,
        }
    }

    /// The highest damage resistance that counts, in percent.
    #[must_use]
    pub fn dr_cap(&self) -> f64 {
        self.dr_cap
    }

    /// The share of the incoming damage that always gets through.
    #[must_use]
    pub fn min_damage_fraction(&self) -> f64 {
        self.min_damage_fraction
    }
}

/// Damage before armor: `base * skill * condition * power + bonus`.
///
/// A critical hit adds the weapon's critical damage to the base. That placement is
/// a stand-in: the wiki does not say where the critical bonus enters the formula.
#[must_use]
pub fn damage_before_armor(profile: &AttackProfile, power: Power, critical: bool) -> f64 {
    let base = profile.base_damage.get()
        + if critical {
            profile.crit_damage.get()
        } else {
            0.0
        };
    base * profile.skill.get() * profile.condition.get() * power.factor() + profile.bonus.get()
}

/// Damage that gets past armor: resistance first (capped), then the threshold,
/// with at least [`Rules::min_damage_fraction`] of the incoming damage always left.
#[must_use]
pub fn damage_after_armor(rules: &Rules, damage: f64, dt: Points, dr: Points) -> f64 {
    let after_dr = damage * (100.0 - dr.get().min(rules.dr_cap)) / 100.0;
    (after_dr - dt.get()).max(damage * rules.min_damage_fraction)
}
