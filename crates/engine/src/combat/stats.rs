//! Combat numbers that cannot be built out of range.

use thiserror::Error;

/// A combat number was outside its allowed range.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum StatError {
    /// The value was not within the quantity's bounds (or was not a number).
    #[error("{what} must be between {min} and {max}, got {got}")]
    OutOfRange {
        /// The quantity being checked (for example `"multiplier"`).
        what: &'static str,
        /// Smallest allowed value.
        min: f64,
        /// Largest allowed value.
        max: f64,
        /// The rejected value.
        got: f64,
    },
}

/// A finite amount that is zero or more: damage, a threshold, a resistance.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Points(f64);

impl Points {
    /// No points.
    pub const ZERO: Self = Self(0.0);

    /// Builds the amount, rejecting negatives, infinities and NaN.
    ///
    /// # Errors
    ///
    /// Returns [`StatError::OutOfRange`] when `value` is not a finite number of at
    /// least zero.
    pub fn try_new(value: f64) -> Result<Self, StatError> {
        if value.is_finite() && value >= 0.0 {
            Ok(Self(value))
        } else {
            Err(StatError::OutOfRange {
                what: "points",
                min: 0.0,
                max: f64::MAX,
                got: value,
            })
        }
    }

    /// The amount as a plain number.
    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

/// A damage multiplier from skill or weapon condition, from half to full effect.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Multiplier(f64);

impl Multiplier {
    /// Smallest allowed value.
    pub const MIN: f64 = 0.5;
    /// Largest allowed value.
    pub const MAX: f64 = 1.0;
    /// Full effect.
    pub const FULL: Self = Self(1.0);

    /// Builds the multiplier, rejecting values outside 0.5 to 1 and NaN.
    ///
    /// # Errors
    ///
    /// Returns [`StatError::OutOfRange`] when `value` is outside the range.
    pub fn try_new(value: f64) -> Result<Self, StatError> {
        if (Self::MIN..=Self::MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(StatError::OutOfRange {
                what: "multiplier",
                min: Self::MIN,
                max: Self::MAX,
                got: value,
            })
        }
    }

    /// The multiplier as a plain number.
    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

/// A chance from 0 to 100 percent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Percent(u8);

impl Percent {
    /// Never.
    pub const NEVER: Self = Self(0);
    /// Always.
    pub const ALWAYS: Self = Self(100);

    /// Builds the percentage, rejecting numbers above 100.
    ///
    /// # Errors
    ///
    /// Returns [`StatError::OutOfRange`] when `value` is above 100.
    pub fn try_new(value: u32) -> Result<Self, StatError> {
        match u8::try_from(value) {
            Ok(v) if v <= 100 => Ok(Self(v)),
            _ => Err(StatError::OutOfRange {
                what: "percent",
                min: 0.0,
                max: 100.0,
                got: f64::from(value),
            }),
        }
    }

    /// The percentage as a plain number.
    #[must_use]
    pub fn get(self) -> u8 {
        self.0
    }
}

/// How hard an attack is pressed: the `power` factor of the damage formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Power {
    /// An ordinary attack.
    Normal,
    /// A melee or unarmed power attack, or a VATS attack that is not a sneak attack.
    Boosted,
}

impl Power {
    /// The factor the damage formula multiplies by.
    #[must_use]
    pub fn factor(self) -> f64 {
        match self {
            Self::Normal => 1.0,
            Self::Boosted => 2.0,
        }
    }
}
