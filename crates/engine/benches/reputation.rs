//! Benchmark stub for reputation: the gain-and-read path behind every reputation
//! effect and every reputation condition.
//!
//! Adds fame and infamy to a faction many times, then asks for its tier, standing
//! and title, and prints the median time per round. Run with
//! `cargo bench -p engine --bench reputation`. Record the number as the baseline; a
//! regression above 5% should fail CI once a comparing harness (for example
//! criterion) replaces this stub.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use engine::reputation::{ReputationBook, Standing};
use engine::shared::{FactionId, GameData, Renown, Scope, StandingSink, StandingView};

const RUNS: usize = 15;
const STEPS: u32 = 50_000;

fn median_run(book: &Arc<ReputationBook>, faction: &FactionId) -> Duration {
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let mut standing = Standing::new(Arc::clone(book));
            let start = Instant::now();
            for step in 0..STEPS {
                let points = f64::from(step % 3);
                standing
                    .add_reputation(faction, points, points / 2.0)
                    .expect("the faction is modeled");
                // Keep the results alive so the calls cannot be optimised away.
                std::hint::black_box(standing.reputation(faction));
                std::hint::black_box(standing.renown_tier(faction, Renown::Infamy));
                std::hint::black_box(standing.title(faction));
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    let data = GameData::load(&dir, &Scope::Everywhere).expect("the engine data loads");
    let book = Arc::new(ReputationBook::from_data(&data).expect("the reputation book builds"));
    let faction = FactionId::try_new("goodsprings").expect("literal is a valid id");
    println!(
        "reputation: {:?} per gain-and-read round ({STEPS} per run)",
        median_run(&book, &faction) / STEPS
    );
}
