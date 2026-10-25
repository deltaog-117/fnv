//! Benchmark stub for the pack effects every quest reward and dialogue line
//! applies: giving items, paying caps and taking items back.
//!
//! Applies a mixed batch to a fresh `Inventory` many times and prints the median
//! time per batch. Run with `cargo bench -p engine --bench inventory`. Record the
//! number as the baseline; a regression above 5% should fail CI once a comparing
//! harness (for example criterion) replaces this stub.

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use engine::inventory::{Inventory, ItemBook, ItemClass};
use engine::shared::{InventorySink, ItemId};

const RUNS: usize = 15;
const BATCHES: u32 = 100_000;

fn median_run(book: &Arc<ItemBook>, item: &ItemId, one: NonZeroU32) -> Duration {
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            for _ in 0..BATCHES {
                let mut pack = Inventory::new(Arc::clone(book));
                // Keep the results alive so the calls cannot be optimised away.
                std::hint::black_box((
                    pack.give_item(item, one).is_ok(),
                    pack.add_caps(18.0).is_ok(),
                    pack.add_caps(-8.0).is_ok(),
                    pack.take_item(item, one).is_ok(),
                ));
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    let item = ItemId::try_new("stimpak".to_owned()).expect("valid id");
    let book = Arc::new(
        ItemBook::new([(item.clone(), ItemClass::Consumable, 0.5)]).expect("valid entries"),
    );
    let one = NonZeroU32::MIN;
    println!(
        "inventory effects: {:?} per batch of 4 effects ({BATCHES} per run)",
        median_run(&book, &item, one) / BATCHES
    );
}
