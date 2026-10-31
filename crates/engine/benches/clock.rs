//! Benchmark stub for passing time: advancing the clock and reading the date.
//!
//! Advances a clock by an hour many times, formatting it each time, and prints the
//! median time per step. Run with `cargo bench -p engine --bench clock`. Record the
//! number as the baseline; a regression above 5% should fail CI once a comparing
//! harness (for example criterion) replaces this stub.

use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use engine::clock::{GameClock, Span};

const RUNS: usize = 15;
const STEPS: u32 = 100_000;

fn median_run(span: Span) -> Duration {
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let mut clock = GameClock::START;
            let start = Instant::now();
            for _ in 0..STEPS {
                // Keep the results alive so the calls cannot be optimised away.
                std::hint::black_box((clock.advance(span).is_ok(), clock.to_string()));
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    let hour = Span::hours(NonZeroU32::MIN);
    println!(
        "clock: {:?} per advance-and-format ({STEPS} per run)",
        median_run(hour) / STEPS
    );
}
