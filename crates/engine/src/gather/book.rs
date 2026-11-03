//! The finds that exist, read-only.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use super::error::GatherError;
use crate::shared::{Find, FindId, GameData, ItemId, LocationId};

/// A find: what it is called, where it lies and what the player takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindInfo {
    name: String,
    place: LocationId,
    items: Vec<(ItemId, NonZeroU32)>,
}

impl FindInfo {
    /// What the player sees when choosing to take it.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Where it lies.
    #[must_use]
    pub fn place(&self) -> &LocationId {
        &self.place
    }

    /// What is taken, each item once.
    pub fn items(&self) -> impl Iterator<Item = (&ItemId, NonZeroU32)> {
        self.items.iter().map(|(item, count)| (item, *count))
    }
}

/// Every find, checked once at load.
#[derive(Debug, Clone, Default)]
pub struct FindBook {
    finds: BTreeMap<FindId, FindInfo>,
}

impl FindBook {
    /// Builds the book from the loaded data.
    ///
    /// # Errors
    ///
    /// The same as [`FindBook::new`].
    pub fn from_data(data: &GameData) -> Result<Self, GatherError> {
        Self::new(data.finds.iter())
    }

    /// Builds the book from find records.
    ///
    /// # Errors
    ///
    /// [`GatherError::DuplicateFind`] or [`GatherError::DuplicateItem`] when an id
    /// or an item repeats.
    pub fn new<'a>(records: impl IntoIterator<Item = &'a Find>) -> Result<Self, GatherError> {
        let mut finds = BTreeMap::new();
        for record in records {
            let mut items: Vec<(ItemId, NonZeroU32)> = Vec::new();
            for line in record.items.iter() {
                if items.iter().any(|(item, _)| item == &line.item) {
                    return Err(GatherError::DuplicateItem {
                        find: record.id.clone(),
                        item: line.item.clone(),
                    });
                }
                items.push((line.item.clone(), line.count));
            }
            let info = FindInfo {
                name: record.name.clone(),
                place: record.place.clone(),
                items,
            };
            if finds.insert(record.id.clone(), info).is_some() {
                return Err(GatherError::DuplicateFind(record.id.clone()));
            }
        }
        Ok(Self { finds })
    }

    /// The find with this id.
    #[must_use]
    pub fn get(&self, find: &FindId) -> Option<&FindInfo> {
        self.finds.get(find)
    }

    /// The finds at a place, in id order.
    pub fn at<'a>(
        &'a self,
        place: &'a LocationId,
    ) -> impl Iterator<Item = (&'a FindId, &'a FindInfo)> {
        self.finds
            .iter()
            .filter(move |(_, info)| &info.place == place)
    }

    /// How many finds there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.finds.len()
    }

    /// Whether there are no finds.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.finds.is_empty()
    }
}
