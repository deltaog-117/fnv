//! Repair, crafting and gathering through the composed `Game`, over the real data.
//!
//! A worn weapon and a trained Repair skill are made by loading a hand-edited save,
//! which is the same path a real game takes back from disk.

use std::path::Path;
use std::sync::{Arc, LazyLock};

use engine::character::PerkBook;
use engine::craft::{CraftError, RecipeBook, StationBook};
use engine::game::{Arsenal, Game};
use engine::gather::{FindBook, GatherError};
use engine::inventory::ItemBook;
use engine::quests::QuestBook;
use engine::repair::RepairError;
use engine::reputation::ReputationBook;
use engine::rest::BedBook;
use engine::shared::{
    CharacterView, Effect, FindId, GameData, InventoryView, ItemId, LocationId, QuestStage,
    QuestStatus, QuestView, RecipeId, Scope, Skill, StationId, VendorId, apply_all,
};
use engine::trade::VendorBook;
use engine::travel::TravelBook;
use engine::world::WorldBook;
use proptest::prelude::*;

fn place() -> LocationId {
    LocationId::try_new("goodsprings").expect("literal is a valid id")
}

fn item(text: &str) -> ItemId {
    ItemId::try_new(text).expect("literal is a valid id")
}

fn effect(json: serde_json::Value) -> Effect {
    serde_json::from_value(json).expect("the literal is a valid effect")
}

fn give(game: &mut Game, text: &str, n: u32) {
    apply_all(
        &[effect(
            serde_json::json!({"type": "give_item", "item": text, "count": n}),
        )],
        game,
    )
    .expect("the item is given");
}

fn stage(game: &mut Game, quest: &str, stage: u32) -> Result<(), engine::shared::BatchError> {
    apply_all(
        &[effect(
            serde_json::json!({"type": "set_stage", "quest": quest, "stage": stage}),
        )],
        game,
    )
}

static TEMPLATE: LazyLock<Game> = LazyLock::new(|| {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    let data = GameData::load(&dir, &Scope::Around(place())).expect("the engine data loads");
    let quests = Arc::new(QuestBook::new(data.quests.iter()).expect("the quests build"));
    let world = Arc::new(WorldBook::new(data.npcs.iter().map(|npc| &npc.id)));
    let items = Arc::new(ItemBook::from_data(&data).expect("the item book builds"));
    let recipes = RecipeBook::from_data(&data, |item| items.contains(item)).expect("recipes build");
    Game::with_world(quests, world)
        .with_items(items)
        .with_arsenal(Arc::new(
            Arsenal::from_data(&data).expect("the arsenal builds"),
        ))
        .with_perks(Arc::new(PerkBook::base_game(&data)))
        .with_beds(Arc::new(BedBook::from_data(&data).expect("the beds build")))
        .with_recipes(Arc::new(recipes))
        .with_stations(Arc::new(
            StationBook::from_data(&data).expect("stations build"),
        ))
        .with_finds(Arc::new(
            FindBook::from_data(&data).expect("the finds build"),
        ))
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

/// A game whose Repair skill is at least `repair` (as far as the points a veteran
/// earns will go), whose pistol has lost `lost` points, with `pistols` of them in
/// the pack and `kits` weapon repair kits.
fn worn(repair: u8, lost: u32, pistols: u32, kits: u32) -> Game {
    let mut game = TEMPLATE.clone();
    if repair > 0 {
        give_xp(&mut game, 40_000);
        while game.character().skill(Skill::Repair).get() < repair
            && game.spend_skill_point(Skill::Repair).is_ok()
        {}
    }
    if pistols > 0 {
        give(&mut game, "9mm_pistol", pistols);
    }
    if kits > 0 {
        give(&mut game, "weapon_repair_kit", kits);
    }
    let mut save = game.save();
    save.wear.push(engine::saves::SavedWear {
        item: item("9mm_pistol"),
        lost,
    });
    TEMPLATE.restore(&save).expect("the worn pistol loads")
}

fn give_xp(game: &mut Game, amount: u32) {
    apply_all(
        &[effect(
            serde_json::json!({"type": "xp", "amount": f64::from(amount)}),
        )],
        game,
    )
    .expect("experience is given");
}

fn left(game: &Game) -> u8 {
    game.weapon_condition(&item("9mm_pistol"))
        .expect("the pistol wears")
        .get()
}

#[test]
fn the_test_character_really_has_the_repair_skill_asked_for() {
    let game = worn(70, 0, 1, 0);
    assert!(game.character().skill(Skill::Repair).get() >= 70);
}

#[test]
fn a_kit_lifts_a_worn_weapon_to_the_repair_skill_and_is_used_up() {
    let mut game = worn(80, 140, 1, 1);
    assert!(left(&game) < 10);
    let skill = game.character().skill(Skill::Repair).get();
    let after = game.repair_with_kit(&item("9mm_pistol")).expect("mended");
    assert_eq!(left(&game), after.get());
    assert!(
        after.get() >= skill.min(99),
        "{} vs Repair {skill}",
        after.get()
    );
    assert!(after.get() <= 100);
    assert_eq!(game.inventory().item_count(&item("weapon_repair_kit")), 0);
}

#[test]
fn a_kit_that_cannot_help_is_kept() {
    let mut game = worn(80, 0, 1, 1);
    let before = game.save();
    assert!(matches!(
        game.repair_with_kit(&item("9mm_pistol")),
        Err(RepairError::NothingToGain { .. })
    ));
    assert_eq!(game.save(), before);
    assert_eq!(game.inventory().item_count(&item("weapon_repair_kit")), 1);
}

#[test]
fn a_repair_without_a_kit_or_for_something_that_does_not_wear_changes_nothing() {
    let mut game = worn(80, 140, 1, 0);
    let before = game.save();
    assert!(matches!(
        game.repair_with_kit(&item("9mm_pistol")),
        Err(RepairError::NotCarried { .. })
    ));
    give(&mut game, "weapon_repair_kit", 1);
    give(&mut game, "stimpak", 1);
    assert!(matches!(
        game.repair_with_kit(&item("stimpak")),
        Err(RepairError::DoesNotWear(_))
    ));
    assert_eq!(game.inventory().item_count(&item("weapon_repair_kit")), 1);
    assert_ne!(game.save(), before, "the gifts changed it");
}

#[test]
fn merging_two_copies_leaves_one_better_copy() {
    let mut game = worn(90, 100, 2, 0);
    let before = left(&game);
    let after = game.merge_weapons(&item("9mm_pistol")).expect("merged");
    assert_eq!(game.inventory().item_count(&item("9mm_pistol")), 1);
    assert!(after.get() > before);
    assert_eq!(left(&game), after.get());
}

#[test]
fn merging_needs_two_copies_and_changes_nothing_without() {
    let mut game = worn(90, 100, 1, 0);
    let before = game.save();
    assert!(matches!(
        game.merge_weapons(&item("9mm_pistol")),
        Err(RepairError::NotCarried { need: 2, .. })
    ));
    assert_eq!(game.save(), before);
}

fn chet() -> VendorId {
    VendorId::try_new("chet").expect("literal is a valid id")
}

#[test]
fn chet_repairs_for_caps_and_is_paid_what_he_quoted() {
    let mut game = worn(0, 140, 1, 0);
    give_caps(&mut game, 500);
    let quote = game
        .repair_quote(&chet(), &item("9mm_pistol"))
        .expect("a quote");
    assert!(quote.price > 0);
    let (player, vendor) = (game.caps(), game.shop(&chet()).expect("a shop").caps());
    let done = game
        .repair_at(&chet(), &item("9mm_pistol"))
        .expect("mended");
    assert_eq!(done, quote);
    assert_eq!(game.caps(), player - quote.price);
    assert_eq!(
        game.shop(&chet()).expect("a shop").caps(),
        vendor + quote.price
    );
    assert_eq!(left(&game), quote.condition.get());
    assert!(quote.condition.get() >= 50, "Chet's Repair is 50");
}

fn give_caps(game: &mut Game, amount: u32) {
    apply_all(
        &[effect(
            serde_json::json!({"type": "caps", "amount": f64::from(amount)}),
        )],
        game,
    )
    .expect("caps are given");
}

#[test]
fn a_player_who_cannot_pay_is_refused_and_nothing_changes() {
    let mut game = worn(0, 140, 1, 0);
    let before = game.save();
    assert!(matches!(
        game.repair_at(&chet(), &item("9mm_pistol")),
        Err(RepairError::PlayerShort { .. })
    ));
    assert_eq!(game.save(), before);
}

#[test]
fn only_a_vendor_with_a_service_repairs_and_only_while_willing() {
    let mut game = worn(0, 140, 1, 0);
    give_caps(&mut game, 500);
    let trudy = VendorId::try_new("trudy").expect("literal is a valid id");
    assert!(matches!(
        game.repair_quote(&trudy, &item("9mm_pistol")),
        Err(RepairError::NoService(_))
    ));
    let store = LocationId::try_new("goodsprings_general_store").expect("literal is valid");
    assert_eq!(game.repairers_at(&store).count(), 1);
    apply_all(
        &[effect(
            serde_json::json!({"type": "kill_npc", "npc": "chet"}),
        )],
        &mut game,
    )
    .expect("Chet dies");
    assert_eq!(game.repairers_at(&store).count(), 0);
    assert!(matches!(
        game.repair_at(&chet(), &item("9mm_pistol")),
        Err(RepairError::Unavailable(_))
    ));
}

#[test]
fn a_vendor_does_not_lift_a_weapon_past_their_own_skill() {
    let mut game = worn(100, 140, 1, 0);
    give_caps(&mut game, 500);
    let quote = game
        .repair_quote(&chet(), &item("9mm_pistol"))
        .expect("a quote");
    // Chet's Repair is 50, whatever the player's: a 140/150 loss ends near half.
    assert!(quote.condition.get() <= 51, "{}", quote.condition.get());
}

fn campfire() -> StationId {
    StationId::try_new("goodsprings_campfire").expect("literal is a valid id")
}

fn powder() -> RecipeId {
    RecipeId::try_new("healing_powder_craft").expect("literal is a valid id")
}

fn find(text: &str) -> FindId {
    FindId::try_new(text).expect("literal is a valid id")
}

#[test]
fn healing_powder_is_made_from_what_was_gathered_and_finishes_the_find() {
    let mut game = TEMPLATE.clone();
    let home = LocationId::try_new("goodsprings_cemetery").expect("literal is valid");
    assert_eq!(game.finds_at(&home).count(), 1);
    game.gather(&find("goodsprings_broc_flower"))
        .expect("picked");
    game.gather(&find("goodsprings_xander_root"))
        .expect("picked");
    assert_eq!(
        game.finds_at(&home).count(),
        0,
        "a taken find is not offered"
    );
    assert_eq!(game.inventory().item_count(&item("broc_flower")), 1);
    let made = game.craft(&campfire(), &powder()).expect("powder is made");
    assert_eq!(made.item, item("healing_powder"));
    assert_eq!(game.inventory().item_count(&item("healing_powder")), 1);
    assert_eq!(game.inventory().item_count(&item("broc_flower")), 0);
    assert_eq!(game.inventory().item_count(&item("xander_root")), 0);
}

#[test]
fn a_find_can_be_taken_only_once() {
    let mut game = TEMPLATE.clone();
    game.gather(&find("goodsprings_broc_flower"))
        .expect("picked");
    let before = game.save();
    assert!(matches!(
        game.gather(&find("goodsprings_broc_flower")),
        Err(GatherError::AlreadyTaken(_))
    ));
    assert!(matches!(
        game.gather(&find("no_such_find")),
        Err(GatherError::UnknownFind(_))
    ));
    assert_eq!(game.save(), before);
    assert_eq!(game.inventory().item_count(&item("broc_flower")), 1);
}

fn ingredients_in_hand() -> Game {
    let mut game = TEMPLATE.clone();
    give(&mut game, "broc_flower", 1);
    give(&mut game, "xander_root", 1);
    game
}

#[test]
fn making_powder_moves_the_campfire_quest_to_stage_thirty_once_it_is_started() {
    let mut game = ingredients_in_hand();
    stage(&mut game, "by_a_campfire_on_the_trail", 10).expect("the quest starts");
    game.craft(&campfire(), &powder()).expect("powder is made");
    let quest = engine::shared::QuestId::try_new("by_a_campfire_on_the_trail").expect("valid id");
    assert_eq!(game.quest_stage(&quest), Some(QuestStage::new(30)));
    assert_eq!(game.quest_status(&quest), QuestStatus::Active);
}

#[test]
fn making_powder_before_the_quest_starts_leaves_the_quest_alone() {
    let mut game = ingredients_in_hand();
    game.craft(&campfire(), &powder()).expect("powder is made");
    let quest = engine::shared::QuestId::try_new("by_a_campfire_on_the_trail").expect("valid id");
    assert_eq!(game.quest_status(&quest), QuestStatus::NotStarted);
}

#[test]
fn making_powder_again_after_stage_thirty_changes_no_story() {
    let mut game = ingredients_in_hand();
    stage(&mut game, "by_a_campfire_on_the_trail", 10).expect("the quest starts");
    game.craft(&campfire(), &powder()).expect("powder is made");
    give(&mut game, "broc_flower", 1);
    give(&mut game, "xander_root", 1);
    game.craft(&campfire(), &powder())
        .expect("more powder is made");
    let quest = engine::shared::QuestId::try_new("by_a_campfire_on_the_trail").expect("valid id");
    assert_eq!(game.quest_stage(&quest), Some(QuestStage::new(30)));
}

#[test]
fn a_craft_with_something_in_the_way_changes_nothing() {
    let mut game = TEMPLATE.clone();
    give(&mut game, "broc_flower", 1);
    let before = game.save();
    assert!(matches!(
        game.craft(&campfire(), &powder()),
        Err(CraftError::Missing { .. })
    ));
    let bench = StationId::try_new("goodsprings_workbench").expect("literal is valid");
    assert!(matches!(
        game.craft(&bench, &powder()),
        Err(CraftError::WrongStation { .. })
    ));
    let kit = RecipeId::try_new("weapon_repair_kit_craft").expect("literal is valid");
    assert!(matches!(
        game.craft(&bench, &kit),
        Err(CraftError::SkillTooLow { .. } | CraftError::Missing { .. })
    ));
    assert!(matches!(
        game.craft(
            &campfire(),
            &RecipeId::try_new("nothing").expect("valid id")
        ),
        Err(CraftError::UnknownRecipe(_))
    ));
    assert_eq!(game.save(), before);
}

#[test]
fn a_trained_player_with_the_parts_makes_a_weapon_repair_kit_at_the_workbench() {
    let mut game = worn(70, 0, 0, 0);
    for part in ["duct_tape", "scrap_electronics", "wonderglue", "wrench"] {
        give(&mut game, part, 1);
    }
    give(&mut game, "scrap_metal", 2);
    let bench = StationId::try_new("goodsprings_workbench").expect("literal is valid");
    let kit = RecipeId::try_new("weapon_repair_kit_craft").expect("literal is valid");
    game.craft(&bench, &kit).expect("the kit is made");
    assert_eq!(game.inventory().item_count(&item("weapon_repair_kit")), 1);
    assert_eq!(game.inventory().item_count(&item("scrap_metal")), 0);
}

#[test]
fn stations_are_offered_only_where_they_stand() {
    let game = TEMPLATE.clone();
    let source = LocationId::try_new("goodsprings_source").expect("literal is valid");
    let found: Vec<&str> = game
        .stations_at(&source)
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(found, ["goodsprings_campfire"]);
    assert_eq!(game.stations_at(&place()).count(), 0);
    let recipes = game.recipes_at(&campfire());
    assert!(
        recipes
            .iter()
            .any(|(id, _)| id.as_str() == "healing_powder_craft")
    );
}

#[test]
fn an_optional_stage_done_after_the_quest_moved_on_is_not_a_refusal() {
    let mut game = TEMPLATE.clone();
    stage(&mut game, "run_goodsprings_run", 10).expect("the quest starts");
    stage(&mut game, "run_goodsprings_run", 40).expect("Doc's supplies come first");
    stage(&mut game, "run_goodsprings_run", 30).expect("Chet's shakedown is still fine");
    let quest = engine::shared::QuestId::try_new("run_goodsprings_run").expect("valid id");
    assert_eq!(game.quest_stage(&quest), Some(QuestStage::new(40)));
    // A stage that is not optional still only moves forward.
    assert!(stage(&mut game, "run_goodsprings_run", 20).is_err());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// A repair at a vendor moves caps from the player to the vendor and nowhere else,
    /// and never leaves the weapon worse.
    #[test]
    fn a_paid_repair_conserves_caps_and_never_hurts(
        lost in 1..150u32, caps in 0..2_000u32, repair in 0..=100u8,
    ) {
        let mut game = worn(repair, lost, 1, 0);
        give_caps(&mut game, caps);
        let total = game.caps() + game.shop(&chet()).expect("a shop").caps();
        let before = left(&game);
        if game.repair_at(&chet(), &item("9mm_pistol")).is_ok() {
            prop_assert!(left(&game) >= before);
        }
        prop_assert_eq!(game.caps() + game.shop(&chet()).expect("a shop").caps(), total);
    }

    /// A craft either happens whole or not at all: ingredients and product move
    /// together, and a refusal leaves the whole game as it was.
    #[test]
    fn a_craft_is_all_or_nothing(flowers in 0..3u32, roots in 0..3u32, started in any::<bool>()) {
        let mut game = TEMPLATE.clone();
        if flowers > 0 { give(&mut game, "broc_flower", flowers); }
        if roots > 0 { give(&mut game, "xander_root", roots); }
        if started {
            stage(&mut game, "by_a_campfire_on_the_trail", 10).expect("the quest starts");
        }
        let before = game.save();
        match game.craft(&campfire(), &powder()) {
            Ok(_) => {
                prop_assert_eq!(game.inventory().item_count(&item("broc_flower")), flowers - 1);
                prop_assert_eq!(game.inventory().item_count(&item("xander_root")), roots - 1);
                prop_assert_eq!(game.inventory().item_count(&item("healing_powder")), 1);
            }
            Err(_) => prop_assert_eq!(game.save(), before),
        }
    }
}
