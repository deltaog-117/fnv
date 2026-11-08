//! Benchmark stub for addiction: looking up an item's risk, recording a dose and
//! reading withdrawal, the steps behind every chem.
//!
//! Prints the median time per round. Run with `cargo bench -p engine --bench
//! addiction`. Record the number as the baseline; a regression above 5% should fail
//! CI once a comparing harness (for example criterion) replaces this stub.

use std::time::{Duration, Instant};

use engine::addiction::{AddictionBook, Addictions};
use engine::shared::ItemId;

const RUNS: usize = 15;
const STEPS: u64 = 100_000;

fn median_run() -> Duration {
    let jet = ItemId::try_new("jet").expect("literal is a valid id");
    let book = AddictionBook::new([(&jet, "jet", 20.0)]).expect("the literal builds");
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let mut held = Addictions::default();
            let start = Instant::now();
            for minute in 0..STEPS {
                if let Some(risk) = book.risk(&jet) {
                    held.dose(
                        risk.kind(),
                        minute,
                        risk.strikes(u8::try_from(minute % 100 + 1).unwrap_or(1)),
                    );
                }
                std::hint::black_box(held.withdrawal());
                held.expire(minute);
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    println!(
        "addiction: {:?} per risk lookup, dose, withdrawal and expiry ({STEPS} per run)",
        median_run() / u32::try_from(STEPS).unwrap_or(u32::MAX)
    );
}
