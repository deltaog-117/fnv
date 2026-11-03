//! Vendors refill as days pass: the clock alone decides when.

use std::num::NonZeroU32;
use std::sync::Arc;

use engine::clock::Span;
use engine::game::Game;
use engine::quests::QuestBook;
use engine::shared::{InventorySink, InventoryView, Quest};
use engine::trade::{DEFAULT_RESTOCK_DAYS, Market, VendorBook};
use engine::world::WorldBook;
use proptest::prelude::*;
use serde_json::json;

use crate::support::{count, game, item, items, npc, record, shopkeeper, vendor};

const DAY: u64 = 24 * 60;

fn hours(n: u32) -> Span {
    Span::hours(NonZeroU32::new(n).expect("the literal is not zero"))
}

fn ammo_in_shop(game: &Game) -> u32 {
    game.shop(&vendor("shopkeeper"))
        .expect("the shop exists")
        .count(&item("ammo"))
}

fn caps_in_shop(game: &Game) -> u32 {
    game.shop(&vendor("shopkeeper"))
        .expect("the shop exists")
        .caps()
}

/// A game with the shopkeeper's ammo bought out.
fn bought_out() -> Game {
    let mut game = game(1, 5_000);
    game.buy(&vendor("shopkeeper"), &item("ammo"), count(20))
        .expect("the stock is for sale");
    assert_eq!(ammo_in_shop(&game), 0);
    game
}

#[test]
fn a_vendor_with_no_interval_restocks_every_three_days() {
    assert_eq!(DEFAULT_RESTOCK_DAYS.get(), 3);
    let book = VendorBook::new([&shopkeeper()]).expect("valid");
    let info = book.get(&vendor("shopkeeper")).expect("present");
    assert_eq!(info.restock_days(), DEFAULT_RESTOCK_DAYS);
}

#[test]
fn a_bought_out_shop_stays_empty_until_the_restock_day() {
    let mut game = bought_out();
    // The game starts at 08:00 on day 1; the first restock is at midnight on day 4.
    game.wait(hours(24 + 24 + 7)).expect("time passes");
    assert_eq!(ammo_in_shop(&game), 0, "day 3 at 15:00 is too early");
    game.wait(hours(9)).expect("time passes");
    assert_eq!(game.clock().day(), 4);
    assert_eq!(ammo_in_shop(&game), 20, "day 4 has begun");
    assert_eq!(caps_in_shop(&game), 300, "the caps are refilled too");
}

#[test]
fn what_the_player_sold_is_gone_with_the_restock() {
    let mut game = bought_out();
    game.sell(&vendor("shopkeeper"), &item("ammo"), count(3))
        .expect("the shop takes ammo back");
    assert_eq!(ammo_in_shop(&game), 3);
    game.wait(hours(72)).expect("time passes");
    assert_eq!(ammo_in_shop(&game), 20, "back to the stock, not 23");
}

#[test]
fn a_restock_does_not_touch_the_player() {
    let mut game = bought_out();
    let (caps, ammo) = (game.caps(), game.item_count(&item("ammo")));
    game.wait(hours(24 * 10)).expect("time passes");
    assert_eq!(game.caps(), caps);
    assert_eq!(game.item_count(&item("ammo")), ammo);
}

#[test]
fn an_interval_in_the_record_is_used() {
    let quests = QuestBook::new(std::iter::empty::<&Quest>()).expect("no quests");
    let keeper = npc("keeper");
    let daily = record(json!({
        "id": "shopkeeper", "name": "The shop", "npc": "keeper", "place": "town", "caps": 300,
        "stock": [{"item": "ammo", "count": 20}], "restock_days": 1,
    }));
    let mut game = Game::with_world(Arc::new(quests), Arc::new(WorldBook::new([&keeper])))
        .with_items(items())
        .with_vendors(Arc::new(VendorBook::new([&daily]).expect("valid")))
        .expect("stocked items have values");
    game.add_caps(1_000.0).expect("caps can be added");
    game.buy(&vendor("shopkeeper"), &item("ammo"), count(20))
        .expect("bought");
    game.wait(hours(15)).expect("time passes");
    assert_eq!(
        ammo_in_shop(&game),
        0,
        "23:00 on day 1 is before the first midnight"
    );
    game.wait(hours(2)).expect("time passes");
    assert_eq!(ammo_in_shop(&game), 20, "01:00 on day 2 is after it");
}

#[test]
fn a_zero_interval_is_not_a_record() {
    let result: Result<engine::shared::Vendor, _> = serde_json::from_value(json!({
        "id": "v", "name": "V", "npc": "n", "place": "p", "caps": 1, "restock_days": 0,
    }));
    assert!(result.is_err());
}

#[test]
fn the_market_names_the_vendors_it_refilled() {
    let book = Arc::new(VendorBook::new([&shopkeeper()]).expect("valid"));
    let mut market = Market::new(book);
    assert!(market.restock(100, 100).is_empty());
    assert!(market.restock(100, 3 * DAY - 1).is_empty());
    assert_eq!(market.restock(3 * DAY - 1, 3 * DAY), [vendor("shopkeeper")]);
}

proptest! {
    /// However the days are cut into waits, the same time gives the same shop.
    #[test]
    fn waiting_in_many_steps_equals_waiting_at_once(
        steps in proptest::collection::vec(1_u32..80, 1..12)
    ) {
        let mut stepped = bought_out();
        let mut at_once = bought_out();
        for step in &steps {
            stepped.wait(hours(*step)).expect("time passes");
        }
        at_once.wait(hours(steps.iter().sum())).expect("time passes");
        prop_assert_eq!(ammo_in_shop(&stepped), ammo_in_shop(&at_once));
        prop_assert_eq!(caps_in_shop(&stepped), caps_in_shop(&at_once));
        prop_assert_eq!(stepped.clock(), at_once.clock());
    }
}
