//! Quest records: stages, the wiki walkthrough and the consequence rules.

use serde::Deserialize;

use crate::shared::{Condition, Dlc, Effect, LocationId, NpcId, QuestId, QuestStage};

/// Whether a quest belongs to the main story.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestKind {
    /// Part of the main questline.
    Main,
    /// A side quest.
    Side,
}

/// What a consequence rule does to its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleKind {
    /// The target fails.
    Fails,
    /// The target becomes unavailable.
    Locks,
    /// The target becomes available.
    Unlocks,
    /// Only one of the target and this quest can be completed.
    Exclusive,
    /// The target starts.
    Triggers,
    /// The target must be done first.
    Requires,
    /// Some other effect, described in the note.
    Effect,
}

/// What kind of thing a consequence rule points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleTargetType {
    /// A quest.
    Quest,
    /// A game ending.
    Ending,
    /// A faction.
    Faction,
    /// A companion.
    Companion,
    /// An NPC.
    Npc,
    /// A perk.
    Perk,
    /// An item.
    Item,
    /// A location.
    Location,
    /// A dialogue option.
    Dialogue,
    /// A free-form effect.
    Effect,
}

/// One step of a quest as the journal shows it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stage {
    /// The stage number (5, 10, 20, ...).
    pub stage: QuestStage,
    /// The journal text.
    pub description: String,
    /// True when reaching this stage completes the quest.
    #[serde(default)]
    pub finish: bool,
    /// Effects applied once, the first time the quest reaches this stage.
    #[serde(default)]
    pub on_enter: Vec<Effect>,
}

/// One box of the wiki's flowchart.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WalkthroughNode {
    /// The box's role in the flowchart.
    pub kind: String,
    /// What it says.
    pub text: String,
}

/// A hand-curated consequence of this quest, with the wiki's wording beside the
/// machine-readable `conditions` and `effects`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// What the rule does.
    pub kind: RuleKind,
    /// The target's name as the wiki writes it.
    pub target: String,
    /// What the target is.
    pub target_type: RuleTargetType,
    /// The target's id, when it is a quest.
    pub target_id: Option<QuestId>,
    /// The wiki's condition, in prose.
    pub when: Option<String>,
    /// The wiki's note, in prose.
    pub note: Option<String>,
    /// The condition in machine-readable form; empty until authored.
    #[serde(default)]
    pub conditions: Vec<Condition>,
    /// The outcome in machine-readable form; empty until authored.
    #[serde(default)]
    pub effects: Vec<Effect>,
}

/// A quest.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Quest {
    /// The quest's unique id.
    pub id: QuestId,
    /// Display name.
    pub name: String,
    /// Main or side.
    pub kind: QuestKind,
    /// False for quests without a Pip-Boy marker.
    pub tracked: Option<bool>,
    /// The game or add-on it belongs to.
    pub dlc: Dlc,
    /// The game's editor id.
    pub edid: Option<String>,
    /// The game's form id.
    pub form_id: Option<String>,
    /// Wiki page the record was imported from.
    pub source: Option<String>,
    /// Giver names as the wiki writes them.
    #[serde(default)]
    pub giver: Vec<String>,
    /// Location names as the wiki writes them.
    #[serde(default)]
    pub location: Vec<String>,
    /// The wiki's description of the reward.
    pub reward_text: Option<String>,
    /// Experience for completing it.
    pub xp: Option<u32>,
    /// Quests that come before it.
    #[serde(default)]
    pub previous: Vec<QuestId>,
    /// Quests it leads to.
    #[serde(default)]
    pub leads_to: Vec<QuestId>,
    /// Quests related to it.
    #[serde(default)]
    pub related: Vec<QuestId>,
    /// Linked wiki pages that are not quests in the data.
    #[serde(default)]
    pub unresolved_links: Vec<String>,
    /// The journal stages, in order.
    pub stages: Vec<Stage>,
    /// The wiki's flowchart rows; several nodes in a row are branches.
    #[serde(default)]
    pub walkthrough: Vec<Vec<WalkthroughNode>>,
    /// The wiki's notes.
    #[serde(default)]
    pub notes: Vec<String>,
    /// Consequences of choices in this quest.
    #[serde(default)]
    pub rules: Vec<Rule>,
    /// `giver` matched to NPC ids.
    #[serde(default)]
    pub giver_ids: Vec<NpcId>,
    /// `location` matched to location ids.
    #[serde(default)]
    pub location_ids: Vec<LocationId>,
}
