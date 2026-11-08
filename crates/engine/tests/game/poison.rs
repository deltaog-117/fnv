//! Poison through `Game`: venom that gets in past resistance, the hit points it
//! costs as time passes, and antidotes.

use std::num::NonZeroU32;
use std::sync::Arc;

use engine::clock::Span;
use engine::combat::Dice;
use engine::game::Game;
use engine::inventory::{InventoryError, ItemBook, ItemClass};
use engine::poison::DRAIN_PER_MINUTE;
use engine::quests::QuestBook;
use engine::shared::{
    Ailment, Effect, InventorySink, InventoryView, ItemId, ItemUse, Quest, apply,
};
use serde_json::json;

/// Never rolls low, so nothing here addicts.
struct Calm;

impl Dice for Calm {
    fn percent(&mut self) -> u8 {
        100
    }
}

fn take(game: &mut Game, name: &str) -> Result<engine::game::Used, InventoryError> {
    game.use_item_rolling(&item(name), &mut Calm)
}

fn item(text: &str) -> ItemId {
    ItemId::try_new(text.to_owned()).expect("literal is a valid id")
}

fn minutes(n: u32) -> Span {
    Span::minutes(NonZeroU32::new(n).expect("literal is not zero"))
}

fn used(value: serde_json::Value) -> ItemUse {
    serde_json::from_value(value).expect("the literal is a valid use")
}

/// Venom (60 poison), an antivenom, a purge that also cleans rads, and Blood Shield
/// (+50 poison resistance for 2 minutes). The default character has Endurance 5, so
/// 20% poison resistance of its own.
fn game() -> Game {
    let quests = QuestBook::new(std::iter::empty::<&Quest>()).expect("no quests is a valid book");
    let uses = [
        ("venom", json!({"boost": {"poison": 60}})),
        ("antivenom", json!({"boost": {"cure_poison": true}})),
        (
            "shield",
            json!({"boost": {"effects": [{"stat": "poison_resistance", "amount": 50, "minutes": 2}]}}),
        ),
    ];
    let items = ItemBook::new(
        uses.iter()
            .map(|(name, _)| (item(name), ItemClass::Consumable, 0.0)),
    )
    .and_then(|book| {
        book.with_uses(
            uses.iter()
                .map(|(name, value)| (item(name), used(value.clone()))),
        )
    })
    .expect("the literal entries are valid");
    let mut game = Game::new(Arc::new(quests)).with_items(Arc::new(items));
    for (name, _) in &uses {
        game.give_item(&item(name), NonZeroU32::new(10).expect("not zero"))
            .expect("in the book");
    }
    game
}

#[test]
fn venom_gets_in_after_the_players_own_resistance() {
    let mut game = game();
    assert_eq!(game.poison_resistance(), 20, "5 * (5 - 1)");
    let used = take(&mut game, "venom").expect("used");
    assert_eq!(used.poison_in, 48, "60 with 20% kept out");
    assert_eq!(game.poison(), 48);
}

#[test]
fn a_resistance_boost_keeps_more_out_until_it_wears_off() {
    let mut game = game();
    game.use_item(&item("shield")).expect("used");
    assert_eq!(game.poison_resistance(), 70);
    let used = take(&mut game, "venom").expect("used");
    assert_eq!(used.poison_in, 18, "60 with 70% kept out");
    game.wait(minutes(2)).expect("time passes");
    assert_eq!(game.poison_resistance(), 20);
}

#[test]
fn time_turns_poison_into_wounds_at_a_fixed_rate() {
    let mut game = game();
    take(&mut game, "venom").expect("used");
    let hp = game.hp();
    game.wait(minutes(2)).expect("time passes");
    let cost = 2 * u32::from(DRAIN_PER_MINUTE);
    assert_eq!(game.hp(), hp - cost);
    assert_eq!(game.poison(), 48 - 2 * DRAIN_PER_MINUTE);
}

#[test]
fn poison_alone_never_takes_the_last_hit_point() {
    let mut game = game();
    for _ in 0..10 {
        take(&mut game, "venom").expect("used");
    }
    game.wait(minutes(60 * 24)).expect("time passes");
    assert!(game.is_alive());
    assert_eq!(game.hp(), 1);
    assert_eq!(game.poison(), 0, "all of it was paid out");
}

#[test]
fn an_antivenom_ends_the_poison_and_is_kept_when_there_is_none() {
    let mut game = game();
    assert_eq!(
        take(&mut game, "antivenom").map(|_| ()),
        Err(InventoryError::NotPoisoned)
    );
    assert_eq!(
        game.inventory().item_count(&item("antivenom")),
        10,
        "not spent"
    );
    take(&mut game, "venom").expect("used");
    let used = take(&mut game, "antivenom").expect("used");
    assert!(used.cured_poison);
    assert_eq!(game.poison(), 0);
    assert_eq!(game.inventory().item_count(&item("antivenom")), 9);
}

#[test]
fn a_cure_effect_clears_poison() {
    let mut game = game().with_poison(300);
    apply(
        &Effect::Cure {
            ailment: Ailment::Poison,
        },
        &mut game,
    )
    .expect("cured");
    assert_eq!(game.poison(), 0);
}
