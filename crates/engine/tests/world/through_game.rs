//! World effects and conditions through the composed `Game`.

use std::sync::Arc;

use engine::game::Game;
use engine::quests::QuestBook;
use engine::shared::{Condition, Effect, EffectError, apply_all, evaluate};
use serde_json::{Value, json};

use crate::support::{book, npc};

fn game() -> Game {
    Game::with_world(Arc::new(QuestBook::default()), book())
}

fn effect(value: Value) -> Effect {
    serde_json::from_value(value).expect("the literal is a valid effect")
}

fn condition(value: Value) -> Condition {
    serde_json::from_value(value).expect("the literal is a valid condition")
}

#[test]
fn world_effects_change_what_conditions_read() {
    let mut game = game();
    let flag = condition(json!({"type": "flag", "flag": "f"}));
    let dead = condition(json!({"type": "npc", "npc": "doc", "state": "dead"}));
    assert!(!evaluate(&flag, &game) && !evaluate(&dead, &game));
    apply_all(
        &[
            effect(json!({"type": "set_flag", "flag": "f"})),
            effect(json!({"type": "kill_npc", "npc": "doc"})),
        ],
        &mut game,
    )
    .expect("world effects apply");
    assert!(evaluate(&flag, &game) && evaluate(&dead, &game));
}

#[test]
fn an_unknown_npc_stops_the_batch_and_reports_how_far_it_got() {
    let mut game = game();
    let result = apply_all(
        &[
            effect(json!({"type": "set_flag", "flag": "f"})),
            effect(json!({"type": "kill_npc", "npc": "ghost"})),
        ],
        &mut game,
    );
    let error = result.expect_err("ghost is not in the roster");
    assert!(matches!(error.source, EffectError::Unknown { .. }));
    // The pre-flight cannot see roster ids, so the flag before it took hold.
    assert_eq!(error.applied, 1);
    assert!(evaluate(
        &condition(json!({"type": "flag", "flag": "f"})),
        &game
    ));
}

#[test]
fn a_game_without_a_roster_refuses_npc_effects() {
    let mut game = Game::new(Arc::new(QuestBook::default()));
    let result = apply_all(
        &[effect(json!({"type": "kill_npc", "npc": "doc"}))],
        &mut game,
    );
    assert!(result.is_err());
}

#[test]
fn meeting_goes_through_the_game() {
    let mut game = game();
    game.meet(&npc("doc")).expect("alive");
    assert!(evaluate(
        &condition(json!({"type": "npc", "npc": "doc", "state": "met"})),
        &game
    ));
}
