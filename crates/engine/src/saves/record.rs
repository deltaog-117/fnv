//! The save as data: plain numbers, ids and lists, with nothing borrowed from the
//! features.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::error::SaveError;
use crate::shared::{
    BoostStat, CompanionId, Effect, EncounterId, EndingId, FactionId, FindId, FlagId, ItemId,
    LocationId, NpcId, PerkId, QuestId, QuestStage, Skill, Special, TraitId, VendorId,
};

/// The newest version this game reads and the only one it writes.
pub const VERSION: u32 = 6;

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
    /// Radiation absorbed, in rads.
    pub rads: u16,
    /// What the player is addicted to, by name.
    pub addictions: Vec<SavedAddiction>,
    /// Poison in the blood, in hit points still to be lost.
    pub poison: u16,
    /// The companions in the party, in the order they joined.
    pub party: Vec<CompanionId>,
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
    /// Item stacks. A weapon listed here is that many new copies, which is how a
    /// version 2 save is read.
    pub items: BTreeMap<ItemId, u32>,
    /// The weapons carried one by one, each with its own wear.
    pub weapons: Vec<SavedWeapon>,
    /// The number of the weapon in hand.
    pub wielded: Option<u32>,
    /// The kind of weapon in hand, for a save that does not say which copy
    /// (version 2). A game writes `wielded` instead.
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

/// One addiction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedAddiction {
    /// What the player is addicted to.
    pub kind: String,
    /// The minute of the last dose.
    pub last_dose: u64,
}

/// One carried weapon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedWeapon {
    /// Its number in the pack.
    pub id: u32,
    /// What kind of weapon it is.
    pub item: ItemId,
    /// Condition points lost.
    pub lost: u32,
    /// The weapon mods fitted to it.
    pub mods: Vec<ItemId>,
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
        // Version 3 gave every weapon its own wear. The old per-kind wear becomes the
        // wear of each copy of that kind; a weapon with no record was new and stays
        // a plain count, which loads as new copies.
        2 => {
            let format = |what: &str| SaveError::Format(what.to_owned());
            let object = value
                .as_object_mut()
                .ok_or_else(|| format("the save is not an object"))?;
            let wear = object.remove("wear").unwrap_or_default();
            let pack = object
                .get_mut("pack")
                .and_then(serde_json::Value::as_object_mut)
                .ok_or_else(|| format("the save has no pack"))?;
            let mut weapons = Vec::new();
            let mut first: std::collections::BTreeMap<String, u64> = Default::default();
            let mut entries: Vec<(String, u64)> = wear
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|entry| {
                    Some((
                        entry.get("item")?.as_str()?.to_owned(),
                        entry.get("lost")?.as_u64()?,
                    ))
                })
                .collect();
            entries.sort();
            let mut next = 0u64;
            for (item, lost) in entries {
                let count = pack
                    .get_mut("items")
                    .and_then(serde_json::Value::as_object_mut)
                    .and_then(|items| items.remove(&item))
                    .and_then(|count| count.as_u64())
                    .unwrap_or(0);
                for _ in 0..count {
                    first.entry(item.clone()).or_insert(next);
                    weapons.push(serde_json::json!({"id": next, "item": item, "lost": lost}));
                    next += 1;
                }
            }
            let in_hand = pack
                .get("weapon")
                .and_then(serde_json::Value::as_str)
                .and_then(|kind| first.get(kind).copied());
            if in_hand.is_some() {
                pack.insert("weapon".to_owned(), serde_json::Value::Null);
            }
            pack.insert("weapons".to_owned(), serde_json::Value::Array(weapons));
            pack.insert("wielded".to_owned(), serde_json::json!(in_hand));
            object.insert("version".to_owned(), serde_json::Value::from(3));
            Ok(value)
        }
        // Version 4 added radiation and addiction; an older game had neither, so the
        // player starts clean.
        3 => {
            let object = value
                .as_object_mut()
                .ok_or_else(|| SaveError::Format("the save is not an object".to_owned()))?;
            object.insert("rads".to_owned(), serde_json::Value::from(0));
            object.insert(
                "addictions".to_owned(),
                serde_json::Value::Array(Vec::new()),
            );
            object.insert("version".to_owned(), serde_json::Value::from(4));
            Ok(value)
        }
        // Version 5 added weapon mods and poison; an older game had no mods fitted and
        // no poison in the blood.
        4 => {
            let format = |what: &str| SaveError::Format(what.to_owned());
            let object = value
                .as_object_mut()
                .ok_or_else(|| format("the save is not an object"))?;
            object.insert("poison".to_owned(), serde_json::Value::from(0));
            let weapons = object
                .get_mut("pack")
                .and_then(|pack| pack.get_mut("weapons"))
                .and_then(serde_json::Value::as_array_mut)
                .ok_or_else(|| format("the save has no list of weapons"))?;
            for weapon in weapons {
                weapon
                    .as_object_mut()
                    .ok_or_else(|| format("a weapon is not an object"))?
                    .insert("mods".to_owned(), serde_json::Value::Array(Vec::new()));
            }
            object.insert("version".to_owned(), serde_json::Value::from(5));
            Ok(value)
        }
        // Version 6 added the party; an older game had nobody travelling with the player.
        5 => {
            let object = value
                .as_object_mut()
                .ok_or_else(|| SaveError::Format("the save is not an object".to_owned()))?;
            object.insert("party".to_owned(), serde_json::Value::Array(Vec::new()));
            object.insert("version".to_owned(), serde_json::Value::from(6));
            Ok(value)
        }
        _ => Err(SaveError::UnknownVersion(from)),
    }
}
