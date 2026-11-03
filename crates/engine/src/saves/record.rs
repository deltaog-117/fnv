//! The save as data: plain numbers, ids and lists, with nothing borrowed from the
//! features.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::error::SaveError;
use crate::shared::{
    BoostStat, Effect, EncounterId, EndingId, FactionId, FindId, FlagId, ItemId, LocationId, NpcId,
    PerkId, QuestId, QuestStage, Skill, Special, TraitId, VendorId,
};

/// The newest version this game reads and the only one it writes.
pub const VERSION: u32 = 2;

/// Everything that changes during play.
///
/// Lists are sorted, so saving the same game twice gives the same text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Save {
    /// Which version of the record this is.
    pub version: u32,
    /// Minutes on the game clock.
    pub clock_minutes: u64,
    /// Where the player stands, once the game has a map.
    pub at: Option<LocationId>,
    /// Hit points lost and not yet healed.
    pub wounds: u32,
    /// Every quest that was started or shut out.
    pub quests: Vec<SavedQuest>,
    /// Flags, NPC fates and endings.
    pub world: SavedWorld,
    /// Fame, infamy and karma.
    pub standing: SavedStanding,
    /// The player character.
    pub character: SavedCharacter,
    /// What the player carries and wears.
    pub pack: SavedPack,
    /// What each merchant has now.
    pub shops: Vec<SavedShop>,
    /// The boosts in force.
    pub effects: Vec<SavedEffect>,
    /// How worn each weapon is.
    pub wear: Vec<SavedWear>,
    /// The finds the player has already picked up.
    pub gathered: Vec<FindId>,
    /// Fights the player has won.
    pub won: Vec<EncounterId>,
    /// Consequence rules that already fired.
    pub fired: Vec<SavedFired>,
    /// Quest effects postponed until their feature is built.
    pub deferred: Vec<SavedDeferred>,
}

/// Where a started quest stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SavedPhase {
    /// In progress.
    Active,
    /// Done.
    Completed,
    /// Failed or shut out.
    Failed,
}

/// One quest's progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedQuest {
    /// The quest.
    pub quest: QuestId,
    /// Where it stands.
    pub status: SavedPhase,
    /// The stage reached, if any.
    pub stage: Option<QuestStage>,
}

/// What became of an NPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SavedFate {
    /// Alive and friendly.
    Friendly,
    /// Alive and hostile.
    Hostile,
    /// Dead.
    Dead,
    /// Gone without a body.
    Removed,
}

/// An NPC the story has touched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedNpc {
    /// The NPC.
    pub npc: NpcId,
    /// What became of them.
    pub fate: SavedFate,
    /// Whether the player has met them.
    pub met: bool,
}

/// An ending that was unlocked or locked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedEnding {
    /// The ending.
    pub ending: EndingId,
    /// Whether it is reachable.
    pub open: bool,
}

/// The story so far.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedWorld {
    /// The flags that are set.
    pub flags: Vec<FlagId>,
    /// The NPCs the story has touched.
    pub npcs: Vec<SavedNpc>,
    /// The endings unlocked or locked.
    pub endings: Vec<SavedEnding>,
}

/// One faction's fame and infamy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedScore {
    /// The faction or town.
    pub faction: FactionId,
    /// Fame points.
    pub fame: f64,
    /// Infamy points.
    pub infamy: f64,
}

/// Reputation and karma.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedStanding {
    /// Karma, from -1000 to 1000.
    pub karma: f64,
    /// Every faction with points; one with none is left out.
    pub factions: Vec<SavedScore>,
}

/// The player character.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedCharacter {
    /// The seven attributes; all of them must be present.
    pub special: BTreeMap<Special, u8>,
    /// Total experience.
    pub xp: u32,
    /// Points spent on each skill; a skill with none is left out.
    pub spent: BTreeMap<Skill, u8>,
    /// The tagged skills.
    pub tagged: Vec<Skill>,
    /// The traits chosen.
    pub traits: Vec<TraitId>,
    /// The perks taken, each with its rank.
    pub perks: BTreeMap<PerkId, u8>,
    /// Skill points banked, in half points.
    pub skill_halves: u32,
    /// Perks waiting to be chosen.
    pub perk_picks: u32,
}

/// What the player carries and wears.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedPack {
    /// Bottle caps.
    pub caps: u32,
    /// Item stacks.
    pub items: BTreeMap<ItemId, u32>,
    /// The wielded weapon.
    pub weapon: Option<ItemId>,
    /// The outfit worn.
    pub outfit: Option<ItemId>,
    /// The headgear worn.
    pub head: Option<ItemId>,
}

/// One merchant's shop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedShop {
    /// The merchant.
    pub vendor: VendorId,
    /// Caps they hold.
    pub caps: u32,
    /// Their stock.
    pub stock: BTreeMap<ItemId, u32>,
}

/// One boost in force.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedEffect {
    /// The item that started it.
    pub source: ItemId,
    /// What it changes.
    pub stat: BoostStat,
    /// By how much.
    pub amount: i16,
    /// The minute it ends.
    pub until: u64,
}

/// A weapon's wear.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedWear {
    /// The weapon.
    pub item: ItemId,
    /// Condition points lost.
    pub lost: u32,
}

/// A consequence rule that already fired.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedFired {
    /// The quest the rule belongs to.
    pub quest: QuestId,
    /// The rule's place in that quest's list.
    pub rule: usize,
}

/// A quest effect waiting for a feature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedDeferred {
    /// The quest that produced it.
    pub quest: QuestId,
    /// The effect.
    pub effect: Effect,
    /// The missing feature.
    pub feature: String,
}

impl Save {
    /// The save as JSON text.
    ///
    /// # Errors
    ///
    /// [`SaveError::Format`] when a number cannot be written, which a game never
    /// produces.
    pub fn to_json(&self) -> Result<String, SaveError> {
        serde_json::to_string_pretty(self).map_err(|error| SaveError::Format(error.to_string()))
    }

    /// Reads a save from JSON text, upgrading an older version first.
    ///
    /// # Errors
    ///
    /// [`SaveError::Format`] for text that is not a save, [`SaveError::TooNew`] for a
    /// file from a newer game and [`SaveError::UnknownVersion`] for a version that
    /// never existed.
    pub fn from_json(text: &str) -> Result<Self, SaveError> {
        let mut value: serde_json::Value =
            serde_json::from_str(text).map_err(|error| SaveError::Format(error.to_string()))?;
        let found = value
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| SaveError::Format("the save has no version number".to_owned()))?;
        if found > u64::from(VERSION) {
            return Err(SaveError::TooNew {
                found,
                supported: VERSION,
            });
        }
        let mut version = found;
        while version < u64::from(VERSION) {
            value = upgrade(value, version)?;
            version += 1;
        }
        serde_json::from_value(value).map_err(|error| SaveError::Format(error.to_string()))
    }
}

/// Turns a save of `from` into one of `from + 1`.
fn upgrade(mut value: serde_json::Value, from: u64) -> Result<serde_json::Value, SaveError> {
    match from {
        // Version 2 added the finds taken; a version 1 game had none to take.
        1 => {
            let object = value
                .as_object_mut()
                .ok_or_else(|| SaveError::Format("the save is not an object".to_owned()))?;
            object.insert("gathered".to_owned(), serde_json::Value::Array(Vec::new()));
            object.insert("version".to_owned(), serde_json::Value::from(2));
            Ok(value)
        }
        _ => Err(SaveError::UnknownVersion(from)),
    }
}
