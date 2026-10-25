//! The quest log against the real quest data.

use std::path::Path;
use std::sync::Arc;

use engine::quests::{QuestBook, QuestLog};
use engine::shared::{GameData, QuestSink, QuestStatus, QuestView, Scope};

fn everything() -> GameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    GameData::load(&dir, &Scope::Everywhere).expect("the engine data loads")
}

#[test]
fn every_quest_builds_into_the_book() {
    let data = everything();
    let book = QuestBook::new(data.quests.iter()).expect("every quest's stages are in order");
    assert!(data.quests.iter().all(|quest| book.contains(&quest.id)));
}

#[test]
fn walking_every_quest_through_its_stages_ends_where_the_data_says() {
    let data = everything();
    let book = Arc::new(QuestBook::new(data.quests.iter()).expect("book builds"));
    for quest in data.quests.iter().filter(|q| !q.stages.is_empty()) {
        let mut log = QuestLog::new(Arc::clone(&book));
        for stage in &quest.stages {
            log.set_stage(&quest.id, stage.stage)
                .unwrap_or_else(|error| panic!("{}: {error}", quest.id.as_str()));
            let expected = if stage.finish {
                QuestStatus::Completed
            } else {
                QuestStatus::Active
            };
            assert_eq!(
                log.quest_status(&quest.id),
                expected,
                "{}",
                quest.id.as_str()
            );
            if stage.finish {
                break;
            }
        }
    }
}

#[test]
fn ghost_town_gunfight_reads_like_the_journal() {
    let data = everything();
    let book = Arc::new(QuestBook::new(data.quests.iter()).expect("book builds"));
    let gunfight = crate::support::id("ghost_town_gunfight");
    let mut log = QuestLog::new(Arc::clone(&book));
    log.start_quest(&gunfight).expect("starts");
    log.set_stage(&gunfight, crate::support::stage(60))
        .expect("skips the optional steps");
    assert_eq!(log.quest_status(&gunfight), QuestStatus::Active);
    log.set_stage(&gunfight, crate::support::stage(70))
        .expect("finishes");
    assert_eq!(log.quest_status(&gunfight), QuestStatus::Completed);
    assert_eq!(book.name(&gunfight), Some("Ghost Town Gunfight"));
}
