//! The clock and the lengths of time that move it.

use std::fmt;
use std::num::NonZeroU32;

use super::error::ClockError;

const MINUTES_PER_HOUR: u64 = 60;
const MINUTES_PER_DAY: u64 = 24 * MINUTES_PER_HOUR;
/// The game opens in the morning of day 1, so the first day is not spent in the dark.
const START_HOUR: u64 = 8;

/// A length of time, at least one minute.
///
/// The count is private and built only from non-zero numbers, so a zero-length
/// wait cannot be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Span(u64);

impl Span {
    /// A span of `count` minutes.
    #[must_use]
    pub fn minutes(count: NonZeroU32) -> Self {
        Self(u64::from(count.get()))
    }

    /// A span of `count` hours.
    #[must_use]
    pub fn hours(count: NonZeroU32) -> Self {
        Self(u64::from(count.get()) * MINUTES_PER_HOUR)
    }

    /// The length in minutes, never zero.
    #[must_use]
    pub fn as_minutes(self) -> u64 {
        self.0
    }
}

/// What day and time it is in the game.
///
/// Stored as the minutes since the start of day 1 at 00:00. Day, hour and minute
/// are worked out from that one number, so they can never disagree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GameClock {
    minutes: u64,
}

impl GameClock {
    /// Day 1, 08:00: where a new game begins.
    pub const START: Self = Self {
        minutes: START_HOUR * MINUTES_PER_HOUR,
    };

    /// A clock at `minutes` since the start of day 1 at 00:00, for restoring a
    /// saved game.
    #[must_use]
    pub fn from_minutes(minutes: u64) -> Self {
        Self { minutes }
    }

    /// Minutes since the start of day 1 at 00:00.
    #[must_use]
    pub fn as_minutes(self) -> u64 {
        self.minutes
    }

    /// The day, counting from 1.
    #[must_use]
    pub fn day(self) -> u64 {
        self.minutes / MINUTES_PER_DAY + 1
    }

    /// The hour of the day, 0 to 23.
    #[must_use]
    pub fn hour(self) -> u8 {
        // Below 24 by the modulo, so the cast cannot lose anything.
        (self.minutes % MINUTES_PER_DAY / MINUTES_PER_HOUR) as u8
    }

    /// The minute of the hour, 0 to 59.
    #[must_use]
    pub fn minute(self) -> u8 {
        // Below 60 by the modulo, so the cast cannot lose anything.
        (self.minutes % MINUTES_PER_HOUR) as u8
    }

    /// Moves the clock forward by `span`.
    ///
    /// # Errors
    ///
    /// [`ClockError::Overflow`] when the new time does not fit; the clock is left
    /// as it was.
    pub fn advance(&mut self, span: Span) -> Result<(), ClockError> {
        self.minutes = self
            .minutes
            .checked_add(span.as_minutes())
            .ok_or(ClockError::Overflow)?;
        Ok(())
    }
}

impl Default for GameClock {
    /// The start of a new game.
    fn default() -> Self {
        Self::START
    }
}

impl fmt::Display for GameClock {
    /// For example `Day 1, 08:00`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Day {}, {:02}:{:02}",
            self.day(),
            self.hour(),
            self.minute()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nz(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).expect("the literal is not zero")
    }

    #[test]
    fn a_new_game_starts_at_eight_in_the_morning_of_day_one() {
        let clock = GameClock::default();
        assert_eq!((clock.day(), clock.hour(), clock.minute()), (1, 8, 0));
        assert_eq!(clock.to_string(), "Day 1, 08:00");
    }

    #[test]
    fn sixteen_hours_from_the_start_is_the_next_midnight() {
        let mut clock = GameClock::START;
        clock
            .advance(Span::hours(nz(16)))
            .expect("far from the limit");
        assert_eq!(clock.to_string(), "Day 2, 00:00");
    }

    #[test]
    fn minutes_roll_into_hours() {
        let mut clock = GameClock::START;
        clock
            .advance(Span::minutes(nz(75)))
            .expect("far from the limit");
        assert_eq!(clock.to_string(), "Day 1, 09:15");
    }

    #[test]
    fn advancing_past_the_largest_time_is_refused_and_changes_nothing() {
        let mut clock = GameClock::from_minutes(u64::MAX - 10);
        let before = clock;
        assert_eq!(
            clock.advance(Span::minutes(nz(11))),
            Err(ClockError::Overflow)
        );
        assert_eq!(clock, before);
        assert_eq!(clock.advance(Span::minutes(nz(10))), Ok(()));
        assert_eq!(clock.as_minutes(), u64::MAX);
    }
}
