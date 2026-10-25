//! Benchmark stub for the quest log, the hot path of every quest condition and
//! stage change.
//!
//! Replays a quest from its first stage to its finish many times and prints the
//! median time per full playthrough. Run with `cargo bench -p engine --bench
//! quests`. Record the number as the baseline; a regression above 5% should fail
//! CI once a comparing harness (for example criterion) replaces this stub.

use std::sync::Arc;
use std::time::{Duration, Instant};

use engine::quests::{QuestBook, QuestLog};
use engine::shared::{Quest, QuestId, QuestSink, QuestStage};
use serde_json::json;

const RUNS: usize = 15;
const PLAYTHROUGHS: u32 = 100_000;
const STAGES: [u32; 9] = [5, 10, 20, 30, 40, 50, 60, 65, 70];

fn gunfight() -> Quest {
    let stages: Vec<_> = STAGES
        .iter()
        .map(|n| json!({"stage": n, "description": "step", "finish": *n == 70}))
        .collect();
    serde_json::from_value(json!({
        "id": "ghost_town_gunfight",
        "name": "Ghost Town Gunfight",
        "kind": "side",
        "dlc": "base",
        "stages": stages,
    }))
    .expect("the literal is a valid quest record")
}

fn median_run(book: &Arc<QuestBook>, quest: &QuestId) -> Duration {
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            for _ in 0..PLAYTHROUGHS {
                let mut log = QuestLog::new(Arc::clone(book));
                for stage in STAGES {
                    // Keep the result alive so the move cannot be optimised away.
                    std::hint::black_box(log.set_stage(quest, QuestStage::new(stage)).is_ok());
                }
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    let book = Arc::new(QuestBook::new([&gunfight()]).expect("stages are in order"));
    let quest = QuestId::try_new("ghost_town_gunfight".to_owned()).expect("valid id");
    println!(
        "quest playthrough: {:?} per playthrough of {} stages ({PLAYTHROUGHS} per run)",
        median_run(&book, &quest) / PLAYTHROUGHS,
        STAGES.len()
    );
}
