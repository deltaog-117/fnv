//! The data loader: reads `data/engine/` into typed catalogs.
//!
//! Every record is parsed on its own, so an error names the file and the record
//! id. Cross-references between files are checked by `scripts/validate.py`;
//! the loader checks what it needs to build the catalogs and what it can see
//! inside one file (duplicate ids, dialogue topic links).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde_json::Value;
use thiserror::Error;

use super::data::{
    Ammo, Armor, Consumable, Dialogue, Encounter, Faction, Location, Npc, Quest, Weapon,
};
use super::error::ModelError;
use super::ids::{DialogueId, EncounterId, FactionId, ItemId, LocationId, NpcId, QuestId};

/// A record that carries its own unique id.
pub trait Record {
    /// The id type of this record.
    type Id: Ord + Clone + fmt::Display;

    /// The record's id.
    fn id(&self) -> &Self::Id;
}

macro_rules! impl_record {
    ($($ty:ident => $id:ident),* $(,)?) => {
        $(impl Record for $ty {
            type Id = $id;

            fn id(&self) -> &Self::Id {
                &self.id
            }
        })*
    };
}

impl_record!(
    Weapon => ItemId,
    Armor => ItemId,
    Ammo => ItemId,
    Consumable => ItemId,
    Quest => QuestId,
    Npc => NpcId,
    Location => LocationId,
    Faction => FactionId,
    Dialogue => DialogueId,
    Encounter => EncounterId,
);

/// Why the data could not be loaded.
#[derive(Debug, Error)]
pub enum LoadError {
    /// A file could not be read.
    #[error("cannot read {}: {source}", path.display())]
    Io {
        /// The file or directory.
        path: PathBuf,
        /// What the operating system said.
        source: std::io::Error,
    },
    /// A file is not valid JSON, or not the expected top-level shape.
    #[error("{} is not valid JSON of the expected shape: {source}", path.display())]
    Json {
        /// The file.
        path: PathBuf,
        /// What the parser said, with line and column.
        source: serde_json::Error,
    },
    /// One record does not match its type.
    #[error("{}: record {id}: {source}", path.display())]
    Record {
        /// The file.
        path: PathBuf,
        /// The record's id, or `#<position>` when it has none.
        id: String,
        /// What was wrong with it.
        source: serde_json::Error,
    },
    /// Two records in one file share an id.
    #[error("{}: id {id} appears twice", path.display())]
    DuplicateId {
        /// The file.
        path: PathBuf,
        /// The repeated id.
        id: String,
    },
    /// A dialogue file's id differs from its file name.
    #[error("{}: file holds dialogue id {found}, expected {expected}", path.display())]
    DialogueIdMismatch {
        /// The file.
        path: PathBuf,
        /// The id inside the file.
        found: DialogueId,
        /// The id the file name implies.
        expected: DialogueId,
    },
    /// Two topics in one dialogue file share a key.
    #[error("{}: topic key {key} appears twice", path.display())]
    DuplicateTopic {
        /// The file.
        path: PathBuf,
        /// The repeated key.
        key: String,
    },
    /// A topic lists a follow-up that is not a topic of the same file.
    #[error("{}: topic {topic} lists next {target}, which is not a topic of this file", path.display())]
    DanglingTopic {
        /// The file.
        path: PathBuf,
        /// The topic with the bad link.
        topic: String,
        /// The missing target.
        target: String,
    },
    /// A dialogue file lists a start topic that is not a topic of the same file.
    #[error("{}: start lists {target}, which is not a topic of this file", path.display())]
    DanglingStart {
        /// The file.
        path: PathBuf,
        /// The missing target.
        target: String,
    },
    /// A dialogue file name is not a valid id.
    #[error("{}: file name is not a valid dialogue id: {source}", path.display())]
    BadFileName {
        /// The file.
        path: PathBuf,
        /// Why the name was rejected.
        source: ModelError,
    },
    /// The scope names a location that does not exist.
    #[error("scope root {0} is not a location in locations.json")]
    UnknownScopeRoot(LocationId),
}

/// An id-indexed set of records, with one record per id.
#[derive(Debug, Clone)]
pub struct Catalog<T: Record> {
    records: BTreeMap<T::Id, T>,
}

impl<T: Record> Catalog<T> {
    /// Indexes `records` by id.
    ///
    /// # Errors
    ///
    /// Returns [`LoadError::DuplicateId`] naming `path` when two records share an id.
    fn from_records(path: &Path, records: Vec<T>) -> Result<Self, LoadError> {
        let mut indexed = BTreeMap::new();
        for record in records {
            let id = record.id().clone();
            if indexed.insert(id.clone(), record).is_some() {
                return Err(LoadError::DuplicateId {
                    path: path.to_owned(),
                    id: id.to_string(),
                });
            }
        }
        Ok(Self { records: indexed })
    }

    /// The record with this id.
    #[must_use]
    pub fn get(&self, id: &T::Id) -> Option<&T> {
        self.records.get(id)
    }

    /// True when a record has this id.
    #[must_use]
    pub fn contains(&self, id: &T::Id) -> bool {
        self.records.contains_key(id)
    }

    /// The records, ordered by id.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.records.values()
    }

    /// How many records there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// True when there are no records.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

/// How much of the world to load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// Everything: every quest, NPC and dialogue file.
    Everywhere,
    /// A location, the places inside it, and the quests, NPCs and dialogue that
    /// belong there. Items, factions and locations always load in full.
    Around(LocationId),
}

/// The loaded content.
#[derive(Debug, Clone)]
pub struct GameData {
    /// Every location on every map.
    pub locations: Catalog<Location>,
    /// Every faction and town.
    pub factions: Catalog<Faction>,
    /// Every weapon.
    pub weapons: Catalog<Weapon>,
    /// Every piece of armor and clothing.
    pub armor: Catalog<Armor>,
    /// Every ammunition type.
    pub ammo: Catalog<Ammo>,
    /// Every food, drink, chem and medicine.
    pub consumables: Catalog<Consumable>,
    /// The quests in scope.
    pub quests: Catalog<Quest>,
    /// The NPCs in scope.
    pub npcs: Catalog<Npc>,
    /// The dialogue files in scope.
    pub dialogue: Catalog<Dialogue>,
    /// Every fight the player can start.
    pub encounters: Catalog<Encounter>,
}

impl GameData {
    /// Loads the engine data under `dir` (normally `data/engine`).
    ///
    /// # Errors
    ///
    /// Returns a [`LoadError`] naming the file, and the record where one is to
    /// blame, for the first problem found.
    pub fn load(dir: &Path, scope: &Scope) -> Result<Self, LoadError> {
        let locations = read_catalog::<Location>(dir, "locations.json")?;
        let mut quests = read_catalog::<Quest>(dir, "quests.json")?;
        let mut npcs = read_catalog::<Npc>(dir, "npcs.json")?;
        let encounters = read_catalog::<Encounter>(dir, "encounters.json")?;

        let dialogue_ids = match scope {
            Scope::Everywhere => list_dialogue_ids(&dir.join("dialogue"))?,
            Scope::Around(root) => {
                let areas = areas_within(&locations, root)?;
                quests
                    .records
                    .retain(|_, q| q.location_ids.iter().any(|l| areas.contains(l)));
                let givers: BTreeSet<NpcId> = quests
                    .iter()
                    .flat_map(|q| q.giver_ids.iter().cloned())
                    .collect();
                // A fight started here needs its foes' records, wherever they live.
                let foes: BTreeSet<NpcId> = encounters
                    .iter()
                    .filter(|e| areas.contains(&e.place))
                    .flat_map(|e| e.foes.iter().map(|foe| foe.npc.clone()))
                    .collect();
                npcs.records.retain(|id, n| {
                    givers.contains(id)
                        || foes.contains(id)
                        || n.location_ids.iter().any(|l| areas.contains(l))
                });
                npcs.iter()
                    .flat_map(|n| n.dialogue.iter().cloned())
                    .collect()
            }
        };

        let mut dialogue = Vec::with_capacity(dialogue_ids.len());
        for id in &dialogue_ids {
            dialogue.push(read_dialogue(&dir.join("dialogue"), id)?);
        }

        Ok(Self {
            factions: read_catalog(dir, "factions.json")?,
            weapons: read_catalog(dir, "weapons.json")?,
            armor: read_catalog(dir, "armor.json")?,
            ammo: read_catalog(dir, "ammo.json")?,
            consumables: read_catalog(dir, "consumables.json")?,
            encounters,
            dialogue: Catalog::from_records(&dir.join("dialogue"), dialogue)?,
            locations,
            quests,
            npcs,
        })
    }
}

/// `root` and every location inside it, found through `parent` links.
fn areas_within(
    locations: &Catalog<Location>,
    root: &LocationId,
) -> Result<BTreeSet<LocationId>, LoadError> {
    if !locations.contains(root) {
        return Err(LoadError::UnknownScopeRoot(root.clone()));
    }
    let mut inside = BTreeSet::from([root.clone()]);
    // `children` is only filled for some places, so follow `parent` until no
    // new location joins the set.
    loop {
        let before = inside.len();
        for location in locations.iter() {
            if location.parent.as_ref().is_some_and(|p| inside.contains(p)) {
                inside.insert(location.id.clone());
            }
        }
        if inside.len() == before {
            return Ok(inside);
        }
    }
}

/// Reads a file holding a JSON array of records and indexes it by id.
fn read_catalog<T: Record + DeserializeOwned>(
    dir: &Path,
    file: &str,
) -> Result<Catalog<T>, LoadError> {
    let path = dir.join(file);
    let bytes = fs::read(&path).map_err(|source| LoadError::Io {
        path: path.clone(),
        source,
    })?;
    let values: Vec<Value> = serde_json::from_slice(&bytes).map_err(|source| LoadError::Json {
        path: path.clone(),
        source,
    })?;
    let mut records = Vec::with_capacity(values.len());
    for (position, value) in values.into_iter().enumerate() {
        let id = value
            .get("id")
            .and_then(Value::as_str)
            .map_or_else(|| format!("#{position}"), str::to_owned);
        let record = serde_json::from_value(value).map_err(|source| LoadError::Record {
            path: path.clone(),
            id,
            source,
        })?;
        records.push(record);
    }
    Catalog::from_records(&path, records)
}

/// Reads one dialogue file and checks its id and topic links.
fn read_dialogue(dir: &Path, id: &DialogueId) -> Result<Dialogue, LoadError> {
    let path = dir.join(format!("{id}.json"));
    let bytes = fs::read(&path).map_err(|source| LoadError::Io {
        path: path.clone(),
        source,
    })?;
    let dialogue: Dialogue =
        serde_json::from_slice(&bytes).map_err(|source| LoadError::Record {
            path: path.clone(),
            id: id.to_string(),
            source,
        })?;
    if &dialogue.id != id {
        return Err(LoadError::DialogueIdMismatch {
            path,
            found: dialogue.id,
            expected: id.clone(),
        });
    }
    check_topics(&path, &dialogue)?;
    Ok(dialogue)
}

/// Topic keys must be unique, and `start` and `next` may only point at keys of the
/// same file.
fn check_topics(path: &Path, dialogue: &Dialogue) -> Result<(), LoadError> {
    let mut keys = BTreeSet::new();
    for topic in &dialogue.topics {
        if !keys.insert(&topic.key) {
            return Err(LoadError::DuplicateTopic {
                path: path.to_owned(),
                key: topic.key.to_string(),
            });
        }
    }
    if let Some(target) = dialogue.start.iter().find(|t| !keys.contains(t)) {
        return Err(LoadError::DanglingStart {
            path: path.to_owned(),
            target: target.to_string(),
        });
    }
    for topic in &dialogue.topics {
        if let Some(target) = topic.next.iter().find(|t| !keys.contains(t)) {
            return Err(LoadError::DanglingTopic {
                path: path.to_owned(),
                topic: topic.key.to_string(),
                target: target.to_string(),
            });
        }
    }
    Ok(())
}

/// The ids of every `*.json` file in the dialogue directory, in name order.
fn list_dialogue_ids(dir: &Path) -> Result<BTreeSet<DialogueId>, LoadError> {
    let io = |source| LoadError::Io {
        path: dir.to_owned(),
        source,
    };
    let mut ids = BTreeSet::new();
    for entry in fs::read_dir(dir).map_err(io)? {
        let path = entry.map_err(io)?.path();
        if path.extension().is_some_and(|e| e == "json") {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let id = DialogueId::try_new(stem).map_err(|source| LoadError::BadFileName {
                path: path.clone(),
                source,
            })?;
            ids.insert(id);
        }
    }
    Ok(ids)
}
