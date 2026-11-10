//! How long ago something happened in a quest: the game notes the time of each
//! start, finish, failure and stage, a rule can ask for whole days since, and
//! waiting runs the rules that ask.

use std::num::NonZeroU32;
use std::sync::Arc;

use engine::clock::Span;
use engine::game::Game;
use engine::quests::QuestBook;
use engine::saves::Save;
use engine::shared::{
    Condition, Effect, FlagId, Quest, QuestEvent, QuestId, QuestStage, QuestView, WorldView,
    apply_all, evaluate,
};
use proptest::prelude::*;
use serde_json::json;

/// Quest `warned` has stages 10 and 20; once three days have passed since stage 10,
/// its rule sets the flag `patrol_sent`. A second rule reads the days since the
/// quest began and sets `long_ago`.
fn game() -> Game {
    let quests: Vec<Quest> = [json!({
        "id": "warned", "name": "warned", "kind": "side", "dlc": "base",
        "stages": [
            {"stage": 10, "description": "s"},
            {"stage": 20, "description": "s"},
        ],
        "rules": [
            {
                "kind": "locks", "target": "T", "target_type": "quest", "target_id": "warned",
                "conditions": [
                    {"type": "days_since", "quest": "warned", "event": {"stage": 10}, "days": 3},
                ],
                "effects": [{"type": "set_flag", "flag": "patrol_sent"}],
            },
            {
                "kind": "locks", "target": "T", "target_type": "quest", "target_id": "warned",
                "conditions": [
                    {"type": "days_since", "quest": "warned", "event": "started", "days": 5},
                ],
                "effects": [{"type": "set_flag", "flag": "long_ago"}],
            },
        ],
    })]
    .into_iter()
    .map(|value| serde_json::from_value(value).expect("the literal is a valid quest record"))
    .collect();
    Game::new(Arc::new(
        QuestBook::new(quests.iter()).expect("stages are in order"),
    ))
}

fn id(text: &str) -> QuestId {
    QuestId::try_new(text.to_owned()).expect("literal is a valid id")
}

fn flag(text: &str) -> FlagId {
    FlagId::try_new(text.to_owned()).expect("literal is a valid id")
}

fn stage(game: &mut Game, number: u32) {
    let effect = Effect::SetStage {
        quest: id("warned"),
        stage: QuestStage::new(number),
    };
    apply_all(&[effect], game).expect("the stage is reached");
}

fn wait_hours(game: &mut Game, hours: u32) {
    let span = Span::hours(NonZeroU32::new(hours).expect("not zero"));
    game.wait(span).expect("the player is up");
}

fn since(event: QuestEvent) -> Condition {
    serde_json::from_value(json!({
        "type": "days_since", "quest": "warned",
        "event": serde_json::to_value(event).expect("an event is data"), "days": 1,
    }))
    .expect("the literal is a valid condition")
}

#[test]
fn a_moment_that_has_not_happened_has_no_age_and_is_never_days_ago() {
    let mut game = game();
    assert_eq!(game.minutes_since(&id("warned"), QuestEvent::Started), None);
    wait_hours(&mut game, 72);
    assert!(!evaluate(&since(QuestEvent::Started), &game));
    assert!(!evaluate(&since(QuestEvent::Completed), &game));
}

#[test]
fn the_game_notes_when_a_quest_began_and_each_stage_it_reached() {
    let mut game = game();
    wait_hours(&mut game, 2);
    stage(&mut game, 10);
    wait_hours(&mut game, 5);
    stage(&mut game, 20);
    wait_hours(&mut game, 1);
    let quest = id("warned");
    // Setting a stage starts the quest, so both moments are the same minute.
    assert_eq!(
        game.minutes_since(&quest, QuestEvent::Started),
        Some(6 * 60)
    );
    assert_eq!(
        game.minutes_since(&quest, QuestEvent::Stage(QuestStage::new(10))),
        Some(6 * 60)
    );
    assert_eq!(
        game.minutes_since(&quest, QuestEvent::Stage(QuestStage::new(20))),
        Some(60)
    );
    assert_eq!(game.minutes_since(&quest, QuestEvent::Completed), None);
}

#[test]
fn whole_days_count_and_a_part_of_a_day_does_not() {
    let mut game = game();
    stage(&mut game, 10);
    let one_day = since(QuestEvent::Stage(QuestStage::new(10)));
    wait_hours(&mut game, 23);
    assert!(!evaluate(&one_day, &game), "23 hours is not a day");
    wait_hours(&mut game, 1);
    assert!(evaluate(&one_day, &game), "24 hours is");
}

#[test]
fn waiting_runs_the_rules_that_ask_for_days_and_each_fires_once() {
    let mut game = game();
    stage(&mut game, 10);
    wait_hours(&mut game, 48);
    assert!(!game.flag(&flag("patrol_sent")), "two days is too soon");
    wait_hours(&mut game, 24);
    assert!(game.flag(&flag("patrol_sent")), "the third day sends it");
    assert!(!game.flag(&flag("long_ago")), "five days have not passed");
    wait_hours(&mut game, 48);
    assert!(game.flag(&flag("long_ago")), "the fifth day does");
}

#[test]
fn sleeping_and_walking_move_the_same_clock_the_rules_read() {
    let mut game = game();
    stage(&mut game, 10);
    // Eight hours at a time: waiting is what sleep and travel are made of.
    for _ in 0..9 {
        wait_hours(&mut game, 8);
    }
    assert!(game.flag(&flag("patrol_sent")));
}

#[test]
fn the_times_are_saved_and_restored_and_a_refusal_changes_nothing() {
    let mut game = game();
    wait_hours(&mut game, 3);
    stage(&mut game, 10);
    wait_hours(&mut game, 4);
    let save = game.save();
    assert_eq!(save.quest_times.len(), 2, "started and stage 10");
    let loaded = game.restore(&save).expect("the save loads");
    assert_eq!(loaded.save(), save);
    assert_eq!(
        loaded.minutes_since(&id("warned"), QuestEvent::Started),
        game.minutes_since(&id("warned"), QuestEvent::Started)
    );
    let refused = |change: &dyn Fn(&mut Save)| {
        let mut bad = save.clone();
        change(&mut bad);
        game.restore(&bad).expect_err("the save is refused")
    };
    let part = |error: engine::saves::SaveError| error.to_string();
    assert!(
        part(refused(&|s| s.quest_times[0].minute = u64::MAX)).contains("quest_times"),
        "a moment that has not happened yet"
    );
    assert!(part(refused(&|s| s.quest_times[0].quest = id("nobody"))).contains("quest_times"));
    assert!(
        part(refused(
            &|s| s.quest_times[1].event = QuestEvent::Stage(QuestStage::new(99))
        ))
        .contains("quest_times")
    );
    assert!(
        part(refused(&|s| {
            let again = s.quest_times[0].clone();
            s.quest_times.push(again);
        }))
        .contains("quest_times")
    );
}

proptest! {
    /// However the waiting is split, the days since a stage are the same: the rule
    /// reads the clock, not the path to it.
    #[test]
    fn the_days_since_a_stage_do_not_depend_on_how_the_time_was_split(
        steps in prop::collection::vec(1u32..30, 0..12),
    ) {
        let mut stepped = game();
        stage(&mut stepped, 10);
        let mut at_once = stepped.clone();
        for hours in &steps {
            wait_hours(&mut stepped, *hours);
        }
        let total: u32 = steps.iter().sum();
        if total > 0 {
            wait_hours(&mut at_once, total);
        }
        let quest = id("warned");
        let event = QuestEvent::Stage(QuestStage::new(10));
        prop_assert_eq!(
            stepped.minutes_since(&quest, event),
            at_once.minutes_since(&quest, event)
        );
        prop_assert_eq!(stepped.flag(&flag("patrol_sent")), at_once.flag(&flag("patrol_sent")));
        prop_assert_eq!(stepped.flag(&flag("long_ago")), at_once.flag(&flag("long_ago")));
    }
}
