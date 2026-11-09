//! Benchmark stub for the party: joining and leaving, the work done each time a
//! companion comes or goes before the game re-reads their perk.
//!
//! Fills and empties a full party many times and prints the median time per
//! join-and-leave pair. Run with `cargo bench -p engine --bench companions`. Record
//! the number as the baseline; a regression above 5% should fail CI once a
//! comparing harness (for example criterion) replaces this stub.

use std::time::{Duration, Instant};

use engine::companions::{PARTY_LIMIT, Party};
use engine::shared::CompanionId;

const RUNS: usize = 15;
const STEPS: u32 = 100_000;

fn median_run(people: &[CompanionId]) -> Duration {
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            let mut party = Party::default();
            for step in 0..STEPS {
                let who = &people[step as usize % people.len()];
                // Keep the results alive so the calls cannot be optimised away.
                std::hint::black_box(party.join(who).is_ok());
                std::hint::black_box(party.leave(who).is_ok());
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    let people: Vec<CompanionId> = (0..PARTY_LIMIT + 3)
        .map(|n| CompanionId::try_new(format!("companion_{n}")).expect("a valid id"))
        .collect();
    println!(
        "companions: {:?} per join-and-leave pair ({STEPS} per run)",
        median_run(&people) / STEPS
    );
}
