//! Where chance comes from: the hit model and the dice.

use super::stats::Percent;

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

/// A source of percentile rolls, passed to each attack so a fight is deterministic
/// for a given sequence of rolls.
pub trait Dice {
    /// The next roll, from 1 to 100. A roll outside that range is clamped into it.
    fn percent(&mut self) -> u8;
}
