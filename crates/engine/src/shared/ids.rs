//! Typed identifiers.
//!
//! Every id is a distinct type, so a `QuestId` can never be passed where an
//! `NpcId` is expected. The only way to obtain one is [`try_new`], which enforces
//! the pattern from `schemas/common.schema.json`: `^[a-z0-9][a-z0-9_]*$`.
//!
//! [`try_new`]: QuestId::try_new

use std::fmt;

use serde::{Deserialize, Serialize};

use super::error::ModelError;

/// True when `value` matches `^[a-z0-9][a-z0-9_]*$`.
fn is_valid_id(value: &str) -> bool {
    let mut chars = value.chars();
    let starts_well = chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    starts_well && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident, $kind:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            /// Builds the id, rejecting text that does not match the id pattern.
            ///
            /// # Errors
            ///
            /// Returns [`ModelError::InvalidId`] when `value` is empty, starts with
            /// an underscore, or contains anything but lowercase ASCII letters,
            /// digits and underscores.
            pub fn try_new(value: impl Into<String>) -> Result<Self, ModelError> {
                let value = value.into();
                if is_valid_id(&value) {
                    Ok(Self(value))
                } else {
                    Err(ModelError::InvalidId { kind: $kind, value })
                }
            }

            /// The id as text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = ModelError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::try_new(value)
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

define_id!(
    /// Identifies a quest.
    QuestId,
    "quest"
);
define_id!(
    /// Identifies a faction or a town that tracks reputation.
    FactionId,
    "faction"
);
define_id!(
    /// Identifies a perk.
    PerkId,
    "perk"
);
define_id!(
    /// Identifies a companion.
    CompanionId,
    "companion"
);
define_id!(
    /// Identifies an item of any kind.
    ItemId,
    "item"
);
define_id!(
    /// Identifies a story flag set by an effect and read by a condition.
    FlagId,
    "flag"
);
define_id!(
    /// Identifies a non-player character.
    NpcId,
    "npc"
);
define_id!(
    /// Identifies a game ending.
    EndingId,
    "ending"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_lowercase_digits_and_underscores() {
        assert!(QuestId::try_new("ain_t_that_a_kick_in_the_head").is_ok());
        assert!(ItemId::try_new("357_magnum_revolver").is_ok());
    }

    #[test]
    fn rejects_the_shapes_the_schema_rejects() {
        for bad in ["", "_hidden", "Upper", "has space", "dash-ed", "ünïcode"] {
            assert!(QuestId::try_new(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn error_names_the_kind_of_id() {
        let err = NpcId::try_new("Bad Id").unwrap_err();
        assert!(err.to_string().contains("npc"), "{err}");
    }
}
