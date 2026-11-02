//! A vendor's price cut: earned by standing, applied to purchases only, and never
//! able to make a price free or higher than the list price.

use std::sync::Arc;

use engine::game::Game;
use engine::quests::QuestBook;
use engine::reputation::ReputationBook;
use engine::shared::{
    CharacterView, Effect, Faction, FactionId, InventorySink, InventoryView, Quest, Skill, apply,
};
use engine::trade::{TradeError, VendorBook, buy_total_with_discount};
use engine::world::WorldBook;
use proptest::prelude::*;
use serde_json::json;

use crate::support::{character, count, item, items, npc, record, vendor};

fn town() -> Faction {
    serde_json::from_value(json!({
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
    .expect("the literal is a valid faction record")
}

/// A shopkeeper who takes 25% off for a town that finds the player Accepted or
/// better, or Mixed.
fn discounting_shopkeeper() -> engine::shared::Vendor {
    record(json!({
        "id": "shopkeeper", "name": "The shop", "npc": "keeper", "place": "town", "caps": 300,
        "stock": [{"item": "ammo", "count": 20}, {"item": "pistol", "count": 1}],
        "discount": {"percent": 25, "when": [
            {"type": "reputation", "faction": "town", "level": "accepted"},
            {"type": "reputation", "faction": "town", "level": "mixed", "op": "is"},
        ]},
    }))
}

/// A game where the player holds `caps` and has earned `fame` and `infamy` with the
/// town.
fn game(charisma: u32, caps: u32, fame: f64, infamy: f64) -> Game {
    let quests = QuestBook::new(std::iter::empty::<&Quest>()).expect("no quests is a valid book");
    let keeper = npc("keeper");
    let reputation = ReputationBook::new([&town()]).expect("the town model is valid");
    let mut game = Game::with_world(Arc::new(quests), Arc::new(WorldBook::new([&keeper])))
        .with_items(items())
        .with_character(character(charisma))
        .with_reputation(Arc::new(reputation))
        .with_vendors(Arc::new(
            VendorBook::new([&discounting_shopkeeper()]).expect("the literal vendor is valid"),
        ))
        .expect("every stocked item has a value");
    game.add_caps(f64::from(caps)).expect("caps can be added");
    if fame > 0.0 || infamy > 0.0 {
        let gain = Effect::Reputation {
            faction: FactionId::try_new("town").expect("valid id"),
            fame,
            infamy,
        };
        apply(&gain, &mut game).expect("the town tracks reputation");
    }
    game
}

#[test]
fn a_stranger_pays_the_list_price() {
    let game = game(1, 1_000, 0.0, 0.0);
    let shop = vendor("shopkeeper");
    assert_eq!(game.discount_at(&shop), None);
    assert_eq!(
        game.buy_price_from(&shop, &item("pistol"), count(1)),
        game.buy_price(&item("pistol"), count(1))
    );
}

#[test]
fn an_accepted_player_pays_a_quarter_less_and_is_charged_exactly_that() {
    let mut game = game(1, 1_000, 4.0, 0.0);
    let shop = vendor("shopkeeper");
    let list = game.buy_price(&item("pistol"), count(1)).expect("priced");
    let cut = game
        .buy_price_from(&shop, &item("pistol"), count(1))
        .expect("priced");
    assert!(cut < list, "{cut} is not below {list}");
    let before = game.caps();
    let paid = game.buy(&shop, &item("pistol"), count(1)).expect("bought");
    assert_eq!(paid, cut);
    assert_eq!(game.caps(), before - cut);
}

#[test]
fn a_mixed_standing_earns_the_cut_too() {
    let game = game(1, 1_000, 4.0, 4.0);
    assert_eq!(
        game.discount_at(&vendor("shopkeeper")).map(|p| p.get()),
        Some(25)
    );
}

#[test]
fn a_shunned_player_gets_no_cut() {
    let game = game(1, 1_000, 0.0, 4.0);
    assert_eq!(game.discount_at(&vendor("shopkeeper")), None);
}

#[test]
fn selling_is_not_cut() {
    let stranger = game(1, 0, 0.0, 0.0);
    let friend = game(1, 0, 4.0, 0.0);
    assert_eq!(
        stranger.sell_price(&item("pistol"), count(1)),
        friend.sell_price(&item("pistol"), count(1))
    );
}

#[test]
fn an_unknown_vendor_has_no_price() {
    let game = game(1, 0, 4.0, 0.0);
    assert_eq!(
        game.buy_price_from(&vendor("nobody"), &item("ammo"), count(1)),
        Err(TradeError::UnknownVendor(vendor("nobody")))
    );
}

proptest! {
    #[test]
    fn a_cut_price_is_never_free_and_never_above_the_list_price(
        charisma in 1_u32..=10, number in 1_u32..200, fame in 0.0_f64..20.0,
    ) {
        let game = game(charisma, 0, fame, 0.0);
        let shop = vendor("shopkeeper");
        let list = game.buy_price(&item("ammo"), count(number)).expect("priced");
        let price = game.buy_price_from(&shop, &item("ammo"), count(number)).expect("priced");
        prop_assert!(price >= 1, "a purchase became free");
        prop_assert!(price <= list, "{price} is above the list price {list}");
    }

    #[test]
    fn the_cut_function_agrees_with_the_game(charisma in 1_u32..=10, number in 1_u32..50) {
        let game = game(charisma, 0, 4.0, 0.0);
        let shop = vendor("shopkeeper");
        let percent = game.discount_at(&shop).expect("earned");
        let barter = game.character().skill(Skill::Barter);
        prop_assert_eq!(
            game.buy_price_from(&shop, &item("ammo"), count(number)).expect("priced"),
            buy_total_with_discount(10.0, count(number), barter, percent)
        );
    }
}
