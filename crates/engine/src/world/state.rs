//! What has happened in the world: story flags, NPC fates and endings.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::book::WorldBook;
use crate::shared::{EffectError, EndingId, FlagId, NpcId, NpcState, WorldSink, WorldView};

/// Where an NPC stands. Hostility only exists on a living NPC, so a dead hostile
/// NPC cannot be represented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Alive { hostile: bool },
    Dead,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Npc {
    status: Status,
    met: bool,
}

/// An NPC nobody has touched: alive, friendly, never met.
const UNTOUCHED: Npc = Npc {
    status: Status::Alive { hostile: false },
    met: false,
};

/// The story flags, the NPC fates and the ending locks.
///
/// Cloning is cheap: the roster is shared and only NPCs that changed are stored.
#[derive(Debug, Clone)]
pub struct WorldState {
    book: Arc<WorldBook>,
    flags: HashSet<FlagId>,
    npcs: HashMap<NpcId, Npc>,
    endings: HashMap<EndingId, bool>,
}

impl WorldState {
    /// A new world over the roster: no flag set, every NPC alive.
    #[must_use]
    pub fn new(book: Arc<WorldBook>) -> Self {
        Self {
            book,
            flags: HashSet::new(),
            npcs: HashMap::new(),
            endings: HashMap::new(),
        }
    }

    /// Records that the player has met the NPC.
    ///
    /// # Errors
    ///
    /// [`EffectError`] when the NPC is not in the roster, or is dead or removed:
    /// nobody can be met after that.
    pub fn meet(&mut self, npc: &NpcId) -> Result<(), EffectError> {
        self.update("meet", npc, |current| match current.status {
            Status::Alive { .. } => Ok(Npc {
                met: true,
                ..current
            }),
            Status::Dead => Err("already dead"),
            Status::Removed => Err("already removed"),
        })
    }

    /// The last word on an ending: `Some(true)` unlocked, `Some(false)` locked,
    /// `None` never touched. What an untouched ending means is for the ending
    /// feature to decide.
    #[must_use]
    pub fn ending_reachable(&self, ending: &EndingId) -> Option<bool> {
        self.endings.get(ending).copied()
    }

    fn record(&self, npc: &NpcId) -> Option<Npc> {
        self.npcs
            .get(npc)
            .copied()
            .or_else(|| self.book.contains(npc).then_some(UNTOUCHED))
    }

    fn update(
        &mut self,
        effect: &'static str,
        npc: &NpcId,
        step: impl FnOnce(Npc) -> Result<Npc, &'static str>,
    ) -> Result<(), EffectError> {
        let current = self.record(npc).ok_or_else(|| EffectError::Unknown {
            kind: "npc",
            id: npc.as_str().to_owned(),
        })?;
        let next = step(current).map_err(|reason| EffectError::Refused {
            effect,
            reason: format!("{}: {reason}", npc.as_str()),
        })?;
        self.npcs.insert(npc.clone(), next);
        Ok(())
    }

    /// Turns a living NPC hostile or friendly.
    fn set_attitude(
        &mut self,
        effect: &'static str,
        npc: &NpcId,
        hostile: bool,
    ) -> Result<(), EffectError> {
        self.update(effect, npc, |current| match current.status {
            Status::Alive { .. } => Ok(Npc {
                status: Status::Alive { hostile },
                ..current
            }),
            Status::Dead => Err("already dead"),
            Status::Removed => Err("already removed"),
        })
    }
}

impl WorldView for WorldState {
    fn flag(&self, flag: &FlagId) -> bool {
        self.flags.contains(flag)
    }

    /// An NPC outside the roster is in no state. A removed NPC is in none either:
    /// it is gone, not dead. A dead NPC can still have been met.
    fn npc_is(&self, npc: &NpcId, state: NpcState) -> bool {
        let Some(npc) = self.record(npc) else {
            return false;
        };
        match (state, npc.status) {
            (NpcState::Alive, Status::Alive { .. })
            | (NpcState::Dead, Status::Dead)
            | (NpcState::Hostile, Status::Alive { hostile: true }) => true,
            (NpcState::Met, Status::Alive { .. } | Status::Dead) => npc.met,
            _ => false,
        }
    }
}

impl WorldSink for WorldState {
    fn set_flag(&mut self, flag: &FlagId, value: bool) -> Result<(), EffectError> {
        if value {
            self.flags.insert(flag.clone());
        } else {
            self.flags.remove(flag);
        }
        Ok(())
    }

    /// Killing the dead changes nothing; a removed NPC has no body to kill.
    fn kill_npc(&mut self, npc: &NpcId) -> Result<(), EffectError> {
        self.update("kill_npc", npc, |current| match current.status {
            Status::Alive { .. } | Status::Dead => Ok(Npc {
                status: Status::Dead,
                ..current
            }),
            Status::Removed => Err("already removed"),
        })
    }

    fn set_hostile(&mut self, npc: &NpcId) -> Result<(), EffectError> {
        self.set_attitude("set_hostile", npc, true)
    }

    fn set_friendly(&mut self, npc: &NpcId) -> Result<(), EffectError> {
        self.set_attitude("set_friendly", npc, false)
    }

    /// Removing a dead NPC clears the body; removing a removed NPC changes
    /// nothing.
    fn remove_npc(&mut self, npc: &NpcId) -> Result<(), EffectError> {
        self.update("remove_npc", npc, |current| {
            Ok(Npc {
                status: Status::Removed,
                ..current
            })
        })
    }

    /// Endings have no catalog yet, so any id is accepted.
    fn unlock_ending(&mut self, ending: &EndingId) -> Result<(), EffectError> {
        self.endings.insert(ending.clone(), true);
        Ok(())
    }

    fn lock_ending(&mut self, ending: &EndingId) -> Result<(), EffectError> {
        self.endings.insert(ending.clone(), false);
        Ok(())
    }
}
