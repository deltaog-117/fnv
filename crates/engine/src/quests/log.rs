//! The player's progress through the quests in a [`QuestBook`].

use std::collections::HashMap;
use std::sync::Arc;

use super::book::QuestBook;
use crate::shared::{EffectError, QuestId, QuestSink, QuestStage, QuestStatus, QuestView};

/// Where a started quest stands. "Not started" is the absence of a record, so a
/// stored quest can never claim to be unstarted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Active,
    Completed,
    Failed,
}

impl Phase {
    fn status(self) -> QuestStatus {
        match self {
            Self::Active => QuestStatus::Active,
            Self::Completed => QuestStatus::Completed,
            Self::Failed => QuestStatus::Failed,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Progress {
    phase: Phase,
    stage: Option<QuestStage>,
}

/// The player's quest progress.
///
/// Cloning is cheap: the book is shared, only the progress is copied. That is
/// what lets a caller try a batch of effects on a copy and keep it if it works.
#[derive(Debug, Clone)]
pub struct QuestLog {
    book: Arc<QuestBook>,
    progress: HashMap<QuestId, Progress>,
}

impl QuestLog {
    /// An empty log over the given definitions: nothing started.
    #[must_use]
    pub fn new(book: Arc<QuestBook>) -> Self {
        Self {
            book,
            progress: HashMap::new(),
        }
    }

    /// The definitions this log checks against.
    #[must_use]
    pub fn book(&self) -> &QuestBook {
        &self.book
    }

    /// The quests in progress with their current stage, for the journal. The
    /// order is unspecified; sort by name when displaying.
    pub fn active(&self) -> impl Iterator<Item = (&QuestId, Option<QuestStage>)> {
        self.progress
            .iter()
            .filter(|(_, progress)| progress.phase == Phase::Active)
            .map(|(quest, progress)| (quest, progress.stage))
    }

    fn require_known(&self, quest: &QuestId) -> Result<(), EffectError> {
        if self.book.contains(quest) {
            Ok(())
        } else {
            Err(EffectError::Unknown {
                kind: "quest",
                id: quest.as_str().to_owned(),
            })
        }
    }

    fn refuse(effect: &'static str, quest: &QuestId, reason: &str) -> EffectError {
        EffectError::Refused {
            effect,
            reason: format!("{}: {reason}", quest.as_str()),
        }
    }

    /// Ends a quest unsuccessfully. A finished quest keeps its result, so a late
    /// fail or lock-out cannot undo a completion.
    fn end_failed(&mut self, quest: &QuestId) {
        let progress = self.progress.entry(quest.clone()).or_insert(Progress {
            phase: Phase::Failed,
            stage: None,
        });
        if progress.phase == Phase::Active {
            progress.phase = Phase::Failed;
        }
    }
}

impl QuestView for QuestLog {
    fn quest_status(&self, quest: &QuestId) -> QuestStatus {
        self.progress
            .get(quest)
            .map_or(QuestStatus::NotStarted, |progress| progress.phase.status())
    }

    fn quest_stage(&self, quest: &QuestId) -> Option<QuestStage> {
        self.progress.get(quest).and_then(|progress| progress.stage)
    }
}

impl QuestSink for QuestLog {
    /// Starting an active or completed quest changes nothing; a failed quest
    /// cannot be started again.
    fn start_quest(&mut self, quest: &QuestId) -> Result<(), EffectError> {
        self.require_known(quest)?;
        match self.progress.get(quest).map(|progress| progress.phase) {
            Some(Phase::Failed) => Err(Self::refuse("start_quest", quest, "already failed")),
            Some(Phase::Active | Phase::Completed) => Ok(()),
            None => {
                self.progress.insert(
                    quest.clone(),
                    Progress {
                        phase: Phase::Active,
                        stage: None,
                    },
                );
                Ok(())
            }
        }
    }

    /// Only an active quest can be completed; completing a completed quest
    /// changes nothing.
    fn complete_quest(&mut self, quest: &QuestId) -> Result<(), EffectError> {
        self.require_known(quest)?;
        match self.progress.get_mut(quest) {
            Some(progress) if progress.phase == Phase::Active => {
                progress.phase = Phase::Completed;
                Ok(())
            }
            Some(progress) if progress.phase == Phase::Completed => Ok(()),
            Some(_) => Err(Self::refuse("complete_quest", quest, "already failed")),
            None => Err(Self::refuse("complete_quest", quest, "not started")),
        }
    }

    /// A quest can fail before it starts, because consequence rules shut quests
    /// out ahead of time.
    fn fail_quest(&mut self, quest: &QuestId) -> Result<(), EffectError> {
        self.require_known(quest)?;
        self.end_failed(quest);
        Ok(())
    }

    /// Failed is terminal, so locking a quest out is failing it: nothing can start
    /// it again.
    fn block_quest(&mut self, quest: &QuestId) -> Result<(), EffectError> {
        self.fail_quest(quest)
    }

    /// Moves forward only. Reaching a `finish` stage completes the quest, and
    /// reaching any stage of an unstarted quest starts it.
    fn set_stage(&mut self, quest: &QuestId, stage: QuestStage) -> Result<(), EffectError> {
        self.require_known(quest)?;
        let Some(definition) = self.book.stage(quest, stage) else {
            return Err(EffectError::Unknown {
                kind: "quest stage",
                id: format!("{} stage {}", quest.as_str(), stage.get()),
            });
        };
        let finish = definition.finish;
        if let Some(progress) = self.progress.get(quest) {
            if progress.phase != Phase::Active {
                return Err(Self::refuse(
                    "set_stage",
                    quest,
                    &format!("already {}", progress.phase.name()),
                ));
            }
            if progress.stage.is_some_and(|current| stage < current) {
                return Err(Self::refuse("set_stage", quest, "stages only move forward"));
            }
        }
        self.progress.insert(
            quest.clone(),
            Progress {
                phase: if finish {
                    Phase::Completed
                } else {
                    Phase::Active
                },
                stage: Some(stage),
            },
        );
        Ok(())
    }
}
