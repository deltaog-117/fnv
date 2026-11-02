//! The map: every place, and the links between neighbors.

use std::collections::BTreeMap;

use super::error::TravelError;
use super::pace::minutes_for;
use crate::clock::Span;
use crate::shared::{GameData, Location, LocationId};

/// One way out of a place: where it leads and how long the walk takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    /// The place at the other end.
    pub to: LocationId,
    /// What the player sees for that place.
    pub name: String,
    /// How long the walk takes.
    pub span: Span,
}

#[derive(Debug, Clone)]
struct Place {
    name: String,
    /// Distance in map pixels to each linked place; links are two-way.
    links: BTreeMap<LocationId, f64>,
}

/// Every place and link, checked once at load.
#[derive(Debug, Clone, Default)]
pub struct TravelBook {
    places: BTreeMap<LocationId, Place>,
}

impl TravelBook {
    /// Builds the map from the loaded data.
    ///
    /// # Errors
    ///
    /// The same as [`TravelBook::new`].
    pub fn from_data(data: &GameData) -> Result<Self, TravelError> {
        Self::new(data.locations.iter())
    }

    /// Builds the map from location records.
    ///
    /// A link written on one record joins both places. If both records write it
    /// with different distances, the shorter one is kept.
    ///
    /// # Errors
    ///
    /// - [`TravelError::DuplicatePlace`] when an id repeats;
    /// - [`TravelError::UnknownPlace`] when a link names a place that is not there;
    /// - [`TravelError::SelfLink`] when a place links to itself;
    /// - [`TravelError::BadDistance`] when a distance is not a finite number above
    ///   zero.
    pub fn new<'a>(records: impl IntoIterator<Item = &'a Location>) -> Result<Self, TravelError> {
        let records: Vec<&Location> = records.into_iter().collect();
        let mut places = BTreeMap::new();
        for record in &records {
            let place = Place {
                name: record.name.clone(),
                links: BTreeMap::new(),
            };
            if places.insert(record.id.clone(), place).is_some() {
                return Err(TravelError::DuplicatePlace(record.id.clone()));
            }
        }
        for record in &records {
            for link in &record.neighbors {
                if link.id == record.id {
                    return Err(TravelError::SelfLink(record.id.clone()));
                }
                if !places.contains_key(&link.id) {
                    return Err(TravelError::UnknownPlace {
                        from: record.id.clone(),
                        to: link.id.clone(),
                    });
                }
                if minutes_for(link.distance).is_none() {
                    return Err(TravelError::BadDistance {
                        from: record.id.clone(),
                        to: link.id.clone(),
                        distance: link.distance,
                    });
                }
                for (a, b) in [(&record.id, &link.id), (&link.id, &record.id)] {
                    if let Some(place) = places.get_mut(a) {
                        let kept = place.links.entry(b.clone()).or_insert(link.distance);
                        *kept = kept.min(link.distance);
                    }
                }
            }
        }
        Ok(Self { places })
    }

    /// Whether the place is on the map.
    #[must_use]
    pub fn contains(&self, place: &LocationId) -> bool {
        self.places.contains_key(place)
    }

    /// What the player sees for a place.
    #[must_use]
    pub fn name(&self, place: &LocationId) -> Option<&str> {
        self.places.get(place).map(|p| p.name.as_str())
    }

    /// The time to walk from `from` to `to`, if they are linked.
    #[must_use]
    pub fn span_between(&self, from: &LocationId, to: &LocationId) -> Option<Span> {
        let distance = *self.places.get(from)?.links.get(to)?;
        minutes_for(distance).map(Span::minutes)
    }

    /// The ways out of a place, nearest first and then by id. Empty for a place
    /// that is not on the map or has no links.
    #[must_use]
    pub fn routes_from(&self, from: &LocationId) -> Vec<Route> {
        let Some(place) = self.places.get(from) else {
            return Vec::new();
        };
        let mut links: Vec<(&LocationId, f64)> =
            place.links.iter().map(|(id, d)| (id, *d)).collect();
        links.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(b.0)));
        links
            .into_iter()
            .filter_map(|(to, distance)| {
                let span = minutes_for(distance).map(Span::minutes)?;
                let name = self.places.get(to)?.name.clone();
                Some(Route {
                    to: to.clone(),
                    name,
                    span,
                })
            })
            .collect()
    }

    /// How many places there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.places.len()
    }

    /// Whether the map is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.places.is_empty()
    }
}
