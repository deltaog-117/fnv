//! Benchmark stub for sleeping: the recovery calculation behind every sleep.
//!
//! Works out the hit points restored by a sleep of random-ish lengths many times and
//! prints the median time per call. Run with `cargo bench -p engine --bench rest`.
//! Record the number as the baseline; a regression above 5% should fail CI once a
//! comparing harness (for example criterion) replaces this stub.

use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use engine::clock::Span;
use engine::rest::restored;

const RUNS: usize = 15;
const STEPS: u32 = 100_000;

fn median_run() -> Duration {
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            for step in 1..=STEPS {
                let hours = NonZeroU32::new(step % 24 + 1).expect("one or more");
                // Keep the result alive so the call cannot be optimised away.
                std::hint::black_box(restored(200, Span::hours(hours)));
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    println!(
        "rest: {:?} per recovery calculation ({STEPS} per run)",
        median_run() / STEPS
    );
}
