//! A vendor who deals at base value both ways while the player's standing is Mixed:
//! the wiki's "1:1 prices". It wins over a cut, applies to selling too, and a
//! player in any other standing sees the ordinary Barter prices.

use std::sync::Arc;

use engine::game::Game;
use engine::quests::QuestBook;
use engine::reputation::ReputationBook;
use engine::shared::{Effect, Faction, FactionId, InventorySink, InventoryView, Quest, apply};
use engine::trade::{VendorBook, flat_buy_total, flat_sell_total};
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

/// A shopkeeper who deals flat for a Mixed town, and (to show which wins) also
/// takes 25% off for a Mixed one.
fn flat_shopkeeper(with_cut: bool) -> engine::shared::Vendor {
    let mut value = json!({
        "id": "shopkeeper", "name": "The shop", "npc": "keeper", "place": "town", "caps": 5000,
        "stock": [{"item": "ammo", "count": 20}, {"item": "pistol", "count": 1}],
        "flat_prices": [{"type": "reputation", "faction": "town", "level": "mixed", "op": "is"}],
    });
    if with_cut {
        value["discount"] = json!({"percent": 25, "when": [
            {"type": "reputation", "faction": "town", "level": "mixed", "op": "is"},
        ]});
    }
    record(value)
}

fn game(barter_charisma: u32, with_cut: bool, fame: f64, infamy: f64) -> Game {
    let quests = QuestBook::new(std::iter::empty::<&Quest>()).expect("no quests is a valid book");
    let keeper = npc("keeper");
    let reputation = ReputationBook::new([&town()]).expect("the town model is valid");
    let mut game = Game::with_world(Arc::new(quests), Arc::new(WorldBook::new([&keeper])))
        .with_items(items())
        .with_character(character(barter_charisma))
        .with_reputation(Arc::new(reputation))
        .with_vendors(Arc::new(
            VendorBook::new([&flat_shopkeeper(with_cut)]).expect("the literal vendor is valid"),
        ))
        .expect("every stocked item has a value");
    game.add_caps(1_000.0).expect("caps can be added");
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
fn a_mixed_standing_buys_and_sells_at_base_value() {
    let mut game = game(1, false, 4.0, 4.0);
    let shop = vendor("shopkeeper");
    assert!(game.flat_prices_at(&shop));
    let value = game
        .inventory()
        .value(&item("pistol"))
        .expect("it has a value");
    let expected = flat_buy_total(value, count(1));
    assert_eq!(
        game.buy_price_from(&shop, &item("pistol"), count(1)),
        Ok(expected)
    );
    let before = game.caps();
    assert_eq!(game.buy(&shop, &item("pistol"), count(1)), Ok(expected));
    assert_eq!(game.caps(), before - expected);
    // Selling is one for one as well, and conserves the caps the other way.
    let paid = game.sell(&shop, &item("pistol"), count(1)).expect("sold");
    assert_eq!(paid, flat_sell_total(value, count(1)));
    assert_eq!(game.caps(), before - expected + paid);
}

#[test]
fn any_other_standing_pays_the_barter_prices() {
    for (fame, infamy) in [(0.0, 0.0), (4.0, 0.0), (0.0, 4.0)] {
        let game = game(1, false, fame, infamy);
        let shop = vendor("shopkeeper");
        assert!(!game.flat_prices_at(&shop), "{fame}/{infamy}");
        assert_eq!(
            game.buy_price_from(&shop, &item("pistol"), count(1)),
            game.buy_price(&item("pistol"), count(1))
        );
        assert_eq!(
            game.sell_price_to(&shop, &item("pistol"), count(1)),
            game.sell_price(&item("pistol"), count(1))
        );
    }
}

#[test]
fn a_flat_price_wins_over_a_cut() {
    let game = game(1, true, 4.0, 4.0);
    let shop = vendor("shopkeeper");
    assert!(game.discount_at(&shop).is_some(), "the cut is earned too");
    let value = game
        .inventory()
        .value(&item("pistol"))
        .expect("it has a value");
    assert_eq!(
        game.buy_price_from(&shop, &item("pistol"), count(1)),
        Ok(flat_buy_total(value, count(1)))
    );
}

#[test]
fn an_unknown_vendor_has_no_price() {
    let game = game(1, false, 4.0, 4.0);
    assert!(!game.flat_prices_at(&vendor("nobody")));
    assert!(
        game.sell_price_to(&vendor("nobody"), &item("pistol"), count(1))
            .is_err()
    );
}

proptest! {
    /// A flat price does not depend on Barter, and the player never gains by buying
    /// a thing and selling it straight back to the same flat vendor.
    #[test]
    fn a_flat_vendor_ignores_barter_and_never_pays_more_than_it_charged(
        charisma in 1u32..=10, number in 1u32..20,
    ) {
        let mut game = game(charisma, false, 4.0, 4.0);
        let shop = vendor("shopkeeper");
        let low = game.buy_price_from(&shop, &item("ammo"), count(number));
        let value = game.inventory().value(&item("ammo")).expect("it has a value");
        prop_assert_eq!(&low, &Ok(flat_buy_total(value, count(number))));
        let before = game.caps();
        let paid = game.buy(&shop, &item("ammo"), count(number)).expect("bought");
        prop_assert_eq!(Ok(paid), low);
        let got = game.sell(&shop, &item("ammo"), count(number)).expect("sold");
        prop_assert!(got <= paid, "bought for {paid}, sold for {got}");
        prop_assert!(game.caps() <= before);
    }
}
