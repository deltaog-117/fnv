//! Where chance comes from: the hit model and the dice.

use super::fight::Combatant;
use super::stats::{Percent, Spread};

/// Decides how likely an attack is to hit.
///
/// The wiki gives no hit-chance formula, so the fight asks a model instead of
/// owning one; the real model can replace [`FlatHitChance`] without touching the
/// damage code.
pub trait HitModel<C> {
    /// The chance that `attacker`'s attack on `target` hits.
    fn chance(&self, attacker: &C, target: &C) -> Percent;
}

/// A stand-in model: every attack has the same chance, whoever fights.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlatHitChance(pub Percent);

impl<C> HitModel<C> for FlatHitChance {
    fn chance(&self, _attacker: &C, _target: &C) -> Percent {
        self.0
    }
}

/// How well a fighter can place a shot with the weapon in hand.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aim {
    /// The fighter's skill with the weapon, from 0 to 100.
    pub skill: Percent,
    /// How widely the weapon scatters its shots.
    pub spread: Spread,
}

impl Aim {
    /// No training and a steady weapon: what a fighter that does not say gets.
    pub const UNTRAINED: Self = Self {
        skill: Percent::NEVER,
        spread: Spread::NONE,
    };
}

/// Flat part of the aimed hit chance, in percent.
const BASE_CHANCE: f64 = 40.0;
/// Percent of hit chance each point of skill adds.
const SKILL_WEIGHT: f64 = 0.5;
/// Percent of hit chance each point of spread takes away.
const SPREAD_PENALTY: f64 = 3.0;
/// Lowest and highest chance an attack can have: no shot is hopeless or certain.
const FLOOR_CHANCE: f64 = 5.0;
const CEILING_CHANCE: f64 = 95.0;

/// Hit chance from the attacker's skill and the weapon's spread.
///
/// The constants are a labelled stand-in: the wiki gives no formula, so they are
/// set so a mid-skill shooter with a steady weapon lands about two in three, as the
/// flat model did. Skill raises the chance and spread lowers it, which is the part
/// the data supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AimedHitChance;

impl<C: Combatant> HitModel<C> for AimedHitChance {
    fn chance(&self, attacker: &C, _target: &C) -> Percent {
        let aim = attacker.aim();
        let raw = BASE_CHANCE + f64::from(aim.skill.get()) * SKILL_WEIGHT
            - aim.spread.get() * SPREAD_PENALTY;
        // Clamped into 5 to 95, so the conversion cannot fail.
        Percent::try_new(raw.round().clamp(FLOOR_CHANCE, CEILING_CHANCE) as u32)
            .unwrap_or(Percent::NEVER)
    }
}

/// A source of percentile rolls, passed to each attack so a fight is deterministic
/// for a given sequence of rolls.
pub trait Dice {
    /// The next roll, from 1 to 100. A roll outside that range is clamped into it.
    fn percent(&mut self) -> u8;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{AttackProfile, Magazine, Multiplier, Points};
    use proptest::prelude::*;
    use std::num::NonZeroU32;

    struct Shooter(Aim);

    impl Combatant for Shooter {
        fn max_hp(&self) -> NonZeroU32 {
            NonZeroU32::MIN
        }
        fn max_ap(&self) -> u32 {
            50
        }
        fn dt(&self) -> Points {
            Points::ZERO
        }
        fn dr(&self) -> Points {
            Points::ZERO
        }
        fn crit_chance(&self) -> Percent {
            Percent::NEVER
        }
        fn attack(&self) -> AttackProfile {
            AttackProfile {
                ap_cost: NonZeroU32::MIN,
                base_damage: Points::ZERO,
                crit_damage: Points::ZERO,
                skill: Multiplier::FULL,
                condition: Multiplier::FULL,
                bonus: Points::ZERO,
            }
        }
        fn magazine(&self) -> Option<Magazine> {
            None
        }
        fn aim(&self) -> Aim {
            self.0
        }
    }

    fn chance(skill: u32, spread: f64) -> u8 {
        let aim = Aim {
            skill: Percent::try_new(skill).expect("a valid percent"),
            spread: Spread::try_new(spread).expect("a valid spread"),
        };
        AimedHitChance
            .chance(&Shooter(aim), &Shooter(Aim::UNTRAINED))
            .get()
    }

    #[test]
    fn a_mid_skill_shooter_with_a_steady_weapon_lands_about_two_in_three() {
        assert_eq!(chance(50, 0.5), 64);
    }

    #[test]
    fn nobody_is_hopeless_or_certain() {
        assert_eq!(chance(0, 20.0), 5);
        assert_eq!(chance(100, 0.0), 90);
    }

    proptest! {
        #[test]
        fn skill_never_hurts_and_spread_never_helps(
            skill in 0..=99u32,
            spread in 0.0..20.0f64,
            extra in 0.0..5.0f64,
        ) {
            prop_assert!(chance(skill + 1, spread) >= chance(skill, spread));
            prop_assert!(chance(skill, spread + extra) <= chance(skill, spread));
        }

        #[test]
        fn the_chance_is_always_inside_the_floor_and_ceiling(skill in 0..=100u32, spread in 0.0..1_000.0f64) {
            let c = chance(skill, spread);
            prop_assert!((5..=95).contains(&c));
        }
    }
}
