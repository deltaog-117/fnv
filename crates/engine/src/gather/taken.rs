//! Which finds the player has already taken.

use std::collections::BTreeSet;

use super::error::GatherError;
use crate::shared::FindId;

/// The finds taken so far. A find taken once stays taken.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Gathered {
    taken: BTreeSet<FindId>,
}

impl Gathered {
    /// Whether the find has been taken.
    #[must_use]
    pub fn has_taken(&self, find: &FindId) -> bool {
        self.taken.contains(find)
    }

    /// Marks the find taken.
    ///
    /// # Errors
    ///
    /// [`GatherError::AlreadyTaken`] when it was taken before; nothing changes.
    pub fn take(&mut self, find: &FindId) -> Result<(), GatherError> {
        if self.taken.insert(find.clone()) {
            Ok(())
        } else {
            Err(GatherError::AlreadyTaken(find.clone()))
        }
    }

    /// The taken finds, in id order.
    pub fn iter(&self) -> impl Iterator<Item = &FindId> {
        self.taken.iter()
    }

    /// How many finds have been taken.
    #[must_use]
    pub fn len(&self) -> usize {
        self.taken.len()
    }

    /// Whether nothing has been taken.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.taken.is_empty()
    }
}
