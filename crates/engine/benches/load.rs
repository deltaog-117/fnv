//! Benchmark stub for the data loader.
//!
//! Times a full load and a Goodsprings load and prints the median of several runs.
//! Run with `cargo bench -p engine --bench load`. Record the numbers as the
//! baseline; a regression above 5% should fail CI once a comparing harness
//! (for example criterion) replaces this stub.

use std::path::Path;
use std::time::{Duration, Instant};

use engine::shared::{GameData, LocationId, Scope};

const RUNS: usize = 15;

fn median_load(dir: &Path, scope: &Scope) -> Duration {
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            let data = GameData::load(dir, scope).expect("the real data loads");
            // Keep the result alive so the load cannot be optimised away.
            std::hint::black_box(&data);
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    let root = LocationId::try_new("goodsprings").expect("literal is a valid id");
    println!(
        "load everywhere: {:?}",
        median_load(&dir, &Scope::Everywhere)
    );
    println!(
        "load goodsprings: {:?}",
        median_load(&dir, &Scope::Around(root))
    );
}
