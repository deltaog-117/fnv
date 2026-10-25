//! Builders shared by the dialogue tests.

use std::sync::Arc;

use engine::dialogue::{Conversation, DialogueBook};
use engine::game::Game;
use engine::quests::QuestBook;
use engine::shared::{Dialogue, DialogueId, FlagId, Quest, TopicKey, WorldView};
use serde_json::{Value, json};

pub fn key(text: &str) -> TopicKey {
    TopicKey::try_new(text.to_owned()).expect("literal is a valid id")
}

pub fn dialogue_id(text: &str) -> DialogueId {
    DialogueId::try_new(text.to_owned()).expect("literal is a valid id")
}

pub fn flag(text: &str) -> FlagId {
    FlagId::try_new(text.to_owned()).expect("literal is a valid id")
}

/// A topic record with one response; `extra` holds the authored fields.
pub fn topic(key: &str, extra: Value) -> Value {
    let mut topic = json!({
        "key": key,
        "section": "topics",
        "prompt": format!("say {key}"),
        "responses": [{"text": format!("answer {key}")}],
    });
    if let (Some(base), Some(more)) = (topic.as_object_mut(), extra.as_object()) {
        base.extend(more.clone());
    }
    topic
}

pub fn dialogue(id: &str, start: &[&str], topics: Vec<Value>) -> Dialogue {
    serde_json::from_value(json!({
        "id": id,
        "speaker_name": id,
        "start": start,
        "topics": topics,
    }))
    .expect("the literal is a valid dialogue record")
}

pub fn book(dialogues: &[Dialogue]) -> Arc<DialogueBook> {
    Arc::new(DialogueBook::new(dialogues.iter()).expect("the dialogue is well formed"))
}

pub fn talk(dialogue: &Dialogue) -> Conversation {
    Conversation::open(book(std::slice::from_ref(dialogue)), &dialogue.id)
        .expect("the dialogue has start topics")
}

/// A game with one quest `q` (stages 5 and 10), so quest effects have a target.
pub fn game() -> Game {
    let quest: Quest = serde_json::from_value(json!({
        "id": "q", "name": "q", "kind": "side", "dlc": "base",
        "stages": [
            {"stage": 5, "description": "s"},
            {"stage": 10, "description": "s"},
        ],
    }))
    .expect("the literal is a valid quest record");
    Game::new(Arc::new(
        QuestBook::new([&quest]).expect("stages are in order"),
    ))
}

pub fn has_flag(game: &Game, name: &str) -> bool {
    game.flag(&flag(name))
}

pub fn keys(choices: &[engine::dialogue::Choice]) -> Vec<&str> {
    choices.iter().map(|choice| choice.key.as_str()).collect()
}
