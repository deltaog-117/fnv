//! Integration tests for poison: the points held, what resistance keeps out and how
//! fast time takes them. Antidotes and items through the composed `Game` are in
//! `tests/game/`.

use engine::poison::{DRAIN_PER_MINUTE, Poison, taken_in};
use proptest::prelude::*;

#[test]
fn resistance_keeps_a_share_out_and_rounds_to_the_nearest_point() {
    assert_eq!(taken_in(10, 0), 10);
    assert_eq!(taken_in(10, 50), 5);
    assert_eq!(taken_in(3, 50), 2, "1.5 rounds up");
    assert_eq!(taken_in(10, 100), 0);
    assert_eq!(taken_in(10, 250), 0, "more than full is still full");
}

#[test]
fn absorbing_reports_only_what_really_went_in() {
    let mut poison = Poison::holding(995);
    assert_eq!(poison.absorb(10, 0), 4, "stops at the maximum");
    assert_eq!(poison.left(), Poison::MAX);
    assert_eq!(poison.absorb(10, 0), 0);
}

#[test]
fn holding_never_goes_past_the_maximum() {
    assert_eq!(Poison::holding(u16::MAX).left(), Poison::MAX);
}

#[test]
fn removing_reports_only_what_really_came_out() {
    let mut poison = Poison::holding(30);
    assert_eq!(poison.remove(50), 30);
    assert!(!poison.is_poisoned());
    assert_eq!(poison.remove(50), 0);
}

#[test]
fn time_pays_out_a_fixed_rate_and_never_more_than_is_held() {
    let mut poison = Poison::holding(12);
    assert_eq!(poison.elapse(1), DRAIN_PER_MINUTE);
    assert_eq!(poison.left(), 12 - DRAIN_PER_MINUTE);
    assert_eq!(poison.elapse(60 * 24), 12 - DRAIN_PER_MINUTE);
    assert_eq!(poison, Poison::default());
    assert_eq!(poison.elapse(5), 0);
}

#[test]
fn an_antidote_ends_poison_of_any_size() {
    let mut poison = Poison::holding(Poison::MAX);
    poison.clear();
    assert_eq!(poison, Poison::default());
}

proptest! {
    #[test]
    fn what_gets_in_is_never_more_than_what_was_offered(amount in any::<u16>(), resistance in any::<u8>()) {
        prop_assert!(taken_in(amount, resistance) <= amount);
    }

    #[test]
    fn full_resistance_lets_none_in(amount in any::<u16>(), resistance in 100u8..) {
        prop_assert_eq!(taken_in(amount, resistance), 0);
    }

    #[test]
    fn time_only_lowers_the_points_by_exactly_what_it_reports(
        held in 0..=Poison::MAX,
        minutes in any::<u64>(),
    ) {
        let mut poison = Poison::holding(held);
        let drained = poison.elapse(minutes);
        prop_assert_eq!(poison.left() + drained, held);
        prop_assert!(poison.left() <= held);
    }

    #[test]
    fn the_points_stay_in_range_whatever_comes_in(
        held in any::<u16>(),
        amount in any::<u16>(),
        resistance in any::<u8>(),
    ) {
        let mut poison = Poison::holding(held);
        let got_in = poison.absorb(amount, resistance);
        prop_assert!(poison.left() <= Poison::MAX);
        prop_assert!(got_in <= amount);
    }
}
