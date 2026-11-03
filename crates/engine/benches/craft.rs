//! Benchmark stub for crafting: checking and making one recipe on a game.
//!
//! Gives the ingredients of healing powder, makes the powder at the campfire and
//! prints the median time per give-and-craft round. Run with
//! `cargo bench -p engine --bench craft`. Record the number as the baseline; a
//! regression above 5% should fail CI once a comparing harness (for example
//! criterion) replaces this stub.

use std::num::NonZeroU32;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use engine::craft::{RecipeBook, StationBook};
use engine::game::Game;
use engine::inventory::ItemBook;
use engine::quests::QuestBook;
use engine::shared::{GameData, InventorySink, ItemId, RecipeId, Scope, StationId};
use engine::world::WorldBook;

const RUNS: usize = 15;
const ROUNDS: u32 = 10_000;

fn median_run(game: &Game, station: &StationId, recipe: &RecipeId) -> Duration {
    let flower = ItemId::try_new("broc_flower").expect("valid id");
    let root = ItemId::try_new("xander_root").expect("valid id");
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let mut game = game.clone();
            let start = Instant::now();
            for _ in 0..ROUNDS {
                game.give_item(&flower, NonZeroU32::MIN).expect("a flower");
                game.give_item(&root, NonZeroU32::MIN).expect("a root");
                // Keep the result alive so the call cannot be optimised away.
                std::hint::black_box(game.craft(station, recipe).is_ok());
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
    let quests = Arc::new(QuestBook::new(data.quests.iter()).expect("book builds"));
    let world = Arc::new(WorldBook::new(data.npcs.iter().map(|npc| &npc.id)));
    let items = Arc::new(ItemBook::from_data(&data).expect("the item book builds"));
    let recipes = RecipeBook::from_data(&data, |item| items.contains(item)).expect("recipes build");
    let game = Game::with_world(quests, world)
        .with_items(items)
        .with_recipes(Arc::new(recipes))
        .with_stations(Arc::new(
            StationBook::from_data(&data).expect("stations build"),
        ));
    let station = StationId::try_new("goodsprings_campfire").expect("valid id");
    let recipe = RecipeId::try_new("healing_powder_craft").expect("valid id");
    println!(
        "craft: {:?} per give-and-craft round ({ROUNDS} per run)",
        median_run(&game, &station, &recipe) / ROUNDS
    );
}
