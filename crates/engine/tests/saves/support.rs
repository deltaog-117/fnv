//! A game built from the real data, and a way to give it a history.

use std::num::NonZeroU32;
use std::path::Path;
use std::sync::{Arc, LazyLock};

use engine::character::PerkBook;
use engine::clock::Span;
use engine::combat::{Dice, Power, Side, Status};
use engine::game::{Arsenal, Battle, Game};
use engine::inventory::ItemBook;
use engine::quests::QuestBook;
use engine::reputation::ReputationBook;
use engine::rest::BedBook;
use engine::shared::{
    Effect, EncounterId, GameData, ItemId, LocationId, Scope, VendorId, apply_all,
};
use engine::trade::VendorBook;
use engine::travel::TravelBook;
use engine::world::WorldBook;

pub fn place() -> LocationId {
    LocationId::try_new("goodsprings").expect("literal is a valid id")
}

pub fn item(text: &str) -> ItemId {
    ItemId::try_new(text).expect("literal is a valid id")
}

pub fn vendor(text: &str) -> VendorId {
    VendorId::try_new(text).expect("literal is a valid id")
}

pub fn count(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).expect("a positive count")
}

pub fn effect(json: serde_json::Value) -> Effect {
    serde_json::from_value(json).expect("the literal is a valid effect")
}

pub fn stage(quest: &str, stage: u32) -> Effect {
    effect(serde_json::json!({"type": "set_stage", "quest": quest, "stage": stage}))
}

/// The game just built from the data: what the front end starts from.
static TEMPLATE: LazyLock<Game> = LazyLock::new(|| {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    let data = GameData::load(&dir, &Scope::Around(place())).expect("the engine data loads");
    let quests = Arc::new(QuestBook::new(data.quests.iter()).expect("the quests build"));
    let world = Arc::new(WorldBook::new(data.npcs.iter().map(|npc| &npc.id)));
    Game::with_world(quests, world)
        .with_items(Arc::new(
            ItemBook::from_data(&data).expect("the item book builds"),
        ))
        .with_arsenal(Arc::new(
            Arsenal::from_data(&data).expect("the arsenal builds"),
        ))
        .with_perks(Arc::new(PerkBook::base_game(&data)))
        .with_beds(Arc::new(BedBook::from_data(&data).expect("the beds build")))
        .with_reputation(Arc::new(
            ReputationBook::from_data(&data).expect("the reputation book builds"),
        ))
        .with_vendors(Arc::new(
            VendorBook::from_data(&data).expect("the vendors build"),
        ))
        .expect("the vendors stock known items")
        .with_map(
            Arc::new(TravelBook::from_data(&data).expect("the map builds")),
            &place(),
        )
        .expect("goodsprings is on the map")
});

pub fn template() -> Game {
    TEMPLATE.clone()
}

/// Every roll the same: 1 always hits and always crits, 100 always misses.
pub struct Always(pub u8);

impl Dice for Always {
    fn percent(&mut self) -> u8 {
        self.0
    }
}

/// The player shoots whoever stands until the fight is over.
pub fn fight_to_the_end(battle: &mut Battle) {
    for _ in 0..2_000 {
        if battle.combat().status() != Status::Ongoing {
            return;
        }
        let target = battle.combat().ids().find(|&id| {
            battle.combat().side(id) == Some(Side::Enemy) && battle.combat().is_standing(id)
        });
        let shot = target.map(|id| battle.attack(id, Power::Normal, &mut Always(1)));
        if !matches!(shot, Some(Ok(_))) && battle.reload().is_err() {
            battle.end_turn(&mut Always(1)).expect("the fight goes on");
        }
    }
    panic!("the fight did not end");
}

/// A game with a real history: a won fight, a worn pistol, wounds, a level-up with
/// spent points and a perk, a trade, a boost running, a night's sleep and a walk.
pub fn played() -> Game {
    let mut game = template();
    apply_all(
        &[
            effect(serde_json::json!({"type": "give_item", "item": "9mm_pistol"})),
            effect(serde_json::json!({"type": "give_item", "item": "9mm_round", "count": 60})),
            effect(serde_json::json!({"type": "give_item", "item": "leather_armor"})),
            effect(serde_json::json!({"type": "give_item", "item": "stimpak", "count": 2})),
            effect(serde_json::json!({"type": "give_item", "item": "buffout"})),
            effect(serde_json::json!({"type": "caps", "amount": 300})),
            effect(serde_json::json!({"type": "xp", "amount": 1500})),
            effect(serde_json::json!({"type": "karma", "amount": 40})),
            effect(serde_json::json!({"type": "set_flag", "flag": "trudy_joined_militia"})),
            stage("ghost_town_gunfight", 5),
            stage("ghost_town_gunfight", 10),
            stage("ghost_town_gunfight", 60),
            stage("ghost_town_gunfight", 65),
        ],
        &mut game,
    )
    .expect("the setup effects apply");
    game.equip(&item("9mm_pistol"))
        .expect("the pistol is carried");
    game.equip(&item("leather_armor"))
        .expect("the armor is carried");

    let encounter = EncounterId::try_new("ghost_town_gunfight").expect("literal is a valid id");
    let mut battle = game.start_battle(&encounter).expect("the fight starts");
    fight_to_the_end(&mut battle);
    game.finish_battle(&battle).expect("the fight settles");

    while let Some(pick) = game
        .perks()
        .open_to(game.character())
        .first()
        .map(|p| p.id.clone())
    {
        if game.choose_perk(&pick).is_err() {
            break;
        }
    }
    for skill in [engine::shared::Skill::Guns, engine::shared::Skill::Barter] {
        while game.spend_skill_point(skill).is_ok() && game.character().skill_points() > 3 {}
    }
    game.buy(&vendor("chet"), &item("9mm_round"), count(5))
        .expect("Chet sells rounds");
    game.wait(Span::hours(count(1))).expect("an hour passes");
    let bed = game
        .beds_at(&LocationId::try_new("doc_mitchell_s_house").expect("literal is a valid id"))
        .next()
        .map(|(id, _)| id.clone())
        .expect("Doc has a bed");
    game.sleep(&bed, Span::hours(count(2)))
        .expect("the player sleeps");
    if let Some(route) = game.routes().first().cloned() {
        game.travel(&route.to).expect("the walk is made");
    }
    // Last, so the boost is still running when the game is saved.
    game.use_item(&item("buffout")).expect("buffout is used");
    game
}
