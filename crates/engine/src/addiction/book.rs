//! Which items can addict, and how likely.

use std::collections::BTreeMap;

use thiserror::Error;

use super::state::{AddictionKind, KindError};
use crate::shared::{GameData, ItemId};

/// Why the book could not be built.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum AddictionError {
    /// An item names an addiction that cannot be a kind.
    #[error("{item}: {source}")]
    BadKind {
        /// The item.
        item: ItemId,
        /// What is wrong with the name.
        source: KindError,
    },
    /// An item's chance is not a number from 0 to 100.
    #[error("{item}: an addiction chance of {chance} is not between 0 and 100")]
    BadChance {
        /// The item.
        item: ItemId,
        /// The chance as written.
        chance: f64,
    },
}

/// What using an item risks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Risk {
    kind: AddictionKind,
    chance: u8,
}

impl Risk {
    /// What the player would become addicted to.
    #[must_use]
    pub fn kind(&self) -> &AddictionKind {
        &self.kind
    }

    /// The chance, in percent, from 0 to 100.
    #[must_use]
    pub fn chance(&self) -> u8 {
        self.chance
    }

    /// Whether a roll of 1 to 100 is addicting: at or below the chance.
    #[must_use]
    pub fn strikes(&self, roll: u8) -> bool {
        roll <= self.chance
    }
}

/// Every item that can addict, checked once at load.
#[derive(Debug, Clone, Default)]
pub struct AddictionBook {
    risks: BTreeMap<ItemId, Risk>,
}

impl AddictionBook {
    /// Builds the book from the loaded data. A consumable whose addiction gives no
    /// chance (the wiki writes "no" or "n/a" for a few) carries no risk.
    ///
    /// # Errors
    ///
    /// [`AddictionError`] naming the item whose kind or chance is not valid.
    pub fn from_data(data: &GameData) -> Result<Self, AddictionError> {
        Self::new(data.consumables.iter().filter_map(|item| {
            let addiction = item.addiction.as_ref()?;
            Some((&item.id, addiction.kind.as_str(), addiction.chance_percent?))
        }))
    }

    /// Builds the book from (item, kind, chance in percent) triples.
    ///
    /// # Errors
    ///
    /// [`AddictionError`] naming the item whose kind or chance is not valid.
    pub fn new<'a>(
        entries: impl IntoIterator<Item = (&'a ItemId, &'a str, f64)>,
    ) -> Result<Self, AddictionError> {
        let mut risks = BTreeMap::new();
        for (item, kind, chance) in entries {
            if !(0.0..=100.0).contains(&chance) {
                return Err(AddictionError::BadChance {
                    item: item.clone(),
                    chance,
                });
            }
            let kind = AddictionKind::try_new(kind).map_err(|source| AddictionError::BadKind {
                item: item.clone(),
                source,
            })?;
            // The range check above keeps the rounded chance inside a byte.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let chance = chance.round() as u8;
            risks.insert(item.clone(), Risk { kind, chance });
        }
        Ok(Self { risks })
    }

    /// What using the item risks, if it can addict.
    #[must_use]
    pub fn risk(&self, item: &ItemId) -> Option<&Risk> {
        self.risks.get(item)
    }

    /// Whether any item addicts to this.
    #[must_use]
    pub fn knows(&self, kind: &AddictionKind) -> bool {
        self.risks.values().any(|risk| &risk.kind == kind)
    }

    /// How many items can addict.
    #[must_use]
    pub fn len(&self) -> usize {
        self.risks.len()
    }

    /// Whether no item can addict.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.risks.is_empty()
    }
}
