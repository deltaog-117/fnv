//! The boosts in force.

use crate::shared::{Boost, BoostStat, BoostTally, ItemId};

/// What started a boost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoostSource {
    /// A dose of this item.
    Item(ItemId),
    /// A perk's reward for a fight survived on little health.
    AfterFight,
}

/// One boost in force: what it changes, by how much, what started it and the
/// minute it ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Active {
    /// What started it.
    pub source: BoostSource,
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
        self.list
            .retain(|active| active.source != BoostSource::Item(source.clone()));
        self.list.extend(boosts.iter().map(|boost| Active {
            source: BoostSource::Item(source.clone()),
            stat: boost.stat,
            amount: boost.amount.get(),
            until: now.saturating_add(u64::from(boost.minutes.get())),
        }));
    }

    /// Starts a perk's reward for a fight survived on little health, replacing an
    /// earlier reward to the same stat so the same fight twice does not stack.
    pub fn start_after_fight(&mut self, boost: &Boost, now: u64) {
        self.list.retain(|active| {
            !(active.source == BoostSource::AfterFight && active.stat == boost.stat)
        });
        self.list.push(Active {
            source: BoostSource::AfterFight,
            stat: boost.stat,
            amount: boost.amount.get(),
            until: now.saturating_add(u64::from(boost.minutes.get())),
        });
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
