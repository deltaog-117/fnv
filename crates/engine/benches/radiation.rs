//! Benchmark stub for radiation: taking rads in past resistance, reading the
//! sickness and taking some out, the steps behind every irradiated meal.
//!
//! Prints the median time per round. Run with `cargo bench -p engine --bench
//! radiation`. Record the number as the baseline; a regression above 5% should fail
//! CI once a comparing harness (for example criterion) replaces this stub.

use std::time::{Duration, Instant};

use engine::radiation::Radiation;

const RUNS: usize = 15;
const STEPS: u32 = 200_000;

fn median_run() -> Duration {
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let mut radiation = Radiation::default();
            let start = Instant::now();
            for step in 0..STEPS {
                radiation.absorb(u16::try_from(step % 40).unwrap_or(0), 12);
                std::hint::black_box(radiation.sickness());
                radiation.remove(u16::try_from(step % 25).unwrap_or(0));
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    println!(
        "radiation: {:?} per absorb, sickness and removal ({STEPS} per run)",
        median_run() / STEPS
    );
}
