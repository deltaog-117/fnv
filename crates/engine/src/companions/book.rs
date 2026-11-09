//! The roster of companions, checked once at load.

use std::collections::BTreeMap;

use thiserror::Error;

use crate::shared::{Companion, CompanionId, GameData, NpcId, PerkId};

/// Why the companion data could not be accepted.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CompanionError {
    /// A loadout carried a `when`, which a companion never needs.
    #[error("companion {0}: a loadout may not carry conditions")]
    LoadoutHasConditions(CompanionId),
    /// The companion's NPC is not in the loaded data.
    #[error("companion {companion}: unknown npc {npc}")]
    UnknownNpc {
        /// The companion.
        companion: CompanionId,
        /// The missing NPC.
        npc: NpcId,
    },
    /// The perk the companion grants is not in the loaded data.
    #[error("companion {companion}: unknown perk {perk}")]
    UnknownPerk {
        /// The companion.
        companion: CompanionId,
        /// The missing perk.
        perk: PerkId,
    },
}

/// Every companion, read-only.
#[derive(Debug, Default)]
pub struct CompanionBook {
    records: BTreeMap<CompanionId, Companion>,
}

impl CompanionBook {
    /// Builds the roster from the loaded data.
    ///
    /// # Errors
    ///
    /// [`CompanionError`] naming the companion at fault.
    pub fn from_data(data: &GameData) -> Result<Self, CompanionError> {
        let mut records = BTreeMap::new();
        for companion in data.companions.iter() {
            if !companion.loadout.when.is_empty() {
                return Err(CompanionError::LoadoutHasConditions(companion.id.clone()));
            }
            if !data.npcs.contains(&companion.loadout.npc) {
                return Err(CompanionError::UnknownNpc {
                    companion: companion.id.clone(),
                    npc: companion.loadout.npc.clone(),
                });
            }
            if let Some(perk) = companion
                .perk
                .as_ref()
                .filter(|perk| !data.perks.contains(perk))
            {
                return Err(CompanionError::UnknownPerk {
                    companion: companion.id.clone(),
                    perk: perk.clone(),
                });
            }
            records.insert(companion.id.clone(), companion.clone());
        }
        Ok(Self { records })
    }

    /// The companion with this id.
    #[must_use]
    pub fn get(&self, id: &CompanionId) -> Option<&Companion> {
        self.records.get(id)
    }

    /// Every companion, ordered by id.
    pub fn iter(&self) -> impl Iterator<Item = &Companion> {
        self.records.values()
    }

    /// The companion a given NPC is, when they are one.
    #[must_use]
    pub fn of_npc(&self, npc: &NpcId) -> Option<&CompanionId> {
        self.records
            .values()
            .find(|companion| &companion.loadout.npc == npc)
            .map(|companion| &companion.id)
    }
}
