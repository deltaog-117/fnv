//! Builders shared by the quest tests.

use std::sync::Arc;

use engine::quests::{QuestBook, QuestLog};
use engine::shared::{Quest, QuestId, QuestStage};
use serde_json::json;

pub fn id(text: &str) -> QuestId {
    QuestId::try_new(text.to_owned()).expect("literal is a valid id")
}

pub fn stage(number: u32) -> QuestStage {
    QuestStage::new(number)
}

/// A quest whose stages are `(number, finish)` pairs.
pub fn quest(name: &str, stages: &[(u32, bool)]) -> Quest {
    let stages: Vec<_> = stages
        .iter()
        .map(|(number, finish)| json!({"stage": number, "description": "step", "finish": finish}))
        .collect();
    serde_json::from_value(json!({
        "id": name,
        "name": name,
        "kind": "side",
        "dlc": "base",
        "stages": stages,
    }))
    .expect("the literal is a valid quest record")
}

/// A log over one quest `q` with stages 5, 10, 20 and a finishing 30.
pub fn log() -> QuestLog {
    let book = QuestBook::new([&quest(
        "q",
        &[(5, false), (10, false), (20, false), (30, true)],
    )])
    .expect("stages are in order");
    QuestLog::new(Arc::new(book))
}
