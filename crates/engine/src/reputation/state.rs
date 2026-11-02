//! What the player has earned: fame and infamy per faction, and karma.

use std::collections::HashMap;
use std::sync::Arc;

use super::book::{FactionModel, ReputationBook, standing};
use super::karma::Karma;
use crate::shared::{
    Alignment, EffectError, FactionId, Renown, RenownTier, ReputationLevel, StandingSink,
    StandingView,
};

/// The most points a single score holds. It is far above the highest tier
/// threshold (100), and it keeps repeated gains from ever reaching infinity.
pub const MAX_POINTS: f64 = 10_000.0;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Score {
    fame: f64,
    infamy: f64,
}

impl Score {
    fn of(self, renown: Renown) -> f64 {
        match renown {
            Renown::Fame => self.fame,
            Renown::Infamy => self.infamy,
        }
    }
}

/// The reputation scores and the karma.
///
/// Cloning is cheap: the book is shared and only factions that were touched are
/// stored.
#[derive(Debug, Clone)]
pub struct Standing {
    book: Arc<ReputationBook>,
    scores: HashMap<FactionId, Score>,
    karma: Karma,
}

impl Standing {
    /// A new standing over the factions that track reputation: no points, neutral
    /// karma.
    #[must_use]
    pub fn new(book: Arc<ReputationBook>) -> Self {
        Self {
            book,
            scores: HashMap::new(),
            karma: Karma::default(),
        }
    }

    /// A standing over the same factions with no points and neutral karma, for
    /// loading a save.
    #[must_use]
    pub fn emptied(&self) -> Self {
        Self::new(Arc::clone(&self.book))
    }

    /// The player's karma.
    #[must_use]
    pub fn karma(&self) -> Karma {
        self.karma
    }

    /// The factions and towns that track reputation, by id order, with the name the
    /// player sees.
    pub fn factions(&self) -> impl Iterator<Item = (&FactionId, &str)> {
        self.book.iter().map(|(id, model)| (id, model.name()))
    }

    /// The title the player carries with a faction, for a faction that tracks
    /// reputation.
    #[must_use]
    pub fn title(&self, faction: &FactionId) -> Option<&str> {
        let model = self.book.get(faction)?;
        Some(model.title(
            self.renown_tier(faction, Renown::Fame),
            self.renown_tier(faction, Renown::Infamy),
        ))
    }

    fn score(&self, faction: &FactionId) -> Score {
        self.scores.get(faction).copied().unwrap_or_default()
    }

    fn model(&self, faction: &FactionId) -> Result<&FactionModel, EffectError> {
        self.book.get(faction).ok_or_else(|| EffectError::Unknown {
            kind: "faction that tracks reputation",
            id: faction.as_str().to_owned(),
        })
    }
}

impl StandingView for Standing {
    fn reputation(&self, faction: &FactionId) -> ReputationLevel {
        standing(
            self.renown_tier(faction, Renown::Fame),
            self.renown_tier(faction, Renown::Infamy),
        )
    }

    fn renown_tier(&self, faction: &FactionId, renown: Renown) -> RenownTier {
        self.book
            .get(faction)
            .map_or_else(RenownTier::default, |model| {
                model.tier(self.score(faction).of(renown))
            })
    }

    fn renown_points(&self, faction: &FactionId, renown: Renown) -> f64 {
        self.score(faction).of(renown)
    }

    fn alignment(&self) -> Alignment {
        self.karma.alignment()
    }
}

impl StandingSink for Standing {
    fn add_reputation(
        &mut self,
        faction: &FactionId,
        fame: f64,
        infamy: f64,
    ) -> Result<(), EffectError> {
        self.model(faction)?;
        if !(fame.is_finite() && infamy.is_finite()) {
            return Err(EffectError::NotFinite {
                effect: "reputation",
            });
        }
        if fame < 0.0 || infamy < 0.0 {
            return Err(EffectError::Refused {
                effect: "reputation",
                reason: format!(
                    "{}: fame and infamy only grow, got {fame} and {infamy}",
                    faction.as_str()
                ),
            });
        }
        let score = self.scores.entry(faction.clone()).or_default();
        score.fame = (score.fame + fame).min(MAX_POINTS);
        score.infamy = (score.infamy + infamy).min(MAX_POINTS);
        Ok(())
    }

    fn set_reputation(
        &mut self,
        faction: &FactionId,
        level: ReputationLevel,
    ) -> Result<(), EffectError> {
        let (fame, infamy) = self.model(faction)?.points_for(level);
        self.scores.insert(faction.clone(), Score { fame, infamy });
        Ok(())
    }

    fn add_karma(&mut self, amount: f64) -> Result<(), EffectError> {
        if !amount.is_finite() {
            return Err(EffectError::NotFinite { effect: "karma" });
        }
        self.karma = self.karma.shifted(amount);
        Ok(())
    }
}
