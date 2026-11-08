//! What the mods on one weapon change, added up.

use std::num::NonZeroU32;

use crate::shared::ModEffect;

/// The sum of what a weapon's fitted mods change. The default changes nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Fitting {
    damage: f64,
    condition_hp: f64,
    condition_percent: f64,
    clip: u32,
    spread: f64,
}

impl Fitting {
    /// Adds one mod's effects to this fitting.
    pub(super) fn add(&mut self, effects: &[ModEffect]) {
        for effect in effects {
            match *effect {
                ModEffect::Damage { amount } => self.damage += amount,
                ModEffect::ConditionHp { amount } => self.condition_hp += amount,
                ModEffect::ConditionPercent { amount } => self.condition_percent += amount,
                ModEffect::ClipSize { amount } => self.clip = self.clip.saturating_add(amount),
                ModEffect::Spread { amount } => self.spread += amount,
            }
        }
    }

    /// Whether the fitting changes nothing.
    #[must_use]
    pub fn is_plain(&self) -> bool {
        *self == Self::default()
    }

    /// Damage per shot added; never negative.
    #[must_use]
    pub fn damage(&self) -> f64 {
        self.damage
    }

    /// Rounds added to the magazine.
    #[must_use]
    pub fn clip(&self) -> u32 {
        self.clip
    }

    /// The change to the weapon's spread, in points; never positive.
    #[must_use]
    pub fn spread(&self) -> f64 {
        self.spread
    }

    /// The condition points a weapon has when new, with these mods on: the points
    /// raised by the percent, plus the flat points, rounded to the nearest point.
    /// Never below `base`, because no effect takes condition away.
    #[must_use]
    pub fn condition_max(&self, base: NonZeroU32) -> NonZeroU32 {
        let raised =
            f64::from(base.get()) * (1.0 + self.condition_percent / 100.0) + self.condition_hp;
        // Both parts are never negative, so the rounded total is at least `base`; the
        // clamp keeps that true even for a number too large to fit.
        let points = raised
            .round()
            .clamp(f64::from(base.get()), f64::from(u32::MAX));
        // A float in the range [1, u32::MAX] always converts.
        NonZeroU32::new(points as u32).unwrap_or(base)
    }
}
