//! Numbers with a fixed range and lists that cannot be empty.
//!
//! Out-of-range values cannot be built, so code holding a `SkillLevel` never
//! has to re-check it.

use serde::{Deserialize, Serialize};

use super::error::ModelError;

macro_rules! define_bounded {
    ($(#[$meta:meta])* $name:ident, $min:literal, $max:literal, $what:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(try_from = "u32", into = "u32")]
        pub struct $name(u8);

        impl $name {
            /// Smallest allowed value.
            pub const MIN: u8 = $min;
            /// Largest allowed value.
            pub const MAX: u8 = $max;

            /// Builds the value, rejecting numbers outside the range.
            ///
            /// # Errors
            ///
            /// Returns [`ModelError::OutOfRange`] when `value` is below
            /// [`MIN`](Self::MIN) or above [`MAX`](Self::MAX).
            pub fn try_new(value: u32) -> Result<Self, ModelError> {
                match u8::try_from(value) {
                    Ok(v) if (Self::MIN..=Self::MAX).contains(&v) => Ok(Self(v)),
                    _ => Err(ModelError::OutOfRange {
                        what: $what,
                        min: u32::from(Self::MIN),
                        max: u32::from(Self::MAX),
                        got: value,
                    }),
                }
            }

            /// The value as a plain number.
            #[must_use]
            pub fn get(self) -> u8 {
                self.0
            }
        }

        impl TryFrom<u32> for $name {
            type Error = ModelError;

            fn try_from(value: u32) -> Result<Self, Self::Error> {
                Self::try_new(value)
            }
        }

        impl From<$name> for u32 {
            fn from(value: $name) -> Self {
                u32::from(value.0)
            }
        }
    };
}

define_bounded!(
    /// A skill level, 0 to 100.
    SkillLevel,
    0,
    100,
    "skill level"
);
define_bounded!(
    /// A S.P.E.C.I.A.L. attribute value, 1 to 10.
    SpecialLevel,
    1,
    10,
    "S.P.E.C.I.A.L. level"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_levels_cover_zero_to_one_hundred() {
        assert!(SkillLevel::try_new(0).is_ok());
        assert!(SkillLevel::try_new(100).is_ok());
        assert!(SkillLevel::try_new(101).is_err());
    }

    #[test]
    fn special_levels_cover_one_to_ten() {
        assert!(SpecialLevel::try_new(0).is_err());
        assert!(SpecialLevel::try_new(1).is_ok());
        assert!(SpecialLevel::try_new(10).is_ok());
        assert!(SpecialLevel::try_new(11).is_err());
    }

    #[test]
    fn values_beyond_a_byte_are_rejected_not_wrapped() {
        assert!(SkillLevel::try_new(256).is_err());
    }
}
