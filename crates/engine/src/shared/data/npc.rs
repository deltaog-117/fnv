//! NPC records.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::shared::{DialogueId, Dlc, FactionId, LocationId, NpcId, QuestId, Skill};

/// An NPC's S.P.E.C.I.A.L. stats. Unlike the player's, a stat may be zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcSpecial {
    /// Strength.
    pub strength: Option<u32>,
    /// Perception.
    pub perception: Option<u32>,
    /// Endurance.
    pub endurance: Option<u32>,
    /// Charisma.
    pub charisma: Option<u32>,
    /// Intelligence.
    pub intelligence: Option<u32>,
    /// Agility.
    pub agility: Option<u32>,
    /// Luck.
    pub luck: Option<u32>,
}

/// A non-player character.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Npc {
    /// The NPC's unique id.
    pub id: NpcId,
    /// Display name.
    pub name: String,
    /// The game or add-on the NPC belongs to.
    pub dlc: Dlc,
    /// The game's editor id.
    pub edid: Option<String>,
    /// The game's form id.
    pub form_id: Option<String>,
    /// Wiki page the record was imported from.
    pub source: Option<String>,
    /// Race.
    pub race: Option<String>,
    /// Gender.
    pub gender: Option<String>,
    /// Karma, as the wiki writes it.
    pub karma: Option<String>,
    /// Level.
    pub level: Option<u32>,
    /// S.P.E.C.I.A.L. stats.
    pub special: Option<NpcSpecial>,
    /// Tagged skill values.
    #[serde(default)]
    pub skills: BTreeMap<Skill, u32>,
    /// Hit points.
    pub hp: Option<f64>,
    /// Maximum hit points, when it differs from `hp`.
    pub hp_max: Option<f64>,
    /// Damage threshold.
    pub dt: Option<f64>,
    /// True for recruitable companions.
    pub companion: Option<bool>,
    /// How aggressive the NPC is.
    pub aggression: Option<String>,
    /// How confident the NPC is.
    pub confidence: Option<String>,
    /// Faction or group names as the wiki writes them.
    #[serde(default)]
    pub affiliations: Vec<String>,
    /// Roles such as merchant or doctor.
    #[serde(default)]
    pub roles: Vec<String>,
    /// Location names as the wiki writes them.
    #[serde(default)]
    pub locations: Vec<String>,
    /// The game's own faction names.
    #[serde(default)]
    pub geck_factions: Vec<String>,
    /// Quests the NPC is part of.
    #[serde(default)]
    pub quests: Vec<QuestId>,
    /// Quest links that did not match a quest.
    #[serde(default)]
    pub unresolved_quests: Vec<String>,
    /// This NPC's dialogue files.
    #[serde(default)]
    pub dialogue: Vec<DialogueId>,
    /// `affiliations` matched to faction ids.
    #[serde(default)]
    pub faction_ids: Vec<FactionId>,
    /// `locations` matched to location ids.
    #[serde(default)]
    pub location_ids: Vec<LocationId>,
}
