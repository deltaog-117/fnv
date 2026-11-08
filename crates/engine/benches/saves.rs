//! Benchmark stub for save games: a whole save, written out and loaded back.
//!
//! Builds the Goodsprings game from the real data once, gives it some history, then
//! times `save`, `to_json`, `from_json` and `restore` many times and prints the
//! median per round trip. Run with `cargo bench -p engine --bench saves`. Record the
//! number as the baseline; a regression above 5% should fail CI once a comparing
//! harness (for example criterion) replaces this stub.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use engine::addiction::AddictionBook;
use engine::character::PerkBook;
use engine::game::{Arsenal, Game};
use engine::inventory::ItemBook;
use engine::quests::QuestBook;
use engine::reputation::ReputationBook;
use engine::rest::BedBook;
use engine::saves::Save;
use engine::shared::{Effect, GameData, LocationId, QuestId, QuestStage, Scope, apply_all};
use engine::trade::VendorBook;
use engine::travel::TravelBook;
use engine::weapon_mods::WeaponModBook;
use engine::world::WorldBook;

const RUNS: usize = 15;
const TRIPS: u32 = 200;

fn template() -> Game {
    let place = LocationId::try_new("goodsprings").expect("a valid id");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    let data = GameData::load(&dir, &Scope::Around(place.clone())).expect("the data loads");
    Game::with_world(
        Arc::new(QuestBook::new(data.quests.iter()).expect("quests build")),
        Arc::new(WorldBook::new(data.npcs.iter().map(|npc| &npc.id))),
    )
    .with_items(Arc::new(ItemBook::from_data(&data).expect("items build")))
    .with_addictions(Arc::new(
        AddictionBook::from_data(&data).expect("addictions build"),
    ))
    .with_weapon_mods(Arc::new(
        WeaponModBook::from_data(&data).expect("weapon mods build"),
    ))
    .with_arsenal(Arc::new(Arsenal::from_data(&data).expect("arsenal builds")))
    .with_perks(Arc::new(PerkBook::base_game(&data)))
    .with_beds(Arc::new(BedBook::from_data(&data).expect("beds build")))
    .with_reputation(Arc::new(
        ReputationBook::from_data(&data).expect("reputation builds"),
    ))
    .with_vendors(Arc::new(
        VendorBook::from_data(&data).expect("vendors build"),
    ))
    .expect("vendors stock known items")
    .with_map(
        Arc::new(TravelBook::from_data(&data).expect("map builds")),
        &place,
    )
    .expect("goodsprings is on the map")
}

fn median_trip(template: &Game, played: &Game) -> Duration {
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            for _ in 0..TRIPS {
                let text = played.save().to_json().expect("a save writes");
                let save = Save::from_json(&text).expect("a save reads");
                // Keep the game alive so the load cannot be optimised away.
                std::hint::black_box(template.restore(&save).expect("a save loads"));
            }
            start.elapsed()
        })
        .collect();
    times.sort();
    times[RUNS / 2]
}

fn main() {
    let template = template();
    let mut played = template.clone();
    let quest = QuestId::try_new("ghost_town_gunfight").expect("a valid id");
    let steps: Vec<Effect> = [5, 10, 60, 65]
        .into_iter()
        .map(|stage| Effect::SetStage {
            quest: quest.clone(),
            stage: QuestStage::new(stage),
        })
        .collect();
    apply_all(&steps, &mut played).expect("the quest advances");
    println!(
        "saves: {:?} per save, write, read and load ({TRIPS} per run)",
        median_trip(&template, &played) / TRIPS
    );
}
