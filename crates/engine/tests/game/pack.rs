//! The character and the pack through `Game`: effects in, conditions out.

use std::sync::Arc;

use engine::character::{Character, CharacterRules, SpecialSet};
use engine::game::Game;
use engine::inventory::{ItemBook, ItemClass, Slot};
use engine::quests::QuestBook;
use engine::shared::{
    ArmorSlot, Condition, Effect, InventoryView, ItemId, Quest, QuestId, QuestStatus, QuestView,
    Special, SpecialLevel, apply_all, evaluate,
};
use serde_json::{Value, json};

fn item(text: &str) -> ItemId {
    ItemId::try_new(text.to_owned()).expect("literal is a valid id")
}

/// A quest whose stage 10 pays a pistol and caps, and whose stage 20 pays caps and
/// an item that does not exist, so entering it must fail as a whole.
fn game() -> Game {
    let quest: Quest = serde_json::from_value(json!({
        "id": "q", "name": "q", "kind": "side", "dlc": "base",
        "stages": [
            {"stage": 10, "description": "s", "on_enter": [
                {"type": "give_item", "item": "pistol"},
                {"type": "caps", "amount": 18},
                {"type": "xp", "amount": 200}]},
            {"stage": 20, "description": "s", "finish": true, "on_enter": [
                {"type": "caps", "amount": 100},
                {"type": "give_item", "item": "ghost"}]},
        ],
    }))
    .expect("the literal is a valid quest record");
    let items = ItemBook::new([
        (item("pistol"), ItemClass::Weapon, 3.0),
        (item("anvil"), ItemClass::Consumable, 400.0),
        (item("hat"), ItemClass::Armor(ArmorSlot::Head), 1.0),
    ])
    .expect("the literal entries are valid");
    Game::new(Arc::new(
        QuestBook::new([&quest]).expect("stages are in order"),
    ))
    .with_items(Arc::new(items))
}

fn effect(value: Value) -> Effect {
    serde_json::from_value(value).expect("the literal is a valid effect")
}

fn condition(value: Value) -> Condition {
    serde_json::from_value(value).expect("the literal is a valid condition")
}

#[test]
fn a_stage_reward_lands_in_the_pack_and_the_character() {
    let mut game = game();
    apply_all(
        &[effect(
            json!({"type": "set_stage", "quest": "q", "stage": 10}),
        )],
        &mut game,
    )
    .expect("the stage is reached");
    assert_eq!(game.caps(), 18);
    assert_eq!(game.item_count(&item("pistol")), 1);
    assert_eq!(game.character().xp(), 200);
    assert_eq!(game.character().level().get(), 2);
    assert!(game.deferred().is_empty());
}

#[test]
fn conditions_read_the_pack_and_the_character() {
    let mut game = game();
    apply_all(
        &[effect(
            json!({"type": "set_stage", "quest": "q", "stage": 10}),
        )],
        &mut game,
    )
    .expect("the stage is reached");
    assert!(evaluate(
        &condition(json!({"type": "has_item", "item": "pistol"})),
        &game
    ));
    assert!(evaluate(
        &condition(json!({"type": "special", "stat": "luck", "min": 5})),
        &game
    ));
    assert!(!evaluate(
        &condition(json!({"type": "has_item", "item": "hat"})),
        &game
    ));
}

#[test]
fn a_reward_that_fails_halfway_undoes_the_whole_stage() {
    let mut game = game();
    let id = QuestId::try_new("q".to_owned()).expect("valid id");
    let error = apply_all(
        &[effect(
            json!({"type": "set_stage", "quest": "q", "stage": 20}),
        )],
        &mut game,
    )
    .expect_err("the item does not exist");
    assert_eq!(error.tag, "set_stage");
    assert_eq!(game.quest_status(&id), QuestStatus::NotStarted);
    assert_eq!(
        game.caps(),
        0,
        "the caps paid before the failure are rolled back"
    );
}

#[test]
fn equipping_goes_through_the_game() {
    let mut game = game();
    apply_all(
        &[effect(json!({"type": "give_item", "item": "pistol"}))],
        &mut game,
    )
    .expect("held");
    assert_eq!(game.equip(&item("pistol")), Ok(Slot::Weapon));
    assert!(game.is_equipped(&item("pistol")));
    assert_eq!(game.unequip(Slot::Weapon), Some(item("pistol")));
}

#[test]
fn being_over_the_carry_weight_blocks_nothing_but_is_reported() {
    let weak = Character::new(
        SpecialSet::default().with(Special::Strength, SpecialLevel::try_new(1).expect("legal")),
        CharacterRules::standard(),
    );
    let mut game = game().with_character(weak);
    assert!(!game.is_encumbered());
    // 150 + 10 * 1 = 160 pounds can be carried; one anvil weighs 400.
    apply_all(
        &[effect(json!({"type": "give_item", "item": "anvil"}))],
        &mut game,
    )
    .expect("picking it up is allowed");
    assert!(game.is_encumbered());
}
