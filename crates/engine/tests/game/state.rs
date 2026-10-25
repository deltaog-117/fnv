//! `Game` against hand-built quests and effects.
//!
//! World effects are covered in `tests/world`.

use std::sync::Arc;

use engine::game::Game;
use engine::quests::QuestBook;
use engine::shared::{
    Condition, Effect, EffectError, Quest, QuestStatus, QuestView, apply_all, evaluate,
};
use proptest::prelude::*;
use serde_json::{Value, json};

fn game() -> Game {
    let stages: Vec<Value> = [(5, false), (10, false), (30, true)]
        .iter()
        .map(|(n, finish)| json!({"stage": n, "description": "step", "finish": finish}))
        .collect();
    let quest: Quest = serde_json::from_value(
        json!({"id": "q", "name": "q", "kind": "side", "dlc": "base", "stages": stages}),
    )
    .expect("the literal is a valid quest record");
    Game::new(Arc::new(
        QuestBook::new([&quest]).expect("stages are in order"),
    ))
}

fn effect(value: Value) -> Effect {
    serde_json::from_value(value).expect("the literal is a valid effect")
}

fn quest_id() -> engine::shared::QuestId {
    engine::shared::QuestId::try_new("q".to_owned()).expect("literal is a valid id")
}

#[test]
fn quest_effects_reach_the_quest_log() {
    let mut game = game();
    apply_all(
        &[
            effect(json!({"type": "start_quest", "quest": "q"})),
            effect(json!({"type": "set_stage", "quest": "q", "stage": 10})),
        ],
        &mut game,
    )
    .expect("quest effects apply");
    assert_eq!(game.quest_status(&quest_id()), QuestStatus::Active);
    assert_eq!(game.quests().active().count(), 1);
}

#[test]
fn a_quest_condition_reads_the_quest_log() {
    let mut game = game();
    let active: Condition =
        serde_json::from_value(json!({"type": "quest", "quest": "q", "state": "active"}))
            .expect("the literal is a valid condition");
    assert!(!evaluate(&active, &game));
    apply_all(
        &[effect(json!({"type": "start_quest", "quest": "q"}))],
        &mut game,
    )
    .expect("starts");
    assert!(evaluate(&active, &game));
}

#[test]
fn reads_about_unbuilt_features_give_the_defaults() {
    let game = game();
    let condition: Condition = serde_json::from_value(json!({"type": "flag", "flag": "any"}))
        .expect("the literal is a valid condition");
    assert!(!evaluate(&condition, &game));
}

fn unbuilt_effects() -> Vec<Value> {
    vec![
        json!({"type": "reputation", "faction": "ncr", "fame": 1.0, "infamy": 0.0}),
        json!({"type": "karma", "amount": 5.0}),
        json!({"type": "companion_join", "companion": "c"}),
    ]
}

proptest! {
    #[test]
    fn an_effect_on_an_unbuilt_feature_is_refused_and_changes_nothing(
        index in 0..unbuilt_effects().len()
    ) {
        let mut game = game();
        apply_all(&[effect(json!({"type": "start_quest", "quest": "q"}))], &mut game)
            .expect("starts");
        let before = (game.quest_status(&quest_id()), game.quest_stage(&quest_id()));

        let wanted = effect(unbuilt_effects()[index].clone());
        let result = apply_all(std::slice::from_ref(&wanted), &mut game);

        let error = result.expect_err("the feature is not built");
        let not_built = matches!(
            error.source,
            EffectError::NotBuilt { effect, .. } if effect == wanted.tag()
        );
        prop_assert!(not_built);
        prop_assert_eq!(
            (game.quest_status(&quest_id()), game.quest_stage(&quest_id())),
            before
        );
    }
}
