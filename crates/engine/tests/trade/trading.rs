//! Buying and selling through `Game`, one rule at a time.

use std::sync::Arc;

use engine::game::Game;
use engine::inventory::{ItemBook, ItemClass};
use engine::shared::{Effect, InventorySink, InventoryView, apply};
use engine::trade::{TradeError, VendorBook};

use crate::support::{
    character, count, game, game_with, item, items, npc, place, record, shopkeeper, vendor,
};

fn shop_ammo(game: &Game) -> u32 {
    game.shop(&vendor("shopkeeper"))
        .expect("the shop exists")
        .count(&item("ammo"))
}

fn shop_caps(game: &Game) -> u32 {
    game.shop(&vendor("shopkeeper"))
        .expect("the shop exists")
        .caps()
}

#[test]
fn a_vendor_is_found_at_its_place_only() {
    let game = game(1, 0);
    let town = place("town");
    let found: Vec<_> = game.vendors_at(&town).map(|(id, _)| id.as_str()).collect();
    assert_eq!(found, ["shopkeeper"]);
    assert_eq!(game.vendors_at(&place("elsewhere")).count(), 0);
}

#[test]
fn buying_moves_items_and_caps_between_the_two() {
    // Charisma 1 gives Barter 5: 10 rounds worth 10 each cost 100 * 1.5275 = 153.
    let mut game = game(1, 500);
    let paid = game
        .buy(&vendor("shopkeeper"), &item("ammo"), count(10))
        .expect("a fair trade");
    assert_eq!(paid, 153);
    assert_eq!(game.caps(), 347);
    assert_eq!(game.item_count(&item("ammo")), 10);
    assert_eq!(shop_ammo(&game), 10);
    assert_eq!(shop_caps(&game), 453);
}

#[test]
fn selling_moves_items_and_caps_the_other_way() {
    // 4 rounds worth 10 each pay 40 * 0.4725 = 18.9, which is 19.
    let mut game = game(1, 0);
    game.give_item(&item("ammo"), count(4)).expect("given");
    let paid = game
        .sell(&vendor("shopkeeper"), &item("ammo"), count(4))
        .expect("a fair trade");
    assert_eq!(paid, 19);
    assert_eq!(game.caps(), 19);
    assert_eq!(game.item_count(&item("ammo")), 0);
    assert_eq!(shop_ammo(&game), 24);
    assert_eq!(shop_caps(&game), 281);
}

#[test]
fn a_better_barter_skill_buys_cheaper_and_sells_dearer() {
    let (poor, skilled) = (game(1, 0), game(10, 0));
    let line = count(5);
    assert!(
        skilled.buy_price(&item("pistol"), line).expect("priced")
            < poor.buy_price(&item("pistol"), line).expect("priced")
    );
    assert!(
        skilled.sell_price(&item("pistol"), line).expect("priced")
            > poor.sell_price(&item("pistol"), line).expect("priced")
    );
}

#[test]
fn a_refused_buy_changes_nothing() {
    let mut game = game(1, 500);
    let cases = [
        (item("ammo"), 21, "more than the vendor has"),
        (item("hat"), 1, "an item the vendor does not stock"),
    ];
    for (what, n, why) in cases {
        let error = game
            .buy(&vendor("shopkeeper"), &what, count(n))
            .expect_err(why);
        assert!(
            matches!(error, TradeError::OutOfStock { .. }),
            "{why}: {error:?}"
        );
        assert_eq!(
            (game.caps(), shop_caps(&game), shop_ammo(&game)),
            (500, 300, 20),
            "{why}"
        );
    }
    let mut broke = game_with(character(1));
    let error = broke
        .buy(&vendor("shopkeeper"), &item("pistol"), count(1))
        .expect_err("no caps");
    assert_eq!(error, TradeError::PlayerShort { have: 0, need: 153 });
    assert_eq!(shop_caps(&broke), 300);
}

#[test]
fn the_last_item_leaves_the_stock_entirely() {
    let mut game = game(1, 1000);
    game.buy(&vendor("shopkeeper"), &item("pistol"), count(1))
        .expect("in stock");
    let shop = game.shop(&vendor("shopkeeper")).expect("the shop exists");
    assert_eq!(shop.count(&item("pistol")), 0);
    assert!(shop.stock().all(|(id, _)| id != &item("pistol")));
}

#[test]
fn a_vendor_who_cannot_pay_buys_nothing() {
    // Seven pistols are worth 7 * 100 * 0.4725 = 330.75, more than the 300 held.
    let mut game = game(1, 0);
    game.give_item(&item("pistol"), count(7)).expect("given");
    let error = game
        .sell(&vendor("shopkeeper"), &item("pistol"), count(7))
        .expect_err("too dear");
    assert_eq!(
        error,
        TradeError::VendorShort {
            have: 300,
            need: 331
        }
    );
    assert_eq!(
        (
            game.item_count(&item("pistol")),
            game.caps(),
            shop_caps(&game)
        ),
        (7, 0, 300)
    );
}

#[test]
fn selling_what_you_do_not_carry_is_refused() {
    let mut game = game(1, 0);
    let error = game
        .sell(&vendor("shopkeeper"), &item("ammo"), count(1))
        .expect_err("empty pack");
    assert_eq!(
        error,
        TradeError::NotCarried {
            item: item("ammo"),
            have: 0,
            need: 1
        }
    );
}

#[test]
fn the_equipped_copy_cannot_be_sold_but_a_spare_can() {
    let mut game = game(1, 0);
    game.give_item(&item("pistol"), count(1)).expect("given");
    game.equip(&item("pistol")).expect("a weapon");
    let error = game
        .sell(&vendor("shopkeeper"), &item("pistol"), count(1))
        .expect_err("wielded");
    assert_eq!(error, TradeError::Equipped(item("pistol")));
    game.give_item(&item("pistol"), count(1)).expect("given");
    game.sell(&vendor("shopkeeper"), &item("pistol"), count(1))
        .expect("a spare");
    assert!(game.is_equipped(&item("pistol")), "the wielded one stays");
}

#[test]
fn an_unpriced_item_has_no_price() {
    let game = game(1, 0);
    assert_eq!(
        game.buy_price(&item("ghost"), count(1)),
        Err(TradeError::NoPrice(item("ghost")))
    );
    // A single round is worth 4.7 caps, but one worth a cap is not worth selling.
    assert!(game.sell_price(&item("ammo"), count(1)).is_ok());
}

#[test]
fn an_unknown_vendor_is_refused() {
    let mut game = game(1, 100);
    let error = game
        .buy(&vendor("nobody"), &item("ammo"), count(1))
        .expect_err("unknown");
    assert_eq!(error, TradeError::UnknownVendor(vendor("nobody")));
}

#[test]
fn a_dead_or_hostile_vendor_does_not_trade() {
    for effect in [
        Effect::KillNpc { npc: npc("keeper") },
        Effect::SetHostile { npc: npc("keeper") },
    ] {
        let mut game = game(1, 500);
        apply(&effect, &mut game).expect("the keeper is in the world");
        let town = place("town");
        assert_eq!(game.vendors_at(&town).count(), 0, "{effect:?}");
        let error = game
            .buy(&vendor("shopkeeper"), &item("ammo"), count(1))
            .expect_err("closed");
        assert_eq!(error, TradeError::Unavailable(npc("keeper")), "{effect:?}");
    }
}

#[test]
fn a_player_who_is_down_cannot_trade() {
    let mut game = game(1, 500);
    let max = game.character().max_hp();
    game = game.with_wounds(max);
    assert!(
        game.buy(&vendor("shopkeeper"), &item("ammo"), count(1))
            .is_err()
    );
}

#[test]
fn stock_the_game_cannot_price_is_rejected_when_the_vendors_are_given() {
    let unpriced = Arc::new(
        ItemBook::new([
            (item("ammo"), ItemClass::Ammo, 0.1),
            (item("pistol"), ItemClass::Weapon, 3.0),
        ])
        .expect("valid"),
    );
    let vendors = Arc::new(VendorBook::new([&shopkeeper()]).expect("valid"));
    let error = game_with(character(1))
        .with_items(unpriced)
        .with_vendors(vendors)
        .expect_err("no values");
    assert!(matches!(error, TradeError::NoValue { .. }));
    // The priced book from the support module accepts the same vendors.
    assert!(
        game_with(character(1))
            .with_items(items())
            .shop(&vendor("shopkeeper"))
            .is_some()
    );
}

#[test]
fn the_vendor_book_refuses_repeats() {
    let twice = record(serde_json::json!({
        "id": "shopkeeper", "name": "n", "npc": "keeper", "place": "town", "caps": 1,
    }));
    assert_eq!(
        VendorBook::new([&shopkeeper(), &twice]).expect_err("repeated id"),
        TradeError::DuplicateVendor(vendor("shopkeeper"))
    );
    let doubled = record(serde_json::json!({
        "id": "other", "name": "n", "npc": "keeper", "place": "town", "caps": 1,
        "stock": [{"item": "ammo", "count": 1}, {"item": "ammo", "count": 2}],
    }));
    assert!(matches!(
        VendorBook::new([&doubled]).expect_err("repeated item"),
        TradeError::DuplicateStock { .. }
    ));
}
