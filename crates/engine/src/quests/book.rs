//! Quest definitions: the stages and rules a log checks its moves against.

use std::collections::HashMap;

use thiserror::Error;

use crate::shared::{Quest, QuestId, QuestStage, Rule, Stage};

/// A quest definition was rejected while building the book.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum QuestBookError {
    /// Two quests share an id.
    #[error("quest {quest:?} appears twice")]
    DuplicateQuest {
        /// The repeated id.
        quest: QuestId,
    },
    /// A quest's stages are not in strictly increasing order.
    #[error("quest {quest:?}: stage {stage} does not come after the stage before it")]
    StagesOutOfOrder {
        /// The quest.
        quest: QuestId,
        /// The first stage that breaks the order.
        stage: u32,
    },
}

#[derive(Debug)]
struct Entry {
    name: String,
    stages: Vec<Stage>,
    rules: Vec<Rule>,
    /// Some stage or rule carries effects, so changing this quest has side effects.
    automated: bool,
}

/// Every quest's stages and consequence rules, validated and read-only.
///
/// Stages are kept sorted, which is what lets the log answer "is this stage
/// later than that one" and look a stage up by binary search.
#[derive(Debug, Default)]
pub struct QuestBook {
    quests: HashMap<QuestId, Entry>,
}

impl QuestBook {
    /// Builds the book from loaded quest records.
    ///
    /// # Errors
    ///
    /// [`QuestBookError`] when an id repeats or a quest's stages are out of order.
    pub fn new<'a>(quests: impl IntoIterator<Item = &'a Quest>) -> Result<Self, QuestBookError> {
        let mut book = HashMap::new();
        for quest in quests {
            if let Some(pair) = quest
                .stages
                .windows(2)
                .find(|pair| pair[0].stage >= pair[1].stage)
            {
                return Err(QuestBookError::StagesOutOfOrder {
                    quest: quest.id.clone(),
                    stage: pair[1].stage.get(),
                });
            }
            let automated = quest.stages.iter().any(|stage| !stage.on_enter.is_empty())
                || quest.rules.iter().any(|rule| !rule.effects.is_empty());
            let entry = Entry {
                name: quest.name.clone(),
                stages: quest.stages.clone(),
                rules: quest.rules.clone(),
                automated,
            };
            if book.insert(quest.id.clone(), entry).is_some() {
                return Err(QuestBookError::DuplicateQuest {
                    quest: quest.id.clone(),
                });
            }
        }
        Ok(Self { quests: book })
    }

    /// True when the book defines the quest.
    #[must_use]
    pub fn contains(&self, quest: &QuestId) -> bool {
        self.quests.contains_key(quest)
    }

    /// The quest's display name.
    #[must_use]
    pub fn name(&self, quest: &QuestId) -> Option<&str> {
        self.quests.get(quest).map(|entry| entry.name.as_str())
    }

    /// The quest's stage with that number, if it has one.
    #[must_use]
    pub fn stage(&self, quest: &QuestId, stage: QuestStage) -> Option<&Stage> {
        let stages = &self.quests.get(quest)?.stages;
        stages
            .binary_search_by_key(&stage, |candidate| candidate.stage)
            .ok()
            .map(|index| &stages[index])
    }

    /// True when a stage or rule of the quest carries effects, which is what tells
    /// the game that changing the quest needs more than a log update.
    #[must_use]
    pub fn is_automated(&self, quest: &QuestId) -> bool {
        self.quests.get(quest).is_some_and(|entry| entry.automated)
    }

    /// The quest's consequence rules; empty for an unknown quest.
    #[must_use]
    pub fn rules(&self, quest: &QuestId) -> &[Rule] {
        self.quests
            .get(quest)
            .map_or(&[], |entry| entry.rules.as_slice())
    }
}
