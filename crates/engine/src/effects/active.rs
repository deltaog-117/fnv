//! The boosts in force.

use crate::shared::{Boost, BoostStat, BoostTally, ItemId};

/// One boost in force: what it changes, by how much, which item started it and the
/// minute it ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Active {
    /// The item that started it.
    pub source: ItemId,
    /// What it changes.
    pub stat: BoostStat,
    /// By how much; negative for a penalty.
    pub amount: i16,
    /// The minute on the game clock at which it ends: it is gone at that minute.
    pub until: u64,
}

/// Every boost in force, in the order started.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActiveEffects {
    list: Vec<Active>,
}

impl ActiveEffects {
    /// Starts the boosts of one dose of `source` at minute `now`, replacing any
    /// earlier dose of the same item. Each lasts its own length, counted from
    /// `now`; the end saturates, so no length can wrap the clock around.
    pub fn start(&mut self, source: &ItemId, boosts: &[Boost], now: u64) {
        self.list.retain(|active| &active.source != source);
        self.list.extend(boosts.iter().map(|boost| Active {
            source: source.clone(),
            stat: boost.stat,
            amount: boost.amount.get(),
            until: now.saturating_add(u64::from(boost.minutes.get())),
        }));
    }

    /// The boosts of a save, in the order they were started.
    #[must_use]
    pub fn from_entries(list: Vec<Active>) -> Self {
        Self { list }
    }

    /// Ends every boost whose time has come at minute `now`, and says whether any
    /// ended.
    pub fn expire(&mut self, now: u64) -> bool {
        let before = self.list.len();
        self.list.retain(|active| active.until > now);
        self.list.len() != before
    }

    /// The sum of what is in force.
    #[must_use]
    pub fn tally(&self) -> BoostTally {
        let mut tally = BoostTally::default();
        for active in &self.list {
            tally.add(active.stat, active.amount);
        }
        tally
    }

    /// Everything in force, in the order started.
    pub fn iter(&self) -> impl Iterator<Item = &Active> {
        self.list.iter()
    }

    /// How many boosts are in force.
    #[must_use]
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Whether nothing is in force.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}
