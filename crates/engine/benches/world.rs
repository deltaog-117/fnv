//! Benchmark stub for the world effects every quest rule and dialogue line
//! applies: setting flags and changing NPC fates.
//!
//! Applies a mixed batch to a fresh `WorldState` many times and prints the median
//! time per batch. Run with `cargo bench -p engine --bench world`. Record the
//! number as the baseline; a regression above 5% should fail CI once a comparing
//! harness (for example criterion) replaces this stub.

use std::sync::Arc;
use std::time::{Duration, Instant};

use engine::shared::{FlagId, NpcId, WorldSink};
use engine::world::{WorldBook, WorldState};

const RUNS: usize = 15;
const BATCHES: u32 = 100_000;

fn median_run(book: &Arc<WorldBook>, flag: &FlagId, npc: &NpcId) -> Duration {
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            for _ in 0..BATCHES {
                let mut world = WorldState::new(Arc::clone(book));
                // Keep the results alive so the calls cannot be optimised away.
                std::hint::black_box((
                    world.set_flag(flag, true).is_ok(),
                    world.set_hostile(npc).is_ok(),
                    world.kill_npc(npc).is_ok(),
                    world.remove_npc(npc).is_ok(),
                ));
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    let npc = NpcId::try_new("doc".to_owned()).expect("valid id");
    let flag = FlagId::try_new("f".to_owned()).expect("valid id");
    let book = Arc::new(WorldBook::new([&npc]));
    println!(
        "world effects: {:?} per batch of 4 effects ({BATCHES} per run)",
        median_run(&book, &flag, &npc) / BATCHES
    );
}
