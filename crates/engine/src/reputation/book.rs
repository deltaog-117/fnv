//! The factions that track reputation, read-only.

use std::collections::BTreeMap;

use super::error::ReputationError;
use crate::shared::{Faction, FactionId, GameData, RenownTier, ReputationLevel};

/// The fame and infamy model of one faction or town.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactionModel {
    name: String,
    thresholds: [u32; 4],
    titles: [[String; 4]; 4],
}

impl FactionModel {
    /// What the player sees for the faction.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The points at which each tier starts; the first is always zero.
    #[must_use]
    pub fn thresholds(&self) -> [u32; 4] {
        self.thresholds
    }

    /// The tier a score has reached: how many of the later thresholds it has met.
    #[must_use]
    pub fn tier(&self, points: f64) -> RenownTier {
        let reached = self.thresholds[1..]
            .iter()
            .filter(|&&threshold| points >= f64::from(threshold))
            .count();
        RenownTier::try_new(u32::try_from(reached).unwrap_or(0))
            .expect("at most three of three later thresholds are reached")
    }

    /// The title the player carries for a pair of tiers.
    #[must_use]
    pub fn title(&self, fame: RenownTier, infamy: RenownTier) -> &str {
        &self.titles[usize::from(infamy.get())][usize::from(fame.get())]
    }

    /// The fame and infamy points that put the player at a standing, the fewest
    /// that do: only the tiers the standing needs are reached.
    #[must_use]
    pub fn points_for(&self, level: ReputationLevel) -> (f64, f64) {
        let at = |tier: usize| f64::from(self.thresholds[tier]);
        match level {
            ReputationLevel::Neutral => (0.0, 0.0),
            ReputationLevel::Accepted => (at(1), 0.0),
            ReputationLevel::Liked => (at(2), 0.0),
            ReputationLevel::Idolized => (at(3), 0.0),
            ReputationLevel::Shunned => (0.0, at(1)),
            ReputationLevel::Vilified => (0.0, at(3)),
            ReputationLevel::Mixed => (at(1), at(1)),
        }
    }
}

/// The standing a pair of tiers amounts to.
///
/// The wiki's 4x4 grid has sixteen titles; rules that only need the broad picture
/// get these seven. Fame alone reads as accepted, liked or idolized by tier;
/// infamy alone reads as shunned (tiers one and two) or vilified (tier three);
/// both together read as mixed. A rule that needs a finer line asks for the tier
/// itself.
#[must_use]
pub fn standing(fame: RenownTier, infamy: RenownTier) -> ReputationLevel {
    match (fame.get(), infamy.get()) {
        (0, 0) => ReputationLevel::Neutral,
        (1, 0) => ReputationLevel::Accepted,
        (2, 0) => ReputationLevel::Liked,
        (_, 0) => ReputationLevel::Idolized,
        (0, 1 | 2) => ReputationLevel::Shunned,
        (0, _) => ReputationLevel::Vilified,
        _ => ReputationLevel::Mixed,
    }
}

/// Every faction that tracks reputation, checked once at load.
#[derive(Debug, Clone, Default)]
pub struct ReputationBook {
    factions: BTreeMap<FactionId, FactionModel>,
}

impl ReputationBook {
    /// Builds the book from the loaded data.
    ///
    /// # Errors
    ///
    /// The same as [`ReputationBook::new`].
    pub fn from_data(data: &GameData) -> Result<Self, ReputationError> {
        Self::new(data.factions.iter())
    }

    /// Builds the book from faction records; a faction without a reputation
    /// record is left out.
    ///
    /// # Errors
    ///
    /// - [`ReputationError::DuplicateFaction`] when an id repeats;
    /// - [`ReputationError::Incomplete`] when a record lacks thresholds or titles;
    /// - [`ReputationError::BadThresholds`] when the thresholds do not start at
    ///   zero and rise.
    pub fn new<'a>(
        records: impl IntoIterator<Item = &'a Faction>,
    ) -> Result<Self, ReputationError> {
        let mut factions = BTreeMap::new();
        let mut seen = std::collections::HashSet::new();
        for record in records {
            if !seen.insert(&record.id) {
                return Err(ReputationError::DuplicateFaction(record.id.clone()));
            }
            let Some(model) = &record.reputation else {
                continue;
            };
            let (Some(thresholds), Some(titles)) = (model.thresholds, model.titles.clone()) else {
                return Err(ReputationError::Incomplete(record.id.clone()));
            };
            if thresholds[0] != 0 || !thresholds.is_sorted_by(|a, b| a < b) {
                return Err(ReputationError::BadThresholds {
                    faction: record.id.clone(),
                    thresholds,
                });
            }
            factions.insert(
                record.id.clone(),
                FactionModel {
                    name: record.name.clone(),
                    thresholds,
                    titles,
                },
            );
        }
        Ok(Self { factions })
    }

    /// The model of a faction, if it tracks reputation.
    #[must_use]
    pub fn get(&self, faction: &FactionId) -> Option<&FactionModel> {
        self.factions.get(faction)
    }

    /// The factions that track reputation, in id order.
    pub fn iter(&self) -> impl Iterator<Item = (&FactionId, &FactionModel)> {
        self.factions.iter()
    }

    /// How many factions track reputation.
    #[must_use]
    pub fn len(&self) -> usize {
        self.factions.len()
    }

    /// True when no faction tracks reputation.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.factions.is_empty()
    }
}
