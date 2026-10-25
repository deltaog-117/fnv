//! Benchmark stub for `apply_all` over the composed game state, the path every
//! quest rule and dialogue effect takes.
//!
//! Applies a batch of stage changes to a fresh `Game` many times and prints the
//! median time per batch. Run with `cargo bench -p engine --bench game`. Record
//! the number as the baseline; a regression above 5% should fail CI once a
//! comparing harness (for example criterion) replaces this stub.

use std::sync::Arc;
use std::time::{Duration, Instant};

use engine::game::Game;
use engine::quests::QuestBook;
use engine::shared::{Effect, Quest, QuestId, QuestStage, apply_all};
use serde_json::json;

const RUNS: usize = 15;
const BATCHES: u32 = 100_000;
const STAGES: [u32; 5] = [5, 10, 20, 30, 40];

/// With `automated`, the last stage pays out and a rule watches the quest, so
/// every stage change takes the clone-and-react path.
fn quest(automated: bool) -> Quest {
    let stages: Vec<_> = STAGES
        .iter()
        .map(|n| {
            let on_enter = if automated && *n == 40 {
                json!([{"type": "xp", "amount": 10.0}])
            } else {
                json!([])
            };
            json!({"stage": n, "description": "step", "finish": *n == 40, "on_enter": on_enter})
        })
        .collect();
    let rules = if automated {
        json!([{
            "kind": "locks", "target": "Q", "target_type": "quest", "target_id": "q",
            "conditions": [{"type": "quest", "quest": "q", "state": "failed"}],
            "effects": [{"type": "set_flag", "flag": "seen"}],
        }])
    } else {
        json!([])
    };
    serde_json::from_value(json!({
        "id": "q", "name": "q", "kind": "side", "dlc": "base", "stages": stages, "rules": rules,
    }))
    .expect("the literal is a valid quest record")
}

fn median_run(book: &Arc<QuestBook>, effects: &[Effect]) -> Duration {
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            for _ in 0..BATCHES {
                let mut game = Game::new(Arc::clone(book));
                // Keep the result alive so the call cannot be optimised away.
                std::hint::black_box(apply_all(effects, &mut game).is_ok());
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    for (label, automated) in [("plain", false), ("automated", true)] {
        bench(label, automated);
    }
}

fn bench(label: &str, automated: bool) {
    let book = Arc::new(QuestBook::new([&quest(automated)]).expect("stages are in order"));
    let id = QuestId::try_new("q".to_owned()).expect("valid id");
    let effects: Vec<Effect> = STAGES
        .iter()
        .map(|n| Effect::SetStage {
            quest: id.clone(),
            stage: QuestStage::new(*n),
        })
        .collect();
    println!(
        "apply_all ({label}): {:?} per batch of {} effects ({BATCHES} per run)",
        median_run(&book, &effects) / BATCHES,
        effects.len()
    );
}
