//! What the player is addicted to.

use std::collections::BTreeMap;
use std::fmt;

use thiserror::Error;

use crate::shared::{BoostStat, BoostTally, Special};

/// Without a dose for this long (three days), an addiction wears off by itself. A
/// stand-in: the wiki's figure was not imported.
pub const WITHDRAWAL_MINUTES: u64 = 3 * 24 * 60;

/// The longest kind name, in characters.
const MAX_KIND_LEN: usize = 40;

/// Why a kind name was refused.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KindError {
    /// Nothing was written.
    #[error("an addiction needs a name")]
    Empty,
    /// More than the longest allowed name.
    #[error("an addiction name has at most {MAX_KIND_LEN} characters")]
    TooLong,
    /// A character other than a letter, a digit, a space, `-` or `_`.
    #[error("an addiction name may hold only letters, digits, spaces, '-' and '_', not {0:?}")]
    BadCharacter(char),
}

/// What an addiction is to: "jet", "med-x", "alcohol". Lowercase, trimmed, and
/// checked once, so the same drug never has two spellings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AddictionKind(String);

impl AddictionKind {
    /// Checks and normalizes a name.
    ///
    /// # Errors
    ///
    /// [`KindError`] naming what is wrong with it.
    pub fn try_new(text: &str) -> Result<Self, KindError> {
        let name = text.trim().to_lowercase();
        if name.is_empty() {
            return Err(KindError::Empty);
        }
        if name.chars().count() > MAX_KIND_LEN {
            return Err(KindError::TooLong);
        }
        if let Some(bad) = name
            .chars()
            .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_')))
        {
            return Err(KindError::BadCharacter(bad));
        }
        Ok(Self(name))
    }

    /// The name, in lowercase.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AddictionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Every addiction the player has, with the minute of the last dose.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Addictions {
    held: BTreeMap<AddictionKind, u64>,
}

impl Addictions {
    /// The addictions of a save, each with the minute of its last dose.
    #[must_use]
    pub fn from_entries(entries: impl IntoIterator<Item = (AddictionKind, u64)>) -> Self {
        Self {
            held: entries.into_iter().collect(),
        }
    }

    /// Records a dose of a chem at minute `now`. A player already addicted to it
    /// only has the last dose moved forward; `addicting` says whether a new
    /// addiction starts when there was none. Returns whether one did.
    pub fn dose(&mut self, kind: &AddictionKind, now: u64, addicting: bool) -> bool {
        if let Some(last) = self.held.get_mut(kind) {
            *last = (*last).max(now);
            return false;
        }
        if addicting {
            self.held.insert(kind.clone(), now);
        }
        addicting
    }

    /// Ends every addiction that has gone [`WITHDRAWAL_MINUTES`] without a dose by
    /// minute `now`, and says whether any ended.
    pub fn expire(&mut self, now: u64) -> bool {
        let before = self.held.len();
        self.held
            .retain(|_, last| last.saturating_add(WITHDRAWAL_MINUTES) > now);
        self.held.len() != before
    }

    /// Ends every addiction; returns how many ended.
    pub fn cure(&mut self) -> usize {
        let count = self.held.len();
        self.held.clear();
        count
    }

    /// Whether the player is addicted to anything.
    #[must_use]
    pub fn any(&self) -> bool {
        !self.held.is_empty()
    }

    /// Whether the player is addicted to this.
    #[must_use]
    pub fn has(&self, kind: &AddictionKind) -> bool {
        self.held.contains_key(kind)
    }

    /// How many addictions the player has.
    #[must_use]
    pub fn len(&self) -> usize {
        self.held.len()
    }

    /// Whether the player has none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    /// Each addiction with the minute of its last dose, by name.
    pub fn iter(&self) -> impl Iterator<Item = (&AddictionKind, u64)> {
        self.held.iter().map(|(kind, last)| (kind, *last))
    }

    /// What withdrawal takes away: one Strength and one Agility for every addiction
    /// (a stand-in for the wiki's per-chem effects, which were not imported).
    #[must_use]
    pub fn withdrawal(&self) -> BoostTally {
        let mut tally = BoostTally::default();
        let count = i16::try_from(self.held.len()).unwrap_or(i16::MAX);
        if count > 0 {
            tally.add(BoostStat::Special(Special::Strength), -count);
            tally.add(BoostStat::Special(Special::Agility), -count);
        }
        tally
    }
}
