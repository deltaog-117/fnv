//! Benchmark stub for items in a fight: the check that runs after every command
//! (is any item that works by itself due?) and one item used in the middle of a
//! fight, which clones the game and rebuilds the player's fighter.
//!
//! Prints the median time per call. Run with
//! `cargo bench -p engine --bench fight_items`. Record the numbers as the baseline;
//! a regression above 5% should fail CI once a comparing harness (for example
//! criterion) replaces this stub.

use std::num::NonZeroU32;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use engine::combat::Dice;
use engine::game::{Arsenal, Game};
use engine::inventory::ItemBook;
use engine::quests::QuestBook;
use engine::shared::{
    Effect, EncounterId, GameData, InventorySink, ItemId, LocationId, Scope, apply_all,
};
use engine::world::WorldBook;

const RUNS: usize = 15;
const CHECKS: u32 = 20_000;
const USES: u32 = 200;

struct Miss;

impl Dice for Miss {
    fn percent(&mut self) -> u8 {
        100
    }
}

fn item(text: &str) -> ItemId {
    ItemId::try_new(text).expect("literal is a valid id")
}

fn game() -> Game {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    let place = LocationId::try_new("goodsprings").expect("literal is a valid id");
    let data = GameData::load(&dir, &Scope::Around(place)).expect("the engine data loads");
    let mut game = Game::with_world(
        Arc::new(QuestBook::new(data.quests.iter()).expect("the quests build")),
        Arc::new(WorldBook::new(data.npcs.iter().map(|npc| &npc.id))),
    )
    .with_items(Arc::new(
        ItemBook::from_data(&data).expect("the book builds"),
    ))
    .with_arsenal(Arc::new(
        Arsenal::from_data(&data).expect("the arsenal builds"),
    ));
    let steps: Vec<Effect> = [
        serde_json::json!({"type": "give_item", "item": "9mm_pistol"}),
        serde_json::json!({"type": "give_item", "item": "9mm_round", "count": 60}),
        serde_json::json!({"type": "set_stage", "quest": "ghost_town_gunfight", "stage": 5}),
        serde_json::json!({"type": "set_stage", "quest": "ghost_town_gunfight", "stage": 10}),
        serde_json::json!({"type": "set_stage", "quest": "ghost_town_gunfight", "stage": 60}),
        serde_json::json!({"type": "set_stage", "quest": "ghost_town_gunfight", "stage": 65}),
    ]
    .into_iter()
    .map(|value| serde_json::from_value(value).expect("the literal is a valid effect"))
    .collect();
    apply_all(&steps, &mut game).expect("the setup applies");
    game.equip(&item("9mm_pistol"))
        .expect("the pistol is carried");
    InventorySink::give_item(
        &mut game,
        &item("med_x"),
        NonZeroU32::new(500).expect("not zero"),
    )
    .expect("given");
    game
}

fn median(mut work: impl FnMut() -> Duration) -> Duration {
    let mut times: Vec<Duration> = (0..RUNS).map(|_| work()).collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    let base = game();
    let idle = median(|| {
        let mut game = base.clone();
        let start = Instant::now();
        for _ in 0..CHECKS {
            // Keep the result alive so the call cannot be optimised away.
            std::hint::black_box(game.auto_use());
        }
        start.elapsed()
    });
    println!(
        "auto_use with nothing due: {:?} per check ({CHECKS} per run)",
        idle / CHECKS
    );

    let encounter = EncounterId::try_new("ghost_town_gunfight").expect("literal is a valid id");
    let in_fight = median(|| {
        let mut game = base.clone();
        let mut battle = game.start_battle(&encounter).expect("the fight starts");
        battle.open(&mut Miss);
        let start = Instant::now();
        let mut used = 0;
        for _ in 0..USES {
            if game
                .use_item_in_battle(&mut battle, &item("med_x"), &mut Miss)
                .is_err()
            {
                // The action points ran out: pass and start the next turn.
                battle.end_turn(&mut Miss).ok();
            } else {
                used += 1;
            }
        }
        std::hint::black_box(used);
        start.elapsed()
    });
    println!(
        "use_item_in_battle: {:?} per call ({USES} per run)",
        in_fight / USES
    );
}
