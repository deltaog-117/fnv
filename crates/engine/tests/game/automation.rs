//! Stage effects and consequence rules fired by `Game` on hand-built quests.

use std::sync::Arc;

use engine::game::Game;
use engine::quests::QuestBook;
use engine::shared::{
    Effect, FlagId, Quest, QuestId, QuestStage, QuestStatus, QuestView, WorldView, apply_all,
};
use proptest::prelude::*;
use serde_json::json;

/// Quest `a` pays caps on stage 10, sets a flag on its finishing stage 20 and
/// locks quest `b` when completed before `b` starts. Quest `c` has a stage whose
/// effect names a stage that does not exist, so entering it must fail.
fn game() -> Game {
    let quests: Vec<Quest> = [
        json!({
            "id": "a", "name": "a", "kind": "side", "dlc": "base",
            "stages": [
                {"stage": 5, "description": "s"},
                {"stage": 10, "description": "s", "on_enter": [{"type": "caps", "amount": 10.0}]},
                {"stage": 20, "description": "s", "finish": true,
                 "on_enter": [{"type": "set_flag", "flag": "a_done"}]},
            ],
            "rules": [{
                "kind": "locks", "target": "B", "target_type": "quest", "target_id": "b",
                "conditions": [
                    {"type": "quest", "quest": "a", "state": "completed"},
                    {"type": "quest", "quest": "b", "state": "not_started"},
                ],
                "effects": [{"type": "block_quest", "quest": "b"}],
            }],
        }),
        json!({
            "id": "b", "name": "b", "kind": "side", "dlc": "base",
            "stages": [{"stage": 5, "description": "s"}],
        }),
        json!({
            "id": "c", "name": "c", "kind": "side", "dlc": "base",
            "stages": [
                {"stage": 5, "description": "s",
                 "on_enter": [{"type": "set_flag", "flag": "c_entered"},
                              {"type": "set_stage", "quest": "b", "stage": 99}]},
            ],
        }),
    ]
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

fn set_stage(game: &mut Game, quest: &str, stage: u32) -> Result<(), engine::shared::BatchError> {
    let effect = Effect::SetStage {
        quest: id(quest),
        stage: QuestStage::new(stage),
    };
    apply_all(&[effect], game)
}

fn flag(text: &str) -> FlagId {
    FlagId::try_new(text.to_owned()).expect("literal is a valid id")
}

#[test]
fn a_stage_reward_for_an_unbuilt_feature_is_deferred_not_lost() {
    let mut game = game();
    set_stage(&mut game, "a", 10).expect("the stage is reached");
    let deferred = game.deferred();
    assert_eq!(deferred.len(), 1);
    assert_eq!(deferred[0].quest, id("a"));
    assert_eq!(deferred[0].feature, "inventory");
    assert!(matches!(deferred[0].effect, Effect::Caps { .. }));
}

#[test]
fn stage_effects_pay_once_even_if_the_stage_is_set_again() {
    let mut game = game();
    set_stage(&mut game, "a", 10).expect("first time");
    set_stage(&mut game, "a", 10).expect("second time");
    assert_eq!(game.deferred().len(), 1);
}

#[test]
fn skipping_a_stage_skips_its_effects() {
    let mut game = game();
    set_stage(&mut game, "a", 5).expect("starts");
    set_stage(&mut game, "a", 20).expect("jumps to the end");
    assert!(game.deferred().is_empty());
    assert!(game.flag(&flag("a_done")));
}

#[test]
fn a_rule_fires_when_its_conditions_hold_after_the_quest_changes() {
    let mut game = game();
    set_stage(&mut game, "a", 20).expect("finishes");
    assert_eq!(game.quest_status(&id("a")), QuestStatus::Completed);
    assert_eq!(game.quest_status(&id("b")), QuestStatus::Failed);
}

#[test]
fn a_rule_stays_quiet_when_a_condition_fails() {
    let mut game = game();
    set_stage(&mut game, "b", 5).expect("b starts first");
    set_stage(&mut game, "a", 20).expect("finishes");
    assert_eq!(game.quest_status(&id("b")), QuestStatus::Active);
}

#[test]
fn an_effect_that_fails_undoes_the_whole_change() {
    let mut game = game();
    let error = set_stage(&mut game, "c", 5).expect_err("the nested stage does not exist");
    assert_eq!(error.tag, "set_stage");
    assert_eq!(game.quest_status(&id("c")), QuestStatus::NotStarted);
    assert!(
        !game.flag(&flag("c_entered")),
        "the earlier effect is rolled back"
    );
    assert!(game.deferred().is_empty());
}

fn observe(game: &Game) -> (Vec<QuestStatus>, Vec<Option<QuestStage>>, usize, bool) {
    let quests = [id("a"), id("b"), id("c")];
    (
        quests.iter().map(|q| game.quest_status(q)).collect(),
        quests.iter().map(|q| game.quest_stage(q)).collect(),
        game.deferred().len(),
        game.flag(&flag("a_done")),
    )
}

fn step() -> impl Strategy<Value = (&'static str, u32)> {
    prop_oneof![
        Just(("a", 5)),
        Just(("a", 10)),
        Just(("a", 20)),
        Just(("b", 5)),
        Just(("c", 5)),
        Just(("c", 7)),
    ]
}

proptest! {
    #[test]
    fn any_order_of_moves_pays_each_stage_at_most_once_and_a_failure_changes_nothing(
        steps in proptest::collection::vec(step(), 0..24)
    ) {
        let mut game = game();
        let mut entered_ten = false;
        for (quest, stage) in steps {
            let before = observe(&game);
            match set_stage(&mut game, quest, stage) {
                Ok(()) => entered_ten |= (quest, stage) == ("a", 10),
                Err(_) => prop_assert_eq!(observe(&game), before),
            }
            prop_assert!(game.deferred().len() <= 1);
        }
        prop_assert_eq!(game.deferred().len(), usize::from(entered_ten));
    }
}
