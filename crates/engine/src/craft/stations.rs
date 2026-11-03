//! The crafting stations that exist, read-only.

use std::collections::BTreeMap;

use super::error::CraftError;
use crate::shared::{GameData, LocationId, Station, StationId, StationKind};

/// A station: what it is called, its kind and where it stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StationInfo {
    name: String,
    kind: StationKind,
    place: LocationId,
}

impl StationInfo {
    /// What the player sees when choosing to craft.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The kind of station.
    #[must_use]
    pub fn kind(&self) -> StationKind {
        self.kind
    }

    /// Where the station stands.
    #[must_use]
    pub fn place(&self) -> &LocationId {
        &self.place
    }
}

/// Every station, checked once at load.
#[derive(Debug, Clone, Default)]
pub struct StationBook {
    stations: BTreeMap<StationId, StationInfo>,
}

impl StationBook {
    /// Builds the book from the loaded data.
    ///
    /// # Errors
    ///
    /// The same as [`StationBook::new`].
    pub fn from_data(data: &GameData) -> Result<Self, CraftError> {
        Self::new(data.stations.iter())
    }

    /// Builds the book from station records.
    ///
    /// # Errors
    ///
    /// [`CraftError::DuplicateStation`] when an id repeats.
    pub fn new<'a>(records: impl IntoIterator<Item = &'a Station>) -> Result<Self, CraftError> {
        let mut stations = BTreeMap::new();
        for record in records {
            let info = StationInfo {
                name: record.name.clone(),
                kind: record.kind,
                place: record.place.clone(),
            };
            if stations.insert(record.id.clone(), info).is_some() {
                return Err(CraftError::DuplicateStation(record.id.clone()));
            }
        }
        Ok(Self { stations })
    }

    /// The station with this id.
    #[must_use]
    pub fn get(&self, station: &StationId) -> Option<&StationInfo> {
        self.stations.get(station)
    }

    /// The stations at a place, in id order.
    pub fn at<'a>(
        &'a self,
        place: &'a LocationId,
    ) -> impl Iterator<Item = (&'a StationId, &'a StationInfo)> {
        self.stations
            .iter()
            .filter(move |(_, info)| &info.place == place)
    }

    /// How many stations there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.stations.len()
    }

    /// Whether there are no stations.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stations.is_empty()
    }
}
