//! The repair laws: pure arithmetic over a weapon's condition points.
//!
//! A weapon has `max` condition points when new and has lost `lost` of them; what
//! is left is `max - lost`. Every function takes plain points, so this feature
//! does not need the combat feature's condition type.

use std::num::NonZeroU32;

use crate::shared::SkillLevel;

/// The item a kit repair uses up.
pub const WEAPON_REPAIR_KIT: &str = "weapon_repair_kit";
/// What merging two copies adds on top of their combined condition, as a percent of
/// the weapon's new condition. A stand-in: the wiki gives no bonus.
pub const MERGE_BONUS_PERCENT: u32 = 10;
/// What a full repair costs at a vendor, as a percent of the weapon's value. A
/// stand-in: the wiki gives no repair prices.
pub const PRICE_SHARE_PERCENT: u32 = 50;

fn left(max: NonZeroU32, lost: u32) -> u32 {
    max.get().saturating_sub(lost)
}

/// The condition points a Repair skill can hold a weapon at: the skill's share of
/// `max`, rounded up so any skill above zero mends at least a little.
#[must_use]
pub fn ceiling(max: NonZeroU32, skill: SkillLevel) -> u32 {
    let share = u64::from(max.get()) * u64::from(skill.get());
    // Never above `max` since the skill is at most 100.
    u32::try_from(share.div_ceil(100)).map_or(max.get(), |points| points.min(max.get()))
}

/// The points lost after mending a weapon with a repair kit: what is left rises to
/// the skill's ceiling, and never falls.
#[must_use]
pub fn after_kit(max: NonZeroU32, lost: u32, skill: SkillLevel) -> u32 {
    let lost = lost.min(max.get());
    let now = left(max, lost);
    max.get() - now.max(ceiling(max, skill))
}

/// The points lost after merging two copies of one weapon: what is left of both,
/// plus a bonus, held to the skill's ceiling. The result is never worse than the
/// better of the two copies.
#[must_use]
pub fn after_merge(max: NonZeroU32, lost_a: u32, lost_b: u32, skill: SkillLevel) -> u32 {
    let (a, b) = (
        left(max, lost_a.min(max.get())),
        left(max, lost_b.min(max.get())),
    );
    let bonus = u64::from(max.get()) * u64::from(MERGE_BONUS_PERCENT) / 100;
    let combined = u64::from(a) + u64::from(b) + bonus;
    let capped = u32::try_from(combined).map_or(max.get(), |points| points.min(max.get()));
    let held = capped.min(ceiling(max, skill));
    max.get() - held.max(a.max(b))
}

/// The caps a vendor charges to take a weapon worth `value` caps from `from_lost`
/// points lost to `to_lost`. Nothing restored costs nothing; anything restored
/// costs at least one cap, rounded up so a repair is never free.
#[must_use]
pub fn vendor_price(value: f64, max: NonZeroU32, from_lost: u32, to_lost: u32) -> u32 {
    let restored = from_lost.min(max.get()).saturating_sub(to_lost);
    if restored == 0 {
        return 0;
    }
    let share = f64::from(PRICE_SHARE_PERCENT) / 100.0;
    let cost = value.max(0.0) * share * f64::from(restored) / f64::from(max.get());
    // `ceil` of a non-negative finite number below u32::MAX in practice; a value so
    // large it does not fit reads as the most a player could ever hold.
    let rounded = cost.ceil();
    if rounded >= f64::from(u32::MAX) {
        u32::MAX
    } else {
        // Truncation is exact: `rounded` is a whole number below `u32::MAX`.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let caps = rounded as u32;
        caps.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn max() -> impl Strategy<Value = NonZeroU32> {
        (1..=5_000u32).prop_map(|n| NonZeroU32::new(n).unwrap_or(NonZeroU32::MIN))
    }

    fn skill() -> impl Strategy<Value = SkillLevel> {
        (0..=100u32).prop_map(|n| SkillLevel::try_new(n).unwrap_or_default())
    }

    #[test]
    fn a_skill_of_one_hundred_mends_a_weapon_whole() {
        let max = NonZeroU32::new(200).expect("non-zero");
        let full = SkillLevel::try_new(100).expect("in range");
        assert_eq!(after_kit(max, 150, full), 0);
    }

    #[test]
    fn a_skill_of_zero_mends_nothing() {
        let max = NonZeroU32::new(200).expect("non-zero");
        assert_eq!(after_kit(max, 150, SkillLevel::default()), 150);
    }

    #[test]
    fn a_kit_at_repair_forty_lifts_a_fifth_worn_weapon_to_forty_percent() {
        let max = NonZeroU32::new(200).expect("non-zero");
        let forty = SkillLevel::try_new(40).expect("in range");
        // 20% left is 40 of 200 points; the ceiling is 80 points.
        assert_eq!(after_kit(max, 160, forty), 120);
    }

    #[test]
    fn two_half_worn_copies_merge_to_the_skill_ceiling() {
        let max = NonZeroU32::new(200).expect("non-zero");
        let sixty = SkillLevel::try_new(60).expect("in range");
        // 100 + 100 + 20 points would pass the ceiling of 120.
        assert_eq!(after_merge(max, 100, 100, sixty), 80);
    }

    #[test]
    fn a_repair_that_restores_nothing_is_free() {
        let max = NonZeroU32::new(200).expect("non-zero");
        assert_eq!(vendor_price(100.0, max, 50, 50), 0);
    }

    #[test]
    fn a_full_repair_costs_half_the_value() {
        let max = NonZeroU32::new(200).expect("non-zero");
        assert_eq!(vendor_price(100.0, max, 200, 0), 50);
    }

    proptest! {
        #[test]
        fn a_kit_never_makes_a_weapon_worse(max in max(), lost in 0..10_000u32, skill in skill()) {
            let after = after_kit(max, lost, skill);
            prop_assert!(after <= lost.min(max.get()));
        }

        #[test]
        fn a_higher_skill_never_gives_a_worse_kit_repair(
            max in max(), lost in 0..10_000u32, a in skill(), b in skill(),
        ) {
            let (low, high) = if a <= b { (a, b) } else { (b, a) };
            prop_assert!(after_kit(max, lost, high) <= after_kit(max, lost, low));
        }

        #[test]
        fn a_kit_repair_is_settled_after_one_use(max in max(), lost in 0..10_000u32, skill in skill()) {
            let once = after_kit(max, lost, skill);
            prop_assert_eq!(after_kit(max, once, skill), once);
        }

        #[test]
        fn a_kit_repair_stays_within_the_weapon(max in max(), lost in 0..10_000u32, skill in skill()) {
            prop_assert!(ceiling(max, skill) <= max.get());
            prop_assert!(after_kit(max, lost, skill) <= max.get());
        }

        #[test]
        fn a_merge_is_never_worse_than_the_better_copy(
            max in max(), a in 0..10_000u32, b in 0..10_000u32, skill in skill(),
        ) {
            let after = after_merge(max, a, b, skill);
            prop_assert!(after <= a.min(b).min(max.get()));
        }

        #[test]
        fn a_merge_stays_within_the_weapon(
            max in max(), a in 0..10_000u32, b in 0..10_000u32, skill in skill(),
        ) {
            prop_assert!(after_merge(max, a, b, skill) <= max.get());
        }

        #[test]
        fn a_larger_repair_never_costs_less(
            value in 0.0..50_000.0f64, max in max(), lost in 0..5_000u32, a in 0..5_000u32, b in 0..5_000u32,
        ) {
            let (small, large) = if a <= b { (a, b) } else { (b, a) };
            // Restoring more means ending with fewer points lost.
            let little = vendor_price(value, max, lost, lost.saturating_sub(small));
            let much = vendor_price(value, max, lost, lost.saturating_sub(large));
            prop_assert!(much >= little);
        }

        #[test]
        fn a_repair_is_free_only_when_it_restores_nothing(
            value in 0.0..50_000.0f64, max in max(), lost in 0..5_000u32, to in 0..5_000u32,
        ) {
            let restored = lost.min(max.get()).saturating_sub(to);
            let price = vendor_price(value, max, lost, to);
            prop_assert_eq!(price == 0, restored == 0);
        }
    }
}
