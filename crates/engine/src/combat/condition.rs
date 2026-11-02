//! How worn a weapon is, and what that does to its damage.

use std::num::NonZeroU32;

use super::stats::{Multiplier, Percent};

/// A weapon's condition: the points it has lost out of the points it had new.
///
/// The damage formula takes a multiplier from half to full effect, so a ruined
/// weapon still hurts; the wiki calls this a general guide, not exact. The points
/// lost can never pass the maximum, so condition cannot go below zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Condition {
    lost: u32,
    max: NonZeroU32,
}

impl Condition {
    /// A weapon that has lost `lost` of its `max` points; more than all of them
    /// counts as all of them.
    #[must_use]
    pub fn new(max: NonZeroU32, lost: u32) -> Self {
        Self {
            lost: lost.min(max.get()),
            max,
        }
    }

    /// Points lost so far.
    #[must_use]
    pub fn lost(self) -> u32 {
        self.lost
    }

    /// The same weapon after losing `points` more.
    #[must_use]
    pub fn worn(self, points: u32) -> Self {
        Self::new(self.max, self.lost.saturating_add(points))
    }

    /// What is left, from 0 to 100 percent, rounded down so a scratched weapon never
    /// reads as new.
    #[must_use]
    pub fn percent(self) -> Percent {
        let left = u64::from(self.max.get() - self.lost) * 100 / u64::from(self.max.get());
        // At most 100 by construction, so the conversion cannot fail.
        Percent::try_new(u32::try_from(left).unwrap_or(100)).unwrap_or(Percent::ALWAYS)
    }

    /// The damage multiplier: full when new, half when nothing is left.
    #[must_use]
    pub fn multiplier(self) -> Multiplier {
        let left = f64::from(self.max.get() - self.lost) / f64::from(self.max.get());
        // 0.5 to 1 by construction, so the conversion cannot fail.
        Multiplier::try_new(Multiplier::MIN + (Multiplier::MAX - Multiplier::MIN) * left)
            .unwrap_or(Multiplier::FULL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn max() -> impl Strategy<Value = NonZeroU32> {
        (1..=5_000u32).prop_map(|n| NonZeroU32::new(n).unwrap_or(NonZeroU32::MIN))
    }

    #[test]
    fn a_new_weapon_is_whole_and_does_full_damage() {
        let new = Condition::new(NonZeroU32::MIN.saturating_add(199), 0);
        assert_eq!(new.percent(), Percent::ALWAYS);
        assert_eq!(new.multiplier(), Multiplier::FULL);
    }

    #[test]
    fn a_ruined_weapon_still_does_half_damage() {
        let ruined = Condition::new(NonZeroU32::MIN.saturating_add(99), u32::MAX);
        assert_eq!(ruined.percent(), Percent::NEVER);
        assert_eq!(ruined.multiplier().get(), Multiplier::MIN);
    }

    proptest! {
        #[test]
        fn losing_points_never_helps(max in max(), lost in 0..10_000u32, more in 0..10_000u32) {
            let before = Condition::new(max, lost);
            let after = before.worn(more);
            prop_assert!(after.lost() >= before.lost());
            prop_assert!(after.lost() <= max.get());
            prop_assert!(after.multiplier() <= before.multiplier());
            prop_assert!(after.percent() <= before.percent());
        }

        #[test]
        fn the_multiplier_stays_between_half_and_full(max in max(), lost in any::<u32>()) {
            let m = Condition::new(max, lost).multiplier().get();
            prop_assert!((Multiplier::MIN..=Multiplier::MAX).contains(&m));
        }

        #[test]
        fn wearing_in_steps_equals_wearing_at_once(max in max(), a in 0..3_000u32, b in 0..3_000u32) {
            let start = Condition::new(max, 0);
            prop_assert_eq!(start.worn(a).worn(b), start.worn(a + b));
        }
    }
}
