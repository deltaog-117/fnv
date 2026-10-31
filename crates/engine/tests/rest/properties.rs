//! The recovery law, checked over random sleeps.

use std::num::NonZeroU32;

use engine::clock::Span;
use engine::rest::{FULL_REST_HOURS, restored};
use proptest::prelude::*;

fn nz(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).expect("the range starts at 1")
}

proptest! {
    #[test]
    fn a_sleep_never_restores_more_than_the_maximum(
        max_hp: u32, minutes in 1..=u32::MAX,
    ) {
        prop_assert!(restored(max_hp, Span::minutes(nz(minutes))) <= max_hp);
    }

    #[test]
    fn a_sleep_of_the_full_length_or_more_restores_everything(
        max_hp: u32, extra in 0..=1000u32,
    ) {
        let hours = nz(u32::try_from(FULL_REST_HOURS).expect("small") + extra);
        prop_assert_eq!(restored(max_hp, Span::hours(hours)), max_hp);
    }

    #[test]
    fn sleeping_longer_never_restores_less(
        max_hp in 0..10_000u32, a in 1..=100_000u32, b in 1..=100_000u32,
    ) {
        let (short, long) = (a.min(b), a.max(b));
        prop_assert!(
            restored(max_hp, Span::minutes(nz(short)))
                <= restored(max_hp, Span::minutes(nz(long)))
        );
    }

    #[test]
    fn any_sleep_restores_something_to_a_character_with_hit_points(
        max_hp in 1..10_000u32, minutes in 1..=100_000u32,
    ) {
        prop_assert!(restored(max_hp, Span::minutes(nz(minutes))) >= 1);
    }
}
