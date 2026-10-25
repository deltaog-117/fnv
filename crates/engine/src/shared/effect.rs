//! The `Effect` vocabulary: changes to game state.
//!
//! Quest rules and dialogue choices use these to say what happens. Applying one
//! is the job of the feature that owns the state it changes; here they are only
//! data that can be built, parsed and written back without ever being invalid.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use super::ids::{CompanionId, EndingId, FactionId, FlagId, ItemId, NpcId, QuestId};
use super::vocabulary::{QuestStage, ReputationLevel};

fn default_count() -> NonZeroU32 {
    NonZeroU32::MIN
}

fn default_true() -> bool {
    true
}

/// A change to game state.
///
/// Serialized as an object with a `type` field, matching `$defs/effect` in
/// `schemas/common.schema.json`. Unknown fields are rejected, as the schema
/// rejects them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Effect {
    /// Starts a quest.
    StartQuest {
        /// The quest.
        quest: QuestId,
    },
    /// Completes a quest.
    CompleteQuest {
        /// The quest.
        quest: QuestId,
    },
    /// Fails a quest.
    FailQuest {
        /// The quest.
        quest: QuestId,
    },
    /// Locks a quest out so it can never start.
    BlockQuest {
        /// The quest.
        quest: QuestId,
    },
    /// Moves a quest to a stage.
    SetStage {
        /// The quest.
        quest: QuestId,
        /// The stage to move to.
        stage: QuestStage,
    },
    /// Adds fame and infamy points with a faction.
    Reputation {
        /// The faction or town.
        faction: FactionId,
        /// Fame points to add.
        fame: f64,
        /// Infamy points to add.
        infamy: f64,
    },
    /// Forces the standing with a faction to a level.
    SetReputation {
        /// The faction or town.
        faction: FactionId,
        /// The standing to set.
        level: ReputationLevel,
    },
    /// Changes karma.
    Karma {
        /// Points to add (negative to subtract).
        amount: f64,
    },
    /// Grants experience points.
    Xp {
        /// Points to add.
        amount: f64,
    },
    /// Changes the number of bottle caps.
    Caps {
        /// Caps to add (negative to subtract).
        amount: f64,
    },
    /// Gives the player items.
    GiveItem {
        /// The item.
        item: ItemId,
        /// How many; defaults to one.
        #[serde(default = "default_count")]
        count: NonZeroU32,
    },
    /// Takes items from the player.
    TakeItem {
        /// The item.
        item: ItemId,
        /// How many; defaults to one.
        #[serde(default = "default_count")]
        count: NonZeroU32,
    },
    /// Sets a story flag.
    SetFlag {
        /// The flag.
        flag: FlagId,
        /// The value to set; defaults to set.
        #[serde(default = "default_true")]
        value: bool,
    },
    /// A companion joins the player.
    CompanionJoin {
        /// The companion.
        companion: CompanionId,
    },
    /// A companion leaves the player.
    CompanionLeave {
        /// The companion.
        companion: CompanionId,
    },
    /// Kills a non-player character.
    KillNpc {
        /// The NPC.
        npc: NpcId,
    },
    /// Makes a non-player character hostile.
    SetHostile {
        /// The NPC.
        npc: NpcId,
    },
    /// Makes a non-player character friendly.
    SetFriendly {
        /// The NPC.
        npc: NpcId,
    },
    /// Removes a non-player character from the world.
    RemoveNpc {
        /// The NPC.
        npc: NpcId,
    },
    /// Makes an ending reachable.
    UnlockEnding {
        /// The ending.
        ending: EndingId,
    },
    /// Makes an ending unreachable.
    LockEnding {
        /// The ending.
        ending: EndingId,
    },
}

impl Effect {
    /// Every `type` value an effect can have, in schema order.
    pub const ALL_TAGS: [&'static str; 21] = [
        "start_quest",
        "complete_quest",
        "fail_quest",
        "block_quest",
        "set_stage",
        "reputation",
        "set_reputation",
        "karma",
        "xp",
        "caps",
        "give_item",
        "take_item",
        "set_flag",
        "companion_join",
        "companion_leave",
        "kill_npc",
        "set_hostile",
        "set_friendly",
        "remove_npc",
        "unlock_ending",
        "lock_ending",
    ];

    /// The `type` value of this effect. The match is exhaustive, so adding a
    /// variant without naming it here does not compile.
    #[must_use]
    pub fn tag(&self) -> &'static str {
        match self {
            Self::StartQuest { .. } => "start_quest",
            Self::CompleteQuest { .. } => "complete_quest",
            Self::FailQuest { .. } => "fail_quest",
            Self::BlockQuest { .. } => "block_quest",
            Self::SetStage { .. } => "set_stage",
            Self::Reputation { .. } => "reputation",
            Self::SetReputation { .. } => "set_reputation",
            Self::Karma { .. } => "karma",
            Self::Xp { .. } => "xp",
            Self::Caps { .. } => "caps",
            Self::GiveItem { .. } => "give_item",
            Self::TakeItem { .. } => "take_item",
            Self::SetFlag { .. } => "set_flag",
            Self::CompanionJoin { .. } => "companion_join",
            Self::CompanionLeave { .. } => "companion_leave",
            Self::KillNpc { .. } => "kill_npc",
            Self::SetHostile { .. } => "set_hostile",
            Self::SetFriendly { .. } => "set_friendly",
            Self::RemoveNpc { .. } => "remove_npc",
            Self::UnlockEnding { .. } => "unlock_ending",
            Self::LockEnding { .. } => "lock_ending",
        }
    }
}
