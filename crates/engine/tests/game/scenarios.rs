//! Golden scenarios from `tests/scenarios/`: language-neutral JSON (effects in,
//! expected quest state out) run against the real data, so any implementation of
//! the game can be checked against the same file.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use engine::game::Game;
use engine::quests::QuestBook;
use engine::shared::{Effect, GameData, QuestId, QuestStatus, QuestView, Scope, apply_all};
use serde::Deserialize;

#[derive(Deserialize)]
struct File {
    scenarios: Vec<Scenario>,
}

#[derive(Deserialize)]
struct Scenario {
    name: String,
    steps: Vec<Step>,
    expect: Expect,
}

#[derive(Deserialize)]
struct Step {
    r#do: Effect,
    #[serde(default)]
    fails: bool,
}

#[derive(Deserialize)]
struct Expect {
    #[serde(default)]
    quests: BTreeMap<QuestId, ExpectedQuest>,
    deferred: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct ExpectedQuest {
    status: String,
    stage: Option<u32>,
}

fn status_name(status: QuestStatus) -> &'static str {
    match status {
        QuestStatus::NotStarted => "not_started",
        QuestStatus::Active => "active",
        QuestStatus::Completed => "completed",
        QuestStatus::Failed => "failed",
    }
}

#[test]
fn the_goodsprings_scenarios_hold() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = GameData::load(&root.join("data/engine"), &Scope::Everywhere)
        .expect("the engine data loads");
    let book = Arc::new(QuestBook::new(data.quests.iter()).expect("book builds"));
    let text = fs::read_to_string(root.join("tests/scenarios/goodsprings.json"))
        .expect("the scenario file exists");
    let file: File = serde_json::from_str(&text).expect("the scenario file parses");
    assert!(!file.scenarios.is_empty());

    for scenario in file.scenarios {
        let mut game = Game::new(Arc::clone(&book));
        for step in &scenario.steps {
            let result = apply_all(std::slice::from_ref(&step.r#do), &mut game);
            assert_eq!(
                result.is_err(),
                step.fails,
                "{}: step {:?} -> {result:?}",
                scenario.name,
                step.r#do
            );
        }
        for (quest, expected) in &scenario.expect.quests {
            assert_eq!(
                status_name(game.quest_status(quest)),
                expected.status,
                "{}: {quest}",
                scenario.name
            );
            if let Some(stage) = expected.stage {
                assert_eq!(
                    game.quest_stage(quest).map(|stage| stage.get()),
                    Some(stage),
                    "{}: {quest} stage",
                    scenario.name
                );
            }
        }
        if let Some(tags) = &scenario.expect.deferred {
            let actual: Vec<&str> = game.deferred().iter().map(|d| d.effect.tag()).collect();
            assert_eq!(&actual, tags, "{}: deferred", scenario.name);
        }
    }
}
