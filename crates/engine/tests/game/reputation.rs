//! Fame, infamy and karma through the composed `Game`.

use std::sync::Arc;

use engine::game::Game;
use engine::quests::QuestBook;
use engine::reputation::ReputationBook;
use engine::shared::{
    Condition, Effect, Faction, Quest, QuestId, QuestStatus, QuestView, Renown, StandingView,
    apply_all, evaluate,
};
use serde_json::{Value, json};

fn effect(value: Value) -> Effect {
    serde_json::from_value(value).expect("the literal is a valid effect")
}

fn condition(value: Value) -> Condition {
    serde_json::from_value(value).expect("the literal is a valid condition")
}

fn faction(id: &str) -> engine::shared::FactionId {
    engine::shared::FactionId::try_new(id.to_owned()).expect("literal is a valid id")
}

/// A game with one town (tiers at 3, 8 and 15 points) and two quests: `fame` pays
/// fame on its only stage; `broken` pays fame and then names a stage that does
/// not exist, so entering its stage must fail as a whole.
fn game() -> Game {
    let town: Faction = serde_json::from_value(json!({
        "id": "town", "name": "town", "kind": "town", "dlc": "base",
        "reputation": {
            "thresholds": [0, 3, 8, 15],
            "titles": [
                ["Neutral", "Accepted", "Liked", "Idolized"],
                ["Shunned", "Mixed", "a", "b"],
                ["Hated", "c", "d", "e"],
                ["Vilified", "f", "g", "h"],
            ],
        },
    }))
    .expect("the literal is a valid faction record");
    let quests: Vec<Quest> = [
        json!({
            "id": "fame", "name": "fame", "kind": "side", "dlc": "base",
            "stages": [{"stage": 5, "description": "s", "on_enter": [
                {"type": "reputation", "faction": "town", "fame": 4, "infamy": 0},
                {"type": "karma", "amount": 300},
            ]}],
        }),
        json!({
            "id": "broken", "name": "broken", "kind": "side", "dlc": "base",
            "stages": [{"stage": 5, "description": "s", "on_enter": [
                {"type": "reputation", "faction": "town", "fame": 20, "infamy": 0},
                {"type": "set_stage", "quest": "fame", "stage": 99},
            ]}],
        }),
    ]
    .into_iter()
    .map(|value| serde_json::from_value(value).expect("the literal is a valid quest record"))
    .collect();
    Game::new(Arc::new(
        QuestBook::new(quests.iter()).expect("stages are in order"),
    ))
    .with_reputation(Arc::new(
        ReputationBook::new([&town]).expect("the town builds"),
    ))
}

#[test]
fn reputation_and_karma_effects_apply_instead_of_being_deferred() {
    let mut game = game();
    apply_all(
        &[
            effect(json!({"type": "reputation", "faction": "town", "fame": 9, "infamy": 1})),
            effect(json!({"type": "karma", "amount": -400})),
        ],
        &mut game,
    )
    .expect("both apply");
    assert!(game.deferred().is_empty());
    assert_eq!(game.renown_points(&faction("town"), Renown::Fame), 9.0);
    assert_eq!(game.standing().title(&faction("town")), Some("Liked"));
    assert_eq!(game.standing().karma().get(), -400.0);
}

#[test]
fn conditions_read_the_standing_the_effects_wrote() {
    let mut game = game();
    let liked = condition(json!({"type": "reputation", "faction": "town", "level": "liked"}));
    let tier_two = condition(
        json!({"type": "reputation_tier", "faction": "town", "renown": "fame", "tier": 2}),
    );
    let fame_ten = condition(
        json!({"type": "reputation_points", "faction": "town", "renown": "fame", "min": 10}),
    );
    let evil = condition(json!({"type": "karma", "alignment": "evil"}));
    assert!(!evaluate(&liked, &game));
    assert!(!evaluate(&tier_two, &game));

    apply_all(
        &[
            effect(json!({"type": "reputation", "faction": "town", "fame": 9, "infamy": 0})),
            effect(json!({"type": "karma", "amount": -300})),
        ],
        &mut game,
    )
    .expect("both apply");
    assert!(evaluate(&liked, &game));
    assert!(evaluate(&tier_two, &game));
    assert!(!evaluate(&fame_ten, &game));
    assert!(evaluate(&evil, &game));

    apply_all(
        &[effect(
            json!({"type": "reputation", "faction": "town", "fame": 1, "infamy": 0}),
        )],
        &mut game,
    )
    .expect("applies");
    assert!(evaluate(&fame_ten, &game));
}

#[test]
fn a_stage_reward_pays_reputation_and_karma_once() {
    let mut game = game();
    let set = |game: &mut Game| {
        apply_all(
            &[effect(
                json!({"type": "set_stage", "quest": "fame", "stage": 5}),
            )],
            game,
        )
    };
    set(&mut game).expect("the stage is reached");
    set(&mut game).expect("the stage is set again");
    assert_eq!(game.renown_points(&faction("town"), Renown::Fame), 4.0);
    assert_eq!(game.standing().karma().get(), 300.0);
    assert!(game.deferred().is_empty());
}

#[test]
fn a_stage_that_fails_part_way_leaves_the_standing_as_it_was() {
    let mut game = game();
    let result = apply_all(
        &[effect(
            json!({"type": "set_stage", "quest": "broken", "stage": 5}),
        )],
        &mut game,
    );
    assert!(result.is_err());
    assert_eq!(game.renown_points(&faction("town"), Renown::Fame), 0.0);
    assert_eq!(
        game.quest_status(&QuestId::try_new("broken").expect("valid id")),
        QuestStatus::NotStarted
    );
}

#[test]
fn a_game_with_no_reputation_book_refuses_reputation_but_still_tracks_karma() {
    let mut game = Game::new(Arc::default());
    let refused = apply_all(
        &[effect(
            json!({"type": "reputation", "faction": "town", "fame": 1, "infamy": 0}),
        )],
        &mut game,
    );
    assert!(refused.is_err());
    apply_all(&[effect(json!({"type": "karma", "amount": 5}))], &mut game).expect("applies");
    assert_eq!(game.standing().karma().get(), 5.0);
}
