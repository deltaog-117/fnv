//! Hit points through `Game`: wounds are read as a percentage by conditions, and the
//! heal effect clears them.

use std::sync::Arc;

use engine::game::Game;
use engine::quests::QuestBook;
use engine::shared::{
    CharacterView, Comparison, Condition, Effect, HealthPercent, Quest, apply, evaluate,
};
use proptest::prelude::*;

fn game() -> Game {
    let quests = QuestBook::new(std::iter::empty::<&Quest>()).expect("no quests is a valid book");
    Game::new(Arc::new(quests))
}

fn health_is(percent: u32) -> Condition {
    Condition::Health {
        percent: HealthPercent::try_new(percent).expect("in range"),
        op: Comparison::Is,
    }
}

#[test]
fn a_new_game_is_at_full_health() {
    let game = game();
    assert_eq!(game.health(), HealthPercent::FULL);
    assert!(evaluate(&health_is(100), &game));
}

#[test]
fn wounds_read_as_a_rounded_down_percentage_of_the_maximum() {
    let max = game().character().max_hp();
    let wounded = game().with_wounds(max / 2);
    assert_eq!(wounded.hp(), max - max / 2);
    assert_eq!(
        wounded.health(),
        HealthPercent::of(wounded.hp(), max),
        "the view and the hit points must agree"
    );
    let one_wound = game().with_wounds(1);
    assert!(
        !evaluate(&health_is(100), &one_wound),
        "a single wound is not full health"
    );
}

#[test]
fn the_heal_effect_clears_every_wound() {
    let max = game().character().max_hp();
    let mut game = game().with_wounds(max - 1);
    assert_eq!(game.hp(), 1);
    apply(&Effect::Heal {}, &mut game).expect("healing is built");
    assert_eq!(game.hp(), max);
    assert!(evaluate(&health_is(100), &game));
}

#[test]
fn wounds_cannot_exceed_the_maximum() {
    let max = game().character().max_hp();
    let game = game().with_wounds(u32::MAX);
    assert_eq!(game.hp(), 0);
    assert_eq!(game.health().get(), 0);
    assert!(!game.is_alive());
    assert_eq!(game.character().max_hp(), max);
}

proptest! {
    #[test]
    fn healing_always_ends_at_full_and_wounds_never_read_above_full(wounds in 0u32..1_000) {
        let mut game = game().with_wounds(wounds);
        prop_assert!(game.health().get() <= 100);
        apply(&Effect::Heal {}, &mut game).expect("healing is built");
        prop_assert_eq!(game.health(), HealthPercent::FULL);
        prop_assert_eq!(game.hp(), game.character().max_hp());
    }
}
