//! How many hit points a sleep restores.

use crate::clock::Span;

/// Hours of sleep that restore every hit point.
///
/// The perk texts in the wiki say a sleep restores all health but give no length,
/// so eight hours is a stand-in until the game-data export exists.
pub const FULL_REST_HOURS: u64 = 8;

const MINUTES_PER_HOUR: u128 = 60;

/// The hit points a sleep of `span` restores to a character with `max_hp`.
///
/// Linear in the length of the sleep and rounded up, so even the shortest sleep
/// restores something. Never more than `max_hp`.
#[must_use]
pub fn restored(max_hp: u32, span: Span) -> u32 {
    let full = u128::from(FULL_REST_HOURS) * MINUTES_PER_HOUR;
    // u128 holds the largest max_hp times the largest span, so nothing overflows.
    let share = (u128::from(max_hp) * u128::from(span.as_minutes())).div_ceil(full);
    u32::try_from(share.min(u128::from(max_hp))).unwrap_or(max_hp)
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::*;

    fn hours(n: u32) -> Span {
        Span::hours(NonZeroU32::new(n).expect("the literal is not zero"))
    }

    #[test]
    fn a_full_rest_restores_everything() {
        assert_eq!(restored(120, hours(8)), 120);
    }

    #[test]
    fn a_shorter_sleep_restores_a_share_rounded_up() {
        assert_eq!(restored(100, hours(4)), 50);
        assert_eq!(restored(100, hours(1)), 13);
    }

    #[test]
    fn a_longer_sleep_restores_no_more_than_the_maximum() {
        assert_eq!(restored(100, hours(24)), 100);
        assert_eq!(restored(u32::MAX, hours(u32::MAX)), u32::MAX);
    }
}
