//! Location records and the map graph around them.

use serde::Deserialize;

use crate::shared::{Dlc, FactionId, LocationId, NpcId, QuestId};

/// How big a settlement is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocationSize {
    /// A small place.
    Small,
    /// A medium place.
    Medium,
    /// A large place.
    Large,
}

/// Which world map a location sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapKind {
    /// The Mojave.
    Base,
    /// The Sierra Madre.
    DeadMoney,
    /// Zion.
    HonestHearts,
    /// The Big MT.
    OldWorldBlues,
    /// The Divide.
    LonesomeRoad,
}

/// A point on the 1024x1024 map, in pixels from the top-left corner.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Position {
    /// Pixels from the left edge.
    pub x: f64,
    /// Pixels from the top edge.
    pub y: f64,
}

/// A travel link to a nearby location.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Neighbor {
    /// The location at the other end.
    pub id: LocationId,
    /// Distance in map pixels.
    pub distance: f64,
}

/// A place the player can be: a settlement, a building, a cave or a landmark.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Location {
    /// The location's unique id.
    pub id: LocationId,
    /// Display name.
    pub name: String,
    /// What sort of place it is (settlement, building, cave, ...).
    pub kind: String,
    /// How big it is, for settlements.
    pub size: Option<LocationSize>,
    /// The game or add-on it belongs to.
    pub dlc: Dlc,
    /// Wiki page the record was imported from.
    pub source: Option<String>,
    /// The game's cell name.
    pub cell: Option<String>,
    /// True when it has a map marker.
    pub marked: Option<bool>,
    /// Where the wiki files it, as a heading path.
    pub region: Option<String>,
    /// Which world map it is on.
    pub map: Option<MapKind>,
    /// Where it is on the map; buildings without a marker have none.
    pub position: Option<Position>,
    /// The map marker's category.
    pub map_category: Option<String>,
    /// The place it sits inside.
    pub parent: Option<LocationId>,
    /// The wiki's name for the parent, before it was matched to an id.
    pub parent_text: Option<String>,
    /// Places inside it.
    #[serde(default)]
    pub children: Vec<LocationId>,
    /// Derived travel links; an approximation, not the game's roads.
    #[serde(default)]
    pub neighbors: Vec<Neighbor>,
    /// Faction names as the wiki writes them.
    #[serde(default)]
    pub factions: Vec<String>,
    /// Leaders who live here.
    #[serde(default)]
    pub leaders: Vec<NpcId>,
    /// Merchants who work here.
    #[serde(default)]
    pub merchants: Vec<NpcId>,
    /// Doctors who work here.
    #[serde(default)]
    pub doctors: Vec<NpcId>,
    /// People the wiki names that did not match an NPC.
    #[serde(default)]
    pub people_unresolved: Vec<String>,
    /// Quests that take place here.
    #[serde(default)]
    pub quests: Vec<QuestId>,
    /// Creature names that live here.
    #[serde(default)]
    pub creatures: Vec<String>,
    /// `factions` matched to faction ids.
    #[serde(default)]
    pub faction_ids: Vec<FactionId>,
}
