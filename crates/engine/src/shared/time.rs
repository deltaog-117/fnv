//! A time of day and a window of it, for conditions that ask what hour it is.
//!
//! Sources give windows with odd edges (8:15 pm to 5:45 am), so the unit is the
//! minute, and a window may cross midnight. A window whose two ends are the same
//! minute is refused: it could mean "never" or "all day", and a rule should say
//! which.

use serde::{Deserialize, Serialize};

use super::error::ModelError;

const MINUTES_PER_HOUR: u16 = 60;
const MINUTES_PER_DAY: u16 = 24 * MINUTES_PER_HOUR;

/// A minute of the day, 00:00 to 23:59. Written `"HH:MM"` in data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct MinuteOfDay(u16);

impl MinuteOfDay {
    /// Midnight.
    pub const MIDNIGHT: Self = Self(0);

    /// The minute `hour`:`minute` past midnight.
    ///
    /// # Errors
    ///
    /// [`ModelError::OutOfRange`] when the hour is past 23 or the minute past 59.
    pub fn new(hour: u32, minute: u32) -> Result<Self, ModelError> {
        if hour > 23 {
            return Err(ModelError::OutOfRange {
                what: "hour",
                min: 0,
                max: 23,
                got: hour,
            });
        }
        if minute > 59 {
            return Err(ModelError::OutOfRange {
                what: "minute",
                min: 0,
                max: 59,
                got: minute,
            });
        }
        // Both are in range, so the sum is below 1,440 and fits.
        Ok(Self((hour * 60 + minute) as u16))
    }

    /// The minute of the day that `minutes` since any midnight falls on.
    #[must_use]
    pub fn from_total_minutes(minutes: u64) -> Self {
        // The remainder is below 1,440, so the cast cannot lose anything.
        Self((minutes % u64::from(MINUTES_PER_DAY)) as u16)
    }

    /// Minutes since midnight, 0 to 1,439.
    #[must_use]
    pub fn get(self) -> u16 {
        self.0
    }

    /// The hour, 0 to 23.
    #[must_use]
    pub fn hour(self) -> u8 {
        // At most 23 by the range of the minute count.
        (self.0 / MINUTES_PER_HOUR) as u8
    }

    /// The minute of the hour, 0 to 59.
    #[must_use]
    pub fn minute(self) -> u8 {
        // Below 60 by the modulo.
        (self.0 % MINUTES_PER_HOUR) as u8
    }
}

impl TryFrom<String> for MinuteOfDay {
    type Error = ModelError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        let malformed = || ModelError::InvalidTime(text.clone());
        let (hour, minute) = text.split_once(':').ok_or_else(malformed)?;
        let two_digits = |part: &str| part.len() == 2 && part.bytes().all(|b| b.is_ascii_digit());
        if !two_digits(hour) || !two_digits(minute) {
            return Err(malformed());
        }
        let hour: u32 = hour.parse().map_err(|_| malformed())?;
        let minute: u32 = minute.parse().map_err(|_| malformed())?;
        Self::new(hour, minute)
    }
}

impl From<MinuteOfDay> for String {
    fn from(time: MinuteOfDay) -> Self {
        format!("{:02}:{:02}", time.hour(), time.minute())
    }
}

/// The span of the day from `from` (included) up to `to` (excluded). When `to` is
/// earlier than `from` the window crosses midnight: 18:00 to 06:00 is the night.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "TimeWindowWire", into = "TimeWindowWire")]
pub struct TimeWindow {
    from: MinuteOfDay,
    to: MinuteOfDay,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TimeWindowWire {
    from: MinuteOfDay,
    to: MinuteOfDay,
}

impl TimeWindow {
    /// A window from `from` up to `to`.
    ///
    /// # Errors
    ///
    /// [`ModelError::EmptyWindow`] when both ends are the same minute.
    pub fn new(from: MinuteOfDay, to: MinuteOfDay) -> Result<Self, ModelError> {
        if from == to {
            return Err(ModelError::EmptyWindow(String::from(from)));
        }
        Ok(Self { from, to })
    }

    /// Where the window starts.
    #[must_use]
    pub fn from(self) -> MinuteOfDay {
        self.from
    }

    /// Where the window ends, not included.
    #[must_use]
    pub fn to(self) -> MinuteOfDay {
        self.to
    }

    /// Whether `time` falls inside the window.
    #[must_use]
    pub fn contains(self, time: MinuteOfDay) -> bool {
        if self.from < self.to {
            self.from <= time && time < self.to
        } else {
            time >= self.from || time < self.to
        }
    }
}

impl TryFrom<TimeWindowWire> for TimeWindow {
    type Error = ModelError;

    fn try_from(wire: TimeWindowWire) -> Result<Self, Self::Error> {
        Self::new(wire.from, wire.to)
    }
}

impl From<TimeWindow> for TimeWindowWire {
    fn from(window: TimeWindow) -> Self {
        Self {
            from: window.from,
            to: window.to,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(hour: u32, minute: u32) -> MinuteOfDay {
        MinuteOfDay::new(hour, minute).expect("a valid time")
    }

    #[test]
    fn a_window_that_crosses_midnight_holds_both_sides_of_it() {
        let night = TimeWindow::new(at(20, 15), at(5, 45)).expect("a window");
        assert!(night.contains(at(23, 0)) && night.contains(at(4, 0)));
        assert!(night.contains(at(20, 15)) && !night.contains(at(5, 45)));
        assert!(!night.contains(at(12, 0)));
    }

    #[test]
    fn times_are_read_from_hh_mm_and_nothing_looser() {
        assert_eq!(MinuteOfDay::try_from("06:05".to_owned()), Ok(at(6, 5)));
        for bad in [
            "6:05", "24:00", "12:60", "12-30", "1230", "ab:cd", "12:5", "",
        ] {
            assert!(MinuteOfDay::try_from(bad.to_owned()).is_err(), "{bad}");
        }
    }
}
