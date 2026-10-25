//! Dialogue definitions: the topics a conversation walks through.

use std::collections::HashMap;

use thiserror::Error;

use crate::shared::{Dialogue, DialogueId, NpcId, Topic, TopicKey};

/// A dialogue definition was rejected while building the book.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DialogueBookError {
    /// Two dialogue files share an id.
    #[error("dialogue {dialogue:?} appears twice")]
    DuplicateDialogue {
        /// The repeated id.
        dialogue: DialogueId,
    },
    /// Two topics of one file share a key.
    #[error("dialogue {dialogue:?}: topic {topic:?} appears twice")]
    DuplicateTopic {
        /// The file.
        dialogue: DialogueId,
        /// The repeated key.
        topic: TopicKey,
    },
    /// `start` or a `next` list names a topic the file does not have.
    #[error("dialogue {dialogue:?}: {from} lists {target:?}, which is not a topic of the file")]
    UnknownTopic {
        /// The file.
        dialogue: DialogueId,
        /// Where the link is written: a topic key or `start`.
        from: String,
        /// The missing target.
        target: TopicKey,
    },
    /// A topic that ends the conversation also lists follow-ups.
    #[error("dialogue {dialogue:?}: topic {topic:?} ends the conversation but has a next list")]
    EndingHasNext {
        /// The file.
        dialogue: DialogueId,
        /// The topic.
        topic: TopicKey,
    },
}

#[derive(Debug)]
pub(super) struct Entry {
    speaker: Option<NpcId>,
    topics: Vec<Topic>,
    /// Topic key to position in `topics`.
    index: HashMap<TopicKey, usize>,
    /// Positions of the start topics.
    start: Vec<usize>,
}

impl Entry {
    pub(super) fn topic(&self, position: usize) -> &Topic {
        &self.topics[position]
    }

    pub(super) fn position(&self, key: &TopicKey) -> Option<usize> {
        self.index.get(key).copied()
    }

    pub(super) fn start(&self) -> &[usize] {
        &self.start
    }

    /// Positions of the topics that follow the topic at `position`.
    pub(super) fn follow_ups(&self, position: usize) -> impl Iterator<Item = usize> + '_ {
        self.topics[position]
            .next
            .iter()
            .filter_map(|key| self.position(key))
    }
}

/// Every speaker's topics, validated and read-only.
///
/// Links are resolved to positions once, so walking a conversation never searches
/// by name and a link can never dangle.
#[derive(Debug, Default)]
pub struct DialogueBook {
    dialogues: HashMap<DialogueId, Entry>,
}

impl DialogueBook {
    /// Builds the book from loaded dialogue records.
    ///
    /// # Errors
    ///
    /// [`DialogueBookError`] when an id or a topic key repeats, a link names a
    /// missing topic, or an ending topic lists follow-ups.
    pub fn new<'a>(
        dialogues: impl IntoIterator<Item = &'a Dialogue>,
    ) -> Result<Self, DialogueBookError> {
        let mut book = HashMap::new();
        for dialogue in dialogues {
            let entry = entry(dialogue)?;
            if book.insert(dialogue.id.clone(), entry).is_some() {
                return Err(DialogueBookError::DuplicateDialogue {
                    dialogue: dialogue.id.clone(),
                });
            }
        }
        Ok(Self { dialogues: book })
    }

    /// True when the book has the dialogue.
    #[must_use]
    pub fn contains(&self, dialogue: &DialogueId) -> bool {
        self.dialogues.contains_key(dialogue)
    }

    /// The NPC who speaks the dialogue, when the file was matched to one.
    #[must_use]
    pub fn speaker(&self, dialogue: &DialogueId) -> Option<&NpcId> {
        self.dialogues.get(dialogue)?.speaker.as_ref()
    }

    pub(super) fn entry(&self, dialogue: &DialogueId) -> Option<&Entry> {
        self.dialogues.get(dialogue)
    }
}

fn entry(dialogue: &Dialogue) -> Result<Entry, DialogueBookError> {
    let mut index = HashMap::with_capacity(dialogue.topics.len());
    for (position, topic) in dialogue.topics.iter().enumerate() {
        if index.insert(topic.key.clone(), position).is_some() {
            return Err(DialogueBookError::DuplicateTopic {
                dialogue: dialogue.id.clone(),
                topic: topic.key.clone(),
            });
        }
    }
    let unknown = |from: &str, target: &TopicKey| DialogueBookError::UnknownTopic {
        dialogue: dialogue.id.clone(),
        from: from.to_owned(),
        target: target.clone(),
    };
    let start = dialogue
        .start
        .iter()
        .map(|key| index.get(key).copied().ok_or_else(|| unknown("start", key)))
        .collect::<Result<Vec<_>, _>>()?;
    for topic in &dialogue.topics {
        if let Some(target) = topic.next.iter().find(|key| !index.contains_key(*key)) {
            return Err(unknown(topic.key.as_str(), target));
        }
        if topic.ends && !topic.next.is_empty() {
            return Err(DialogueBookError::EndingHasNext {
                dialogue: dialogue.id.clone(),
                topic: topic.key.clone(),
            });
        }
    }
    Ok(Entry {
        speaker: dialogue.speaker.clone(),
        topics: dialogue.topics.clone(),
        index,
        start,
    })
}
