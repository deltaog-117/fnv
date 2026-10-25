//! The roster of NPCs the world knows about.

use std::collections::HashSet;

use crate::shared::NpcId;

/// Every NPC that can be acted on, read-only.
///
/// The roster is what lets the state refuse an effect aimed at an NPC that does
/// not exist, instead of recording a fate for a typo.
#[derive(Debug, Default)]
pub struct WorldBook {
    npcs: HashSet<NpcId>,
}

impl WorldBook {
    /// Builds the roster from NPC ids; a repeated id is kept once.
    #[must_use]
    pub fn new<'a>(npcs: impl IntoIterator<Item = &'a NpcId>) -> Self {
        Self {
            npcs: npcs.into_iter().cloned().collect(),
        }
    }

    /// Whether the NPC exists.
    #[must_use]
    pub fn contains(&self, npc: &NpcId) -> bool {
        self.npcs.contains(npc)
    }
}
