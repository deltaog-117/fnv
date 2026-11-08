//! Integration tests for radiation: the rads held, what resistance keeps out and the
//! sickness. Items and doctors through the composed `Game` are in `tests/game/`.

use engine::radiation::{RADS_PER_TIER, Radiation, absorbed};
use engine::shared::{Rads, Special};
use proptest::prelude::*;

fn holding(rads: u32) -> Radiation {
    Radiation::holding(Rads::try_new(rads).expect("in range"))
}

#[test]
fn resistance_keeps_a_share_out_and_rounds_to_the_nearest_rad() {
    assert_eq!(absorbed(10, 0), 10);
    assert_eq!(absorbed(10, 50), 5);
    assert_eq!(absorbed(3, 50), 2, "1.5 rounds up");
    assert_eq!(absorbed(10, 100), 0);
    assert_eq!(absorbed(10, 250), 0, "more than full is still full");
}

#[test]
fn absorbing_reports_only_what_really_went_in() {
    let mut radiation = holding(995);
    assert_eq!(radiation.absorb(10, 0), 4, "stops at the maximum");
    assert_eq!(radiation.rads().get(), Rads::MAX);
    assert_eq!(radiation.absorb(10, 0), 0);
}

#[test]
fn removing_reports_only_what_really_came_out() {
    let mut radiation = holding(30);
    assert_eq!(radiation.remove(50), 30);
    assert_eq!(radiation.rads(), Rads::NONE);
    assert_eq!(radiation.remove(50), 0);
}

#[test]
fn each_tier_costs_one_endurance_and_one_agility() {
    assert!(holding(RADS_PER_TIER as u32 - 1).sickness().is_empty());
    let sick = holding(u32::from(RADS_PER_TIER) * 2).sickness();
    assert_eq!(sick.special(Special::Endurance), -2);
    assert_eq!(sick.special(Special::Agility), -2);
    assert_eq!(sick.special(Special::Strength), 0);
}

#[test]
fn clearing_makes_the_player_well() {
    let mut radiation = holding(700);
    radiation.clear();
    assert_eq!(radiation, Radiation::default());
    assert!(radiation.sickness().is_empty());
}

proptest! {
    #[test]
    fn what_gets_in_is_never_more_than_what_was_offered(amount in any::<u16>(), resistance in any::<u8>()) {
        prop_assert!(absorbed(amount, resistance) <= amount);
    }

    #[test]
    fn more_resistance_never_lets_more_in(amount in any::<u16>(), low in 0u8..=100, extra in 0u8..=100) {
        prop_assert!(absorbed(amount, low.saturating_add(extra)) <= absorbed(amount, low));
    }

    #[test]
    fn the_rads_stay_in_range_however_they_change(
        start in 0u32..=999,
        steps in prop::collection::vec((any::<bool>(), any::<u16>(), any::<u8>()), 0..12),
    ) {
        let mut radiation = holding(start);
        for (take_in, amount, resistance) in steps {
            if take_in {
                radiation.absorb(amount, resistance);
            } else {
                radiation.remove(amount);
            }
            prop_assert!(radiation.rads().get() <= Rads::MAX);
        }
    }

    #[test]
    fn more_rads_are_never_less_sick(a in 0u32..=999, b in 0u32..=999) {
        let (low, high) = (a.min(b), a.max(b));
        prop_assert!(holding(low).tier() <= holding(high).tier());
        for stat in [Special::Endurance, Special::Agility] {
            prop_assert!(holding(high).sickness().special(stat) <= holding(low).sickness().special(stat));
        }
    }

    #[test]
    fn what_comes_out_plus_what_is_left_is_what_was_held(start in 0u32..=999, amount in any::<u16>()) {
        let mut radiation = holding(start);
        let out = radiation.remove(amount);
        prop_assert_eq!(u32::from(out) + u32::from(radiation.rads().get()), start);
    }
}
