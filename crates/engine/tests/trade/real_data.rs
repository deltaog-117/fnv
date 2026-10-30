//! The Goodsprings vendors from `data/engine/`.

use std::num::NonZeroU32;
use std::path::Path;
use std::sync::Arc;

use engine::game::Game;
use engine::inventory::ItemBook;
use engine::quests::QuestBook;
use engine::shared::{GameData, InventorySink, InventoryView, Scope};
use engine::trade::VendorBook;
use engine::world::WorldBook;

fn game() -> (Game, VendorBook) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    let data = GameData::load(&dir, &Scope::Everywhere).expect("the engine data loads");
    let quests = Arc::new(QuestBook::new(data.quests.iter()).expect("book builds"));
    let world = Arc::new(WorldBook::new(data.npcs.iter().map(|npc| &npc.id)));
    let items = Arc::new(ItemBook::from_data(&data).expect("the item book builds"));
    let vendors = VendorBook::from_data(&data).expect("the vendor book builds");
    let game = Game::with_world(quests, world)
        .with_items(items)
        .with_vendors(Arc::new(
            VendorBook::from_data(&data).expect("the vendor book builds"),
        ))
        .expect("every stocked item has a value");
    (game, vendors)
}

#[test]
fn the_three_goodsprings_vendors_exist() {
    let (_, vendors) = game();
    let ids: Vec<&str> = vendors.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, ["chet", "doc_mitchell", "trudy"]);
}

#[test]
fn every_stocked_line_can_be_bought_and_the_vendor_is_then_empty_of_it() {
    let (game, vendors) = game();
    for (id, info) in vendors.iter() {
        for (item, stock) in info.stock() {
            let mut game = game.clone();
            let price = game
                .buy_price(item, stock)
                .expect("a stocked item has a price");
            game.add_caps(f64::from(price)).expect("caps can be added");
            let paid = game
                .buy(id, item, stock)
                .expect("the whole line can be bought");
            assert_eq!(paid, price, "{id}: {item}");
            assert_eq!(
                game.shop(id).expect("exists").count(item),
                0,
                "{id}: {item}"
            );
            assert_eq!(game.caps(), 0, "{id}: {item}");
        }
    }
}

#[test]
fn a_vendor_can_pay_for_a_cheap_item() {
    let (mut game, vendors) = game();
    let (id, info) = vendors.iter().next().expect("there is a vendor");
    let (item, _) = info.stock().next().expect("there is stock");
    game.give_item(item, NonZeroU32::MIN).expect("given");
    assert!(game.sell(id, item, NonZeroU32::MIN).is_ok());
}
