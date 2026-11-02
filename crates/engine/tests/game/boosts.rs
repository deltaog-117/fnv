//! Boost items through `Game`: they change the character's numbers, end by
//! themselves as the clock moves, and never leave the player with a hit-point pool
//! they cannot keep.

use std::num::NonZeroU32;
use std::sync::Arc;

use engine::clock::Span;
use engine::game::Game;
use engine::inventory::{InventoryError, ItemBook, ItemClass};
use engine::quests::QuestBook;
use engine::shared::{
    CharacterView, InventorySink, InventoryView, ItemId, ItemUse, Quest, Special,
};
use proptest::prelude::*;
use serde_json::json;

fn item(text: &str) -> ItemId {
    ItemId::try_new(text.to_owned()).expect("literal is a valid id")
}

fn count(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).expect("literal is not zero")
}

fn minutes(n: u32) -> Span {
    Span::minutes(count(n))
}

fn used(value: serde_json::Value) -> ItemUse {
    serde_json::from_value(value).expect("the literal is a valid use")
}

/// Buffout: +60 maximum hit points, +3 Endurance and +2 Strength for 4 minutes.
/// Powder: 18 hit points at once and -1 Perception for 1 minute.
fn game(wounds: u32) -> Game {
    let quests = QuestBook::new(std::iter::empty::<&Quest>()).expect("no quests is a valid book");
    let buffout = used(json!({"boost": {"effects": [
        {"stat": "max_hp", "amount": 60, "minutes": 4},
        {"stat": "endurance", "amount": 3, "minutes": 4},
        {"stat": "strength", "amount": 2, "minutes": 4},
    ]}}));
    let powder = used(json!({"boost": {"heal": 18, "effects": [
        {"stat": "perception", "amount": -1, "minutes": 1},
    ]}}));
    let items = ItemBook::new([
        (item("buffout"), ItemClass::Consumable, 0.0),
        (item("powder"), ItemClass::Consumable, 0.0),
    ])
    .and_then(|book| book.with_uses([(item("buffout"), buffout), (item("powder"), powder)]))
    .expect("the literal entries are valid");
    let mut game = Game::new(Arc::new(quests))
        .with_items(Arc::new(items))
        .with_wounds(wounds);
    for name in ["buffout", "powder"] {
        game.give_item(&item(name), count(5)).expect("in the book");
    }
    game
}

#[test]
fn buffout_raises_the_numbers_it_names_and_they_return_exactly_when_it_ends() {
    let mut game = game(0);
    let (hp, carry) = (game.character().max_hp(), game.character().carry_weight());
    assert_eq!(game.use_item(&item("buffout")), Ok(0));
    // +60 from the boost and +3 Endurance at 20 hit points each.
    assert_eq!(game.character().max_hp(), hp + 120);
    assert_eq!(game.character().carry_weight(), carry + 20);
    game.wait(minutes(3)).expect("time passes");
    assert_eq!(game.character().max_hp(), hp + 120, "a minute left");
    game.wait(minutes(1)).expect("time passes");
    assert_eq!(game.character().max_hp(), hp);
    assert_eq!(game.character().carry_weight(), carry);
    assert_eq!(game.active_effects().count(), 0);
    assert_eq!(game.inventory().item_count(&item("buffout")), 4);
}

#[test]
fn the_earned_attributes_are_never_touched_by_a_boost() {
    let mut game = game(0);
    let before = *game.character().attributes();
    game.use_item(&item("buffout")).expect("used");
    assert_eq!(*game.character().attributes(), before);
    assert_eq!(game.character().special(Special::Endurance).get(), 8);
}

#[test]
fn the_hit_points_a_player_has_stay_when_the_boost_ends() {
    let mut game = game(190);
    let before = game.hp();
    game.use_item(&item("buffout")).expect("used");
    assert_eq!(game.hp(), before + 120);
    game.wait(minutes(4)).expect("time passes");
    assert_eq!(game.hp(), before);
}

#[test]
fn a_boosted_health_pool_falls_to_the_new_maximum_and_does_not_kill() {
    let mut game = game(0);
    let base = game.character().max_hp();
    game.use_item(&item("buffout")).expect("used");
    game.heal(u32::MAX);
    assert_eq!(game.hp(), base + 120);
    game.wait(minutes(4)).expect("time passes");
    assert_eq!(game.hp(), base);
    assert!(game.is_alive());
}

#[test]
fn a_second_dose_replaces_the_first_and_restarts_the_four_minutes() {
    let mut game = game(0);
    let base = game.character().max_hp();
    game.use_item(&item("buffout")).expect("used");
    game.wait(minutes(3)).expect("time passes");
    game.use_item(&item("buffout")).expect("used");
    assert_eq!(game.character().max_hp(), base + 120, "it did not stack");
    game.wait(minutes(3)).expect("time passes");
    assert_eq!(game.character().max_hp(), base + 120, "the clock restarted");
    game.wait(minutes(1)).expect("time passes");
    assert_eq!(game.character().max_hp(), base);
}

#[test]
fn healing_powder_heals_at_once_and_costs_a_point_of_perception_for_a_minute() {
    let mut game = game(50);
    assert_eq!(game.character().special(Special::Perception).get(), 5);
    assert_eq!(game.use_item(&item("powder")), Ok(18));
    assert_eq!(game.character().special(Special::Perception).get(), 4);
    game.wait(minutes(1)).expect("time passes");
    assert_eq!(game.character().special(Special::Perception).get(), 5);
}

#[test]
fn an_item_that_heals_is_kept_by_a_player_with_nothing_to_heal() {
    let mut game = game(0);
    assert_eq!(
        game.use_item(&item("powder")),
        Err(InventoryError::AtFullHealth)
    );
    assert_eq!(game.inventory().item_count(&item("powder")), 5);
    assert_eq!(game.active_effects().count(), 0);
}

#[test]
fn a_player_who_is_down_cannot_take_a_chem() {
    let mut game = game(u32::MAX);
    assert_eq!(game.use_item(&item("buffout")), Err(InventoryError::Down));
    assert_eq!(game.active_effects().count(), 0);
}

#[derive(Debug, Clone)]
enum Step {
    Buffout,
    Powder,
    Wait(u32),
    Heal(u32),
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        Just(Step::Buffout),
        Just(Step::Powder),
        (1_u32..9).prop_map(Step::Wait),
        (0_u32..400).prop_map(Step::Heal),
    ]
}

proptest! {
    #[test]
    fn however_chems_and_time_mix_a_living_player_keeps_a_legal_pool(
        wounds in 0_u32..150, steps in prop::collection::vec(step(), 0..40),
    ) {
        let mut game = game(wounds);
        let base = game.character().max_hp();
        for step in steps {
            match step {
                Step::Buffout => { let _ = game.use_item(&item("buffout")); }
                Step::Powder => { let _ = game.use_item(&item("powder")); }
                Step::Wait(m) => { game.wait(minutes(m)).expect("time passes"); }
                Step::Heal(n) => game.heal(n),
            }
            prop_assert!(game.is_alive(), "a chem or its end killed the player");
            prop_assert!(game.hp() <= game.character().max_hp());
            let now = game.clock().as_minutes();
            prop_assert!(game.active_effects().all(|a| a.until > now));
            if game.active_effects().next().is_none() {
                prop_assert_eq!(game.character().max_hp(), base, "numbers did not return");
            }
        }
    }
}

#[test]
fn damage_taken_while_boosted_cannot_kill_when_the_boost_ends() {
    let mut game = game(190);
    game.use_item(&item("buffout")).expect("used");
    // 125 hit points of damage: more than the earned pool could take.
    let hp = game.hp();
    let wounded = {
        let max = game.character().max_hp();
        max - (hp - 125)
    };
    let mut hurt = game.clone().with_wounds(wounded);
    assert_eq!(hurt.hp(), hp - 125);
    hurt.wait(minutes(4)).expect("time passes");
    assert_eq!(hurt.hp(), 1, "left alive on the last hit point");
    assert!(hurt.is_alive());
}
