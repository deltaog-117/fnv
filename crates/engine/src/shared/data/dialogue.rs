//! Dialogue records: one speaker's topics.
//!
//! The wiki records no links between topics; `conditions`, `effects` and `next`
//! are empty until the graph is authored.

use serde::Deserialize;

use crate::shared::{Condition, DialogueId, Effect, NonEmpty, NpcId, TopicKey};

/// Where in a conversation a topic is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopicSection {
    /// A topic the player picks from the menu.
    Topics,
    /// A scripted exchange.
    Conversation,
    /// A combat bark.
    Combat,
    /// A detection bark.
    Detection,
    /// Anything else.
    Other,
}

/// One line the speaker says.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    /// The spoken text.
    pub text: String,
    /// The emotion it is delivered with.
    pub emotion: Option<String>,
    /// How strong the emotion is, 0 to 100.
    pub intensity: Option<u32>,
}

/// A topic: what the player says, and how the speaker answers.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Topic {
    /// Unique within the file.
    pub key: TopicKey,
    /// The wiki's topic id; not unique.
    pub topic: Option<String>,
    /// Where the topic is used.
    pub section: TopicSection,
    /// What the player says to open the topic.
    pub prompt: Option<String>,
    /// A short label for menus.
    pub label: Option<String>,
    /// A skill or stat check that gates the topic.
    pub check: Option<Condition>,
    /// The speaker's lines, at least one.
    pub responses: NonEmpty<Response>,
    /// Conditions for the topic to be offered; empty until authored.
    #[serde(default)]
    pub conditions: Vec<Condition>,
    /// What choosing the topic does; empty until authored.
    #[serde(default)]
    pub effects: Vec<Effect>,
    /// Topics that follow this one; empty until authored.
    #[serde(default)]
    pub next: Vec<TopicKey>,
}

/// All topics of one speaker.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dialogue {
    /// The file's unique id, equal to its file name.
    pub id: DialogueId,
    /// The NPC speaking, when the file was matched to one.
    pub speaker: Option<NpcId>,
    /// The speaker's name as the wiki writes it.
    pub speaker_name: String,
    /// Wiki page the record was imported from.
    pub source: Option<String>,
    /// The speaker's topics.
    pub topics: Vec<Topic>,
}
