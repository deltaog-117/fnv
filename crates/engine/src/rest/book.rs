//! The beds that exist, read-only.

use std::collections::BTreeMap;

use super::error::RestError;
use crate::shared::{Bed, BedId, GameData, LocationId};

/// A bed: what it is called and where it stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BedInfo {
    name: String,
    place: LocationId,
}

impl BedInfo {
    /// What the player sees when choosing to sleep.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Where the bed stands.
    #[must_use]
    pub fn place(&self) -> &LocationId {
        &self.place
    }
}

/// Every bed, checked once at load.
#[derive(Debug, Clone, Default)]
pub struct BedBook {
    beds: BTreeMap<BedId, BedInfo>,
}

impl BedBook {
    /// Builds the book from the loaded data.
    ///
    /// # Errors
    ///
    /// The same as [`BedBook::new`].
    pub fn from_data(data: &GameData) -> Result<Self, RestError> {
        Self::new(data.beds.iter())
    }

    /// Builds the book from bed records.
    ///
    /// # Errors
    ///
    /// [`RestError::DuplicateBed`] when an id repeats.
    pub fn new<'a>(records: impl IntoIterator<Item = &'a Bed>) -> Result<Self, RestError> {
        let mut beds = BTreeMap::new();
        for record in records {
            let info = BedInfo {
                name: record.name.clone(),
                place: record.place.clone(),
            };
            if beds.insert(record.id.clone(), info).is_some() {
                return Err(RestError::DuplicateBed(record.id.clone()));
            }
        }
        Ok(Self { beds })
    }

    /// The bed with this id.
    #[must_use]
    pub fn get(&self, bed: &BedId) -> Option<&BedInfo> {
        self.beds.get(bed)
    }

    /// The beds at a place, in id order.
    pub fn at<'a>(
        &'a self,
        place: &'a LocationId,
    ) -> impl Iterator<Item = (&'a BedId, &'a BedInfo)> {
        self.beds
            .iter()
            .filter(move |(_, info)| &info.place == place)
    }

    /// How many beds there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.beds.len()
    }

    /// Whether there are no beds.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.beds.is_empty()
    }
}
