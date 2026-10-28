//! Golden dialogue scenarios from `tests/scenarios/goodsprings_dialogue.json`:
//! language-neutral JSON (topics picked, expected quest and flag state) run against
//! the real data, so any implementation of the game can be checked against it.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use engine::character::{Character, CharacterRules, SpecialSet};
use engine::dialogue::{Conversation, DialogueBook};
use engine::game::Game;
use engine::inventory::ItemBook;
use engine::quests::QuestBook;
use engine::shared::{
    DialogueId, Effect, FlagId, GameData, InventoryView, ItemId, QuestId, QuestStatus, QuestView,
    Scope, Special, SpecialLevel, TopicKey, WorldView, apply_all,
};
use engine::world::WorldBook;
use serde::Deserialize;

#[derive(Deserialize)]
struct File {
    scenarios: Vec<Scenario>,
}

#[derive(Deserialize)]
struct Scenario {
    name: String,
    dialogue: DialogueId,
    #[serde(default)]
    setup: Vec<Effect>,
    /// Attributes that differ from the default 5s, so a skill check can pass.
    #[serde(default)]
    special: BTreeMap<Special, u32>,
    /// Hit points already lost when the talk begins.
    #[serde(default)]
    wounds: u32,
    steps: Vec<Step>,
    expect: Expect,
}

#[derive(Deserialize)]
struct Step {
    say: TopicKey,
    #[serde(default)]
    fails: bool,
}

#[derive(Deserialize)]
struct Expect {
    #[serde(default)]
    quests: BTreeMap<QuestId, ExpectedQuest>,
    #[serde(default)]
    flags: Vec<FlagId>,
    /// Items held at the end, by count.
    #[serde(default)]
    items: BTreeMap<ItemId, u32>,
    caps: Option<u32>,
    /// Hit points left at the end.
    hp: Option<u32>,
    over: Option<bool>,
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
fn the_goodsprings_dialogue_scenarios_hold() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = GameData::load(&root.join("data/engine"), &Scope::Everywhere)
        .expect("the engine data loads");
    let quests = Arc::new(QuestBook::new(data.quests.iter()).expect("book builds"));
    let dialogue = Arc::new(DialogueBook::new(data.dialogue.iter()).expect("book builds"));
    // The roster lets a scenario's setup kill or turn an NPC, as the real game can.
    let world = Arc::new(WorldBook::new(data.npcs.iter().map(|npc| &npc.id)));
    let items = Arc::new(ItemBook::from_data(&data).expect("the item book builds"));
    let text = fs::read_to_string(root.join("tests/scenarios/goodsprings_dialogue.json"))
        .expect("the scenario file exists");
    let file: File = serde_json::from_str(&text).expect("the scenario file parses");
    assert!(!file.scenarios.is_empty());

    for scenario in file.scenarios {
        let name = &scenario.name;
        let spread = scenario
            .special
            .iter()
            .fold(SpecialSet::default(), |set, (stat, level)| {
                let level = SpecialLevel::try_new(*level)
                    .unwrap_or_else(|e| panic!("{name}: {stat:?} level: {e}"));
                set.with(*stat, level)
            });
        // The character comes first: the wound cap is taken from it.
        let mut game = Game::with_world(Arc::clone(&quests), Arc::clone(&world))
            .with_items(Arc::clone(&items))
            .with_character(Character::new(spread, CharacterRules::standard()))
            .with_wounds(scenario.wounds);
        apply_all(&scenario.setup, &mut game).unwrap_or_else(|e| panic!("{name}: setup: {e}"));
        let mut talk = Conversation::open(Arc::clone(&dialogue), &scenario.dialogue)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        for step in &scenario.steps {
            let result = talk.choose(&mut game, &step.say);
            assert_eq!(
                result.is_err(),
                step.fails,
                "{name}: say {} -> {result:?}",
                step.say
            );
        }
        for (quest, expected) in &scenario.expect.quests {
            assert_eq!(
                status_name(game.quest_status(quest)),
                expected.status,
                "{name}: {quest}"
            );
            if let Some(stage) = expected.stage {
                assert_eq!(
                    game.quest_stage(quest).map(|s| s.get()),
                    Some(stage),
                    "{name}: {quest} stage"
                );
            }
        }
        for flag in &scenario.expect.flags {
            assert!(game.flag(flag), "{name}: flag {flag} is not set");
        }
        for (item, count) in &scenario.expect.items {
            assert_eq!(game.item_count(item), *count, "{name}: {item} held");
        }
        if let Some(caps) = scenario.expect.caps {
            assert_eq!(game.caps(), caps, "{name}: caps");
        }
        if let Some(hp) = scenario.expect.hp {
            assert_eq!(game.hp(), hp, "{name}: hit points");
        }
        if let Some(over) = scenario.expect.over {
            assert_eq!(talk.is_over(), over, "{name}: over");
        }
    }
}
