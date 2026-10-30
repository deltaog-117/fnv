//! However the player trades, caps and items only change hands: nothing is made or
//! lost, and neither side ever goes below zero.

use engine::game::Game;
use engine::shared::{InventorySink, InventoryView, ItemId};
use proptest::prelude::*;

use crate::support::{count, game, item, vendor};

const NAMES: [&str; 3] = ["ammo", "pistol", "hat"];
const STARTING_CAPS: u32 = 1_000;

#[derive(Debug, Clone)]
enum Trade {
    Buy(usize, u32),
    Sell(usize, u32),
}

fn trade() -> impl Strategy<Value = Trade> {
    prop_oneof![
        (0..NAMES.len(), 1_u32..25).prop_map(|(n, c)| Trade::Buy(n, c)),
        (0..NAMES.len(), 1_u32..25).prop_map(|(n, c)| Trade::Sell(n, c)),
    ]
}

fn holdings(game: &Game, id: &ItemId) -> u32 {
    let shop = game.shop(&vendor("shopkeeper")).expect("the shop exists");
    game.item_count(id) + shop.count(id)
}

proptest! {
    #[test]
    fn trades_only_move_caps_and_items(
        charisma in 1_u32..=10, trades in prop::collection::vec(trade(), 0..40),
    ) {
        let mut game = game(charisma, STARTING_CAPS);
        game.give_item(&item("hat"), count(3)).expect("given");
        let shopkeeper = vendor("shopkeeper");
        let caps_before = game.caps() + game.shop(&shopkeeper).expect("exists").caps();
        let items_before: Vec<u32> = NAMES.iter().map(|n| holdings(&game, &item(n))).collect();
        for step in trades {
            let before = game.clone();
            let result = match &step {
                Trade::Buy(n, c) => game.buy(&shopkeeper, &item(NAMES[*n]), count(*c)),
                Trade::Sell(n, c) => game.sell(&shopkeeper, &item(NAMES[*n]), count(*c)),
            };
            if result.is_err() {
                prop_assert_eq!(game.caps(), before.caps(), "a refusal changed the player's caps");
                prop_assert_eq!(
                    game.shop(&shopkeeper), before.shop(&shopkeeper),
                    "a refusal changed the shop"
                );
            }
        }
        let caps_after = game.caps() + game.shop(&shopkeeper).expect("exists").caps();
        prop_assert_eq!(caps_after, caps_before, "caps were made or lost");
        let items_after: Vec<u32> = NAMES.iter().map(|n| holdings(&game, &item(n))).collect();
        prop_assert_eq!(items_after, items_before, "items were made or lost");
    }
}
