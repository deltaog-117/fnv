//! The clock laws, checked over random spans and starting times.

use std::num::NonZeroU32;

use engine::clock::{ClockError, GameClock, Span};
use proptest::prelude::*;

fn span_minutes() -> impl Strategy<Value = NonZeroU32> {
    (1..=u32::MAX / 2).prop_map(|n| NonZeroU32::new(n).expect("the range starts at 1"))
}

proptest! {
    #[test]
    fn time_only_moves_forward_by_exactly_the_span(
        start in 0..u64::MAX / 2, span in span_minutes(),
    ) {
        let mut clock = GameClock::from_minutes(start);
        clock.advance(Span::minutes(span)).expect("far from the limit");
        prop_assert_eq!(clock.as_minutes(), start + u64::from(span.get()));
    }

    #[test]
    fn waiting_twice_is_waiting_once_for_the_total(
        start in 0..u64::MAX / 2, a in span_minutes(), b in span_minutes(),
    ) {
        let mut twice = GameClock::from_minutes(start);
        twice.advance(Span::minutes(a)).expect("far from the limit");
        twice.advance(Span::minutes(b)).expect("far from the limit");
        let total = a.get() + b.get();
        let mut once = GameClock::from_minutes(start);
        once.advance(Span::minutes(NonZeroU32::new(total).expect("sum of two non-zero")))
            .expect("far from the limit");
        prop_assert_eq!(twice, once);
    }

    #[test]
    fn the_date_is_always_well_formed(minutes: u64) {
        let clock = GameClock::from_minutes(minutes);
        prop_assert!(clock.day() >= 1);
        prop_assert!(clock.hour() < 24);
        prop_assert!(clock.minute() < 60);
        // The three parts rebuild the stored count exactly.
        let rebuilt = (clock.day() - 1) * 1440
            + u64::from(clock.hour()) * 60
            + u64::from(clock.minute());
        prop_assert_eq!(rebuilt, minutes);
    }

    #[test]
    fn an_hour_is_sixty_minutes(start in 0..u64::MAX / 2, hours in 1..=1000u32) {
        let hours = NonZeroU32::new(hours).expect("the range starts at 1");
        let mut by_hours = GameClock::from_minutes(start);
        by_hours.advance(Span::hours(hours)).expect("far from the limit");
        let minutes = hours.get() * 60;
        let mut by_minutes = GameClock::from_minutes(start);
        by_minutes
            .advance(Span::minutes(NonZeroU32::new(minutes).expect("not zero")))
            .expect("far from the limit");
        prop_assert_eq!(by_hours, by_minutes);
    }

    #[test]
    fn a_refused_advance_leaves_the_clock_alone(
        headroom in 0..1000u64, span in 1..=u32::MAX,
    ) {
        let start = GameClock::from_minutes(u64::MAX - headroom);
        let mut clock = start;
        let span = Span::minutes(NonZeroU32::new(span).expect("the range starts at 1"));
        match clock.advance(span) {
            Ok(()) => prop_assert!(span.as_minutes() <= headroom),
            Err(error) => {
                prop_assert_eq!(error, ClockError::Overflow);
                prop_assert_eq!(clock, start);
            }
        }
    }
}
