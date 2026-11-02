//! Benchmark stub for timed effects: starting a dose, summing what is in force and
//! ending what is over, the three steps behind every chem and every minute.
//!
//! Prints the median time per round. Run with `cargo bench -p engine --bench
//! effects`. Record the number as the baseline; a regression above 5% should fail CI
//! once a comparing harness (for example criterion) replaces this stub.

use std::time::{Duration, Instant};

use engine::effects::ActiveEffects;
use engine::shared::{Boost, ItemId};

const RUNS: usize = 15;
const STEPS: u64 = 50_000;

fn boost(stat: &str, amount: i16) -> Boost {
    serde_json::from_value(serde_json::json!({"stat": stat, "amount": amount, "minutes": 4}))
        .expect("the literal is a valid boost")
}

fn median_run() -> Duration {
    let buffout = ItemId::try_new("buffout").expect("literal is a valid id");
    let dose = [
        boost("max_hp", 60),
        boost("endurance", 3),
        boost("strength", 2),
    ];
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let mut effects = ActiveEffects::default();
            let start = Instant::now();
            for minute in 0..STEPS {
                effects.start(&buffout, &dose, minute);
                effects.expire(minute);
                // Keep the result alive so the calls cannot be optimised away.
                std::hint::black_box(effects.tally());
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    println!(
        "effects: {:?} per dose, expiry and tally ({STEPS} per run)",
        median_run() / u32::try_from(STEPS).unwrap_or(u32::MAX)
    );
}
