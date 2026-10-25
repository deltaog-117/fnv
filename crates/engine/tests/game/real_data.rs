//! `Game` against the real quest data.

use std::path::Path;
use std::sync::Arc;

use engine::game::Game;
use engine::quests::QuestBook;
use engine::shared::{Effect, GameData, QuestStatus, QuestView, Scope, apply_all};

#[test]
fn every_quest_can_be_walked_to_its_end_through_apply_all() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    let data = GameData::load(&dir, &Scope::Everywhere).expect("the engine data loads");
    let book = Arc::new(QuestBook::new(data.quests.iter()).expect("book builds"));
    for quest in data.quests.iter().filter(|q| !q.stages.is_empty()) {
        let mut game = Game::new(Arc::clone(&book));
        // Several finish stages are alternative endings, so stop at the first.
        let end = quest
            .stages
            .iter()
            .position(|stage| stage.finish)
            .map_or(quest.stages.len(), |index| index + 1);
        let effects: Vec<Effect> = quest.stages[..end]
            .iter()
            .map(|stage| Effect::SetStage {
                quest: quest.id.clone(),
                stage: stage.stage,
            })
            .collect();
        apply_all(&effects, &mut game)
            .unwrap_or_else(|error| panic!("{}: {error}", quest.id.as_str()));
        let finishes = quest.stages.iter().any(|stage| stage.finish);
        let expected = if finishes {
            QuestStatus::Completed
        } else {
            QuestStatus::Active
        };
        assert_eq!(
            game.quest_status(&quest.id),
            expected,
            "{}",
            quest.id.as_str()
        );
    }
}
