//! One conversation: where it stands, what can be said next, and what saying it does.

use std::sync::Arc;

use thiserror::Error;

use super::book::{DialogueBook, Entry};
use crate::shared::{
    BatchError, Condition, DialogueId, Response, State, StateView, TopicKey, VendorId, apply_all,
    evaluate,
};

/// A move in a conversation was refused.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum DialogueError {
    /// The book has no such dialogue.
    #[error("no dialogue {dialogue:?}")]
    UnknownDialogue {
        /// The id asked for.
        dialogue: DialogueId,
    },
    /// The dialogue has no start topics yet, so a conversation could not begin.
    #[error("dialogue {dialogue:?} has no start topics: its graph is not authored yet")]
    NotAuthored {
        /// The dialogue.
        dialogue: DialogueId,
    },
    /// The conversation has ended.
    #[error("the conversation is over")]
    Over,
    /// The topic is not on offer now: unknown, or its conditions do not hold.
    #[error("topic {topic:?} is not on offer")]
    NotOffered {
        /// The topic asked for.
        topic: TopicKey,
    },
    /// The topic is listed but its check fails.
    #[error("topic {topic:?} is locked: its check fails")]
    Locked {
        /// The topic asked for.
        topic: TopicKey,
    },
    /// An effect of the topic failed; nothing was applied.
    #[error("topic {topic:?}: {source}")]
    Effects {
        /// The topic.
        topic: TopicKey,
        /// The failing effect.
        #[source]
        source: BatchError,
    },
}

/// An option the player can see in the menu.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    /// What to pass to [`Conversation::choose`].
    pub key: TopicKey,
    /// What the player says: the topic's prompt, else its label, else its key.
    pub text: String,
    /// The check that gates the option, so the menu can show what is needed.
    pub requires: Option<Condition>,
    /// False while the check fails; a locked option cannot be chosen.
    pub unlocked: bool,
}

/// The speaker's answer to a chosen topic.
#[derive(Debug, Clone, PartialEq)]
pub struct Reply {
    /// The topic that was chosen.
    pub topic: TopicKey,
    /// What the speaker says, in order.
    pub lines: Vec<Response>,
    /// True when the topic closed the conversation.
    pub over: bool,
    /// The vendor whose shop the player asked to see, if the topic opens one.
    pub shop: Option<VendorId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Position {
    /// At the start topics.
    Hub,
    /// After the topic at this position, offering its follow-ups.
    Following(usize),
    /// Closed.
    Over,
}

/// A conversation with one speaker.
///
/// Cloning is cheap (the book is shared). A conversation holds no game state, so
/// saving a game mid-talk needs only the dialogue id and this position.
#[derive(Debug, Clone)]
pub struct Conversation {
    book: Arc<DialogueBook>,
    dialogue: DialogueId,
    position: Position,
}

impl Conversation {
    /// Opens a conversation at the speaker's start topics.
    ///
    /// # Errors
    ///
    /// [`DialogueError::UnknownDialogue`] when the book lacks the dialogue, and
    /// [`DialogueError::NotAuthored`] when it has no start topics.
    pub fn open(book: Arc<DialogueBook>, dialogue: &DialogueId) -> Result<Self, DialogueError> {
        let entry = book
            .entry(dialogue)
            .ok_or_else(|| DialogueError::UnknownDialogue {
                dialogue: dialogue.clone(),
            })?;
        if entry.start().is_empty() {
            return Err(DialogueError::NotAuthored {
                dialogue: dialogue.clone(),
            });
        }
        Ok(Self {
            book,
            dialogue: dialogue.clone(),
            position: Position::Hub,
        })
    }

    /// The dialogue this conversation is in.
    #[must_use]
    pub fn dialogue(&self) -> &DialogueId {
        &self.dialogue
    }

    /// True once a topic that ends the conversation was chosen.
    #[must_use]
    pub fn is_over(&self) -> bool {
        self.position == Position::Over
    }

    /// What the player can say now, in the order the data lists it. Empty once the
    /// conversation is over.
    #[must_use]
    pub fn choices<V: StateView + ?Sized>(&self, view: &V) -> Vec<Choice> {
        let Some(entry) = self.book.entry(&self.dialogue) else {
            return Vec::new();
        };
        self.offered(entry, view)
            .into_iter()
            .map(|position| {
                let topic = entry.topic(position);
                Choice {
                    key: topic.key.clone(),
                    text: topic
                        .prompt
                        .as_deref()
                        .or(topic.label.as_deref())
                        .unwrap_or(topic.key.as_str())
                        .to_owned(),
                    requires: topic.check.clone(),
                    unlocked: topic.check.as_ref().is_none_or(|c| evaluate(c, view)),
                }
            })
            .collect()
    }

    /// Says the topic: checks it is on offer and unlocked, applies its effects as a
    /// unit, and moves to its follow-ups (or back to the start, or to the end).
    ///
    /// # Errors
    ///
    /// [`DialogueError`] when the conversation is over, the topic is not on offer
    /// or is locked, or an effect fails. On error the state and the conversation
    /// are left as they were.
    pub fn choose<S: State + Clone>(
        &mut self,
        state: &mut S,
        topic: &TopicKey,
    ) -> Result<Reply, DialogueError> {
        if self.is_over() {
            return Err(DialogueError::Over);
        }
        let entry =
            self.book
                .entry(&self.dialogue)
                .ok_or_else(|| DialogueError::UnknownDialogue {
                    dialogue: self.dialogue.clone(),
                })?;
        let not_offered = || DialogueError::NotOffered {
            topic: topic.clone(),
        };
        let position = entry.position(topic).ok_or_else(not_offered)?;
        if !self.offered(entry, &*state).contains(&position) {
            return Err(not_offered());
        }
        let chosen = entry.topic(position);
        if chosen.check.as_ref().is_some_and(|c| !evaluate(c, &*state)) {
            return Err(DialogueError::Locked {
                topic: topic.clone(),
            });
        }
        // Applied to a copy: `apply_all` can fail part-way, and a half-said topic
        // must not leave half its effects behind.
        let mut trial = state.clone();
        apply_all(&chosen.effects, &mut trial).map_err(|source| DialogueError::Effects {
            topic: topic.clone(),
            source,
        })?;
        *state = trial;
        self.position = if chosen.ends {
            Position::Over
        } else if chosen.next.is_empty() {
            Position::Hub
        } else {
            Position::Following(position)
        };
        Ok(Reply {
            topic: topic.clone(),
            lines: chosen.responses.iter().cloned().collect(),
            over: chosen.ends,
            shop: chosen.shop.clone(),
        })
    }

    /// Positions of the topics on offer: the current list filtered by conditions.
    /// When a topic's follow-ups are all unavailable the player must not be stuck,
    /// so the start topics are offered instead.
    fn offered<V: StateView + ?Sized>(&self, entry: &Entry, view: &V) -> Vec<usize> {
        let available = |positions: &mut dyn Iterator<Item = usize>| -> Vec<usize> {
            positions
                .filter(|&position| {
                    entry
                        .topic(position)
                        .conditions
                        .iter()
                        .all(|condition| evaluate(condition, view))
                })
                .collect()
        };
        let start = || available(&mut entry.start().iter().copied());
        match self.position {
            Position::Over => Vec::new(),
            Position::Hub => start(),
            Position::Following(from) => {
                let follow_ups = available(&mut entry.follow_ups(from));
                if follow_ups.is_empty() {
                    start()
                } else {
                    follow_ups
                }
            }
        }
    }
}
