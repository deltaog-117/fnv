//! Using consumables and throwing items away through `Game`.

use std::num::NonZeroU32;
use std::sync::Arc;

use engine::game::Game;
use engine::inventory::{InventoryError, ItemBook, ItemClass, Slot};
use engine::quests::QuestBook;
use engine::shared::{InventorySink, InventoryView, ItemId, ItemUse, Quest};
use proptest::prelude::*;

fn item(text: &str) -> ItemId {
    ItemId::try_new(text.to_owned()).expect("literal is a valid id")
}

fn count(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).expect("literal is not zero")
}

/// A stimpak that heals 30, a bottle that models no use, and a pistol.
fn game(wounds: u32) -> Game {
    let quests = QuestBook::new(std::iter::empty::<&Quest>()).expect("no quests is a valid book");
    let items = ItemBook::new([
        (item("stimpak"), ItemClass::Consumable, 0.0),
        (item("bottle"), ItemClass::Consumable, 1.0),
        (item("pistol"), ItemClass::Weapon, 3.0),
    ])
    .and_then(|book| book.with_uses([(item("stimpak"), ItemUse::Heal(count(30)))]))
    .expect("the literal entries are valid");
    let mut game = Game::new(Arc::new(quests))
        .with_items(Arc::new(items))
        .with_wounds(wounds);
    for (name, n) in [("stimpak", 3), ("bottle", 1), ("pistol", 1)] {
        game.give_item(&item(name), count(n)).expect("in the book");
    }
    game
}

#[test]
fn a_stimpak_heals_and_is_spent() {
    let mut game = game(50);
    let before = game.hp();
    assert_eq!(game.use_item(&item("stimpak")), Ok(30));
    assert_eq!(game.hp(), before + 30);
    assert_eq!(game.inventory().item_count(&item("stimpak")), 2);
}

#[test]
fn healing_stops_at_the_maximum_and_reports_what_it_restored() {
    let mut game = game(5);
    assert_eq!(game.use_item(&item("stimpak")), Ok(5));
    assert_eq!(game.hp(), game.character().max_hp());
}

#[test]
fn a_player_at_full_health_keeps_the_stimpak() {
    let mut game = game(0);
    assert_eq!(
        game.use_item(&item("stimpak")),
        Err(InventoryError::AtFullHealth)
    );
    assert_eq!(game.inventory().item_count(&item("stimpak")), 3);
}

#[test]
fn an_item_with_no_modeled_effect_is_refused_and_kept() {
    let mut game = game(10);
    for name in ["bottle", "pistol"] {
        assert_eq!(
            game.use_item(&item(name)),
            Err(InventoryError::NoEffect(item(name)))
        );
        assert_eq!(game.inventory().item_count(&item(name)), 1);
    }
}

#[test]
fn an_item_that_is_not_carried_cannot_be_used() {
    let mut game = game(10);
    assert_eq!(
        game.use_item(&item("ghost")),
        Err(InventoryError::NotHeld(item("ghost")))
    );
}

#[test]
fn a_player_who_is_down_cannot_use_anything() {
    let mut game = game(u32::MAX);
    assert_eq!(game.use_item(&item("stimpak")), Err(InventoryError::Down));
    assert_eq!(game.inventory().item_count(&item("stimpak")), 3);
}

#[test]
fn dropping_removes_items_and_keeps_an_equipped_weapon() {
    let mut game = game(0);
    game.drop_item(&item("stimpak"), count(2))
        .expect("three held");
    assert_eq!(game.inventory().item_count(&item("stimpak")), 1);
    game.equip(&item("pistol")).expect("a weapon equips");
    assert_eq!(
        game.drop_item(&item("pistol"), count(1)),
        Err(InventoryError::Equipped(item("pistol")))
    );
    game.unequip(Slot::Weapon);
    game.drop_item(&item("pistol"), count(1))
        .expect("unequipped");
    assert_eq!(game.inventory().item_count(&item("pistol")), 0);
}

proptest! {
    #[test]
    fn using_items_never_exceeds_the_maximum_and_a_refusal_changes_nothing(
        wounds in 0_u32..400,
        uses in proptest::collection::vec(0_usize..3, 0..12)
    ) {
        let mut game = game(wounds);
        let max = game.character().max_hp();
        for which in uses {
            let name = ["stimpak", "bottle", "ghost"][which];
            let (hp, held) = (game.hp(), game.inventory().item_count(&item(name)));
            match game.use_item(&item(name)) {
                Ok(healed) => {
                    prop_assert_eq!(game.hp(), hp + healed);
                    prop_assert_eq!(game.inventory().item_count(&item(name)), held - 1);
                }
                Err(_) => {
                    prop_assert_eq!(game.hp(), hp);
                    prop_assert_eq!(game.inventory().item_count(&item(name)), held);
                }
            }
            prop_assert!(game.hp() <= max);
        }
    }
}
