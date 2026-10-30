//! Benchmark stub for a trade: pricing a line, then moving caps and items between
//! the player and a vendor.
//!
//! Buys and sells the same line on a fresh game many times and prints the median
//! time per buy-and-sell pair. Run with `cargo bench -p engine --bench trade`.
//! Record the number as the baseline; a regression above 5% should fail CI once a
//! comparing harness (for example criterion) replaces this stub.

use std::num::NonZeroU32;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use engine::game::Game;
use engine::inventory::ItemBook;
use engine::quests::QuestBook;
use engine::shared::{GameData, InventorySink, ItemId, Scope, VendorId};
use engine::trade::VendorBook;
use engine::world::WorldBook;

const RUNS: usize = 15;
const PAIRS: u32 = 10_000;

fn median_run(game: &Game, vendor: &VendorId, item: &ItemId, line: NonZeroU32) -> Duration {
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let mut game = game.clone();
            let start = Instant::now();
            for _ in 0..PAIRS {
                // Keep the results alive so the calls cannot be optimised away.
                std::hint::black_box((
                    game.buy(vendor, item, line).is_ok(),
                    game.sell(vendor, item, line).is_ok(),
                ));
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    let data = GameData::load(&dir, &Scope::Everywhere).expect("the engine data loads");
    let quests = Arc::new(QuestBook::new(data.quests.iter()).expect("book builds"));
    let world = Arc::new(WorldBook::new(data.npcs.iter().map(|npc| &npc.id)));
    let items = Arc::new(ItemBook::from_data(&data).expect("the item book builds"));
    let vendors = Arc::new(VendorBook::from_data(&data).expect("the vendor book builds"));
    let (vendor, info) = vendors.iter().next().expect("there is a vendor");
    let (item, _) = info.stock().next().expect("there is stock");
    let mut game = Game::with_world(quests, world)
        .with_items(items)
        .with_vendors(Arc::clone(&vendors))
        .expect("every stocked item has a value");
    game.add_caps(1_000_000.0).expect("caps can be added");
    println!(
        "trade: {:?} per buy-and-sell pair ({PAIRS} per run)",
        median_run(&game, vendor, item, NonZeroU32::MIN) / PAIRS
    );
}
