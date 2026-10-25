//! Benchmark stub for the menu the TUI rebuilds on every frame: the choices of a
//! real, authored conversation, each condition checked against the game state.
//!
//! Builds Doc Mitchell's menu many times and prints the median time per menu. Run
//! with `cargo bench -p engine --bench dialogue`. Record the number as the
//! baseline; a regression above 5% should fail CI once a comparing harness (for
//! example criterion) replaces this stub.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use engine::dialogue::{Conversation, DialogueBook};
use engine::game::Game;
use engine::quests::QuestBook;
use engine::shared::{DialogueId, GameData, Scope};

const RUNS: usize = 15;
const MENUS: u32 = 100_000;

fn median_run(talk: &Conversation, game: &Game) -> Duration {
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            for _ in 0..MENUS {
                // Keep the menu alive so the call cannot be optimised away.
                std::hint::black_box(talk.choices(game));
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
    let quests = Arc::new(QuestBook::new(data.quests.iter()).expect("quests build"));
    let book = Arc::new(DialogueBook::new(data.dialogue.iter()).expect("dialogue builds"));
    let id = DialogueId::try_new("docmitchell".to_owned()).expect("valid id");
    let talk = Conversation::open(book, &id).expect("Doc Mitchell is authored");
    let game = Game::new(quests);
    println!(
        "dialogue menu: {:?} per menu ({MENUS} per run)",
        median_run(&talk, &game) / MENUS
    );
}
