//! Benchmark stub for repair: the three pure laws behind every repair (the kit's
//! ceiling, the merge of two copies and the vendor's price).
//!
//! Runs them over many weapons and skills and prints the median time per round of
//! all three. Run with `cargo bench -p engine --bench repair`. Record the number as
//! the baseline; a regression above 5% should fail CI once a comparing harness
//! (for example criterion) replaces this stub.

use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use engine::repair::{after_kit, after_merge, vendor_price};
use engine::shared::SkillLevel;

const RUNS: usize = 15;
const STEPS: u32 = 100_000;

fn median_run() -> Duration {
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            for step in 1..=STEPS {
                let max = NonZeroU32::new(step % 300 + 1).expect("one or more");
                let skill = SkillLevel::try_new(step % 101).expect("in range");
                let lost = step % (max.get() + 1);
                // Keep the results alive so the calls cannot be optimised away.
                std::hint::black_box((
                    after_kit(max, lost, skill),
                    after_merge(max, lost, lost / 2, skill),
                    vendor_price(f64::from(step), max, lost, lost / 3),
                ));
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    println!(
        "repair: {:?} per round of kit, merge and price ({STEPS} per run)",
        median_run() / STEPS
    );
}
