//! Benchmark stub for travel: one trip out and back, the step behind every walk.
//!
//! Builds a small map once, then walks a loop of trips many times and prints the
//! median time per trip. Run with `cargo bench -p engine --bench travel`. Record the
//! number as the baseline; a regression above 5% should fail CI once a comparing
//! harness (for example criterion) replaces this stub.

use std::sync::Arc;
use std::time::{Duration, Instant};

use engine::game::Game;
use engine::quests::QuestBook;
use engine::shared::{Location, LocationId, Quest};
use engine::travel::TravelBook;

const RUNS: usize = 15;
const STEPS: u32 = 20_000;

fn place(text: &str) -> LocationId {
    LocationId::try_new(text.to_owned()).expect("literal is a valid id")
}

fn record(id: &str, to: &str) -> Location {
    serde_json::from_value(serde_json::json!({
        "id": id, "name": id, "kind": "settlement", "dlc": "base",
        "neighbors": [{"id": to, "distance": 42.0}],
    }))
    .expect("the literal is a valid location record")
}

fn median_run() -> Duration {
    let records = [record("a", "b"), record("b", "a")];
    let book = Arc::new(TravelBook::new(&records).expect("the map is valid"));
    let quests = Arc::new(QuestBook::new(std::iter::empty::<&Quest>()).expect("an empty book"));
    let (a, b) = (place("a"), place("b"));
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let mut game = Game::new(Arc::clone(&quests))
                .with_map(Arc::clone(&book), &a)
                .expect("a is on the map");
            let start = Instant::now();
            for step in 0..STEPS {
                let to = if step % 2 == 0 { &b } else { &a };
                // Keep the result alive so the call cannot be optimised away.
                std::hint::black_box(game.travel(to).ok());
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    println!(
        "travel: {:?} per trip ({STEPS} per run)",
        median_run() / STEPS
    );
}
