//! Faction and town records, including the fame and infamy model.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::shared::{Dlc, FactionId};

/// Whether a record is a faction or a town that tracks reputation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactionKind {
    /// A faction.
    Faction,
    /// A town with its own reputation.
    Town,
}

/// One remaining infobox field from the wiki.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum InfoValue {
    /// A single line of text.
    Text(String),
    /// Several lines.
    List(Vec<String>),
}

/// An event that changes reputation, as the wiki words it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReputationNote {
    /// What happens.
    pub text: String,
    /// Reputation points, when the wiki states them.
    pub delta: Option<i32>,
}

/// How fame and infamy are tracked for one faction or town.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReputationModel {
    /// Minimum points for each of the four tiers.
    pub thresholds: Option<[u32; 4]>,
    /// Titles indexed `[infamy tier][fame tier]`.
    pub titles: Option<[[String; 4]; 4]>,
    /// What the standing means for the player.
    #[serde(default)]
    pub consequences: Vec<ReputationNote>,
    /// Actions that raise reputation.
    #[serde(default)]
    pub gains: Vec<ReputationNote>,
    /// Actions that lower reputation.
    #[serde(default)]
    pub losses: Vec<ReputationNote>,
}

/// A faction, or a town that tracks reputation.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Faction {
    /// The faction's unique id.
    pub id: FactionId,
    /// Display name.
    pub name: String,
    /// Faction or town.
    pub kind: FactionKind,
    /// The game or add-on it belongs to.
    pub dlc: Dlc,
    /// Wiki page the record was imported from.
    pub source: Option<String>,
    /// The game's form id.
    pub form_id: Option<String>,
    /// Leader names as the wiki writes them.
    #[serde(default)]
    pub leaders: Vec<String>,
    /// Location names as the wiki writes them.
    #[serde(default)]
    pub locations: Vec<String>,
    /// The remaining infobox fields, as text.
    #[serde(default)]
    pub info: BTreeMap<String, InfoValue>,
    /// The fame and infamy model, for the factions that have one.
    pub reputation: Option<ReputationModel>,
}
