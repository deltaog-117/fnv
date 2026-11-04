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
use engine::inventory::{CopyId, ItemBook};
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
    for copy in &mut save.pack.weapons {
        copy.lost = lost;
    }
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

/// The first carried 9mm pistol.
fn pistol(game: &Game) -> CopyId {
    game.inventory()
        .copies_of(&item("9mm_pistol"))
        .next()
        .expect("a pistol is carried")
}

/// What is left of the first carried pistol, in percent.
fn left(game: &Game) -> u8 {
    game.copy_condition(pistol(game))
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
    let after = game.repair_with_kit(pistol(&game)).expect("mended");
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
        game.repair_with_kit(pistol(&game)),
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
        game.repair_with_kit(pistol(&game)),
        Err(RepairError::NotCarried { .. })
    ));
    give(&mut game, "weapon_repair_kit", 1);
    give(&mut game, "stimpak", 1);
    assert!(matches!(
        game.repair_with_kit(CopyId::from_saved(u32::MAX)),
        Err(RepairError::NoSuchWeapon(_))
    ));
    assert_eq!(game.inventory().item_count(&item("weapon_repair_kit")), 1);
    assert_ne!(game.save(), before, "the gifts changed it");
}

#[test]
fn merging_two_copies_leaves_one_better_copy() {
    let mut game = worn(90, 100, 2, 0);
    let before = left(&game);
    let (keep, gone) = pair(&game);
    let after = game.merge_weapons(keep, gone).expect("merged");
    assert_eq!(game.inventory().item_count(&item("9mm_pistol")), 1);
    assert!(after.get() > before);
    assert_eq!(left(&game), after.get());
}

#[test]
fn merging_needs_two_copies_and_changes_nothing_without() {
    let mut game = worn(90, 100, 1, 0);
    let before = game.save();
    let only = pistol(&game);
    assert!(matches!(
        game.merge_weapons(only, only),
        Err(RepairError::NotAPair)
    ));
    assert_eq!(game.save(), before);
}

/// The first two carried pistols.
fn pair(game: &Game) -> (CopyId, CopyId) {
    let mut copies = game.inventory().copies_of(&item("9mm_pistol"));
    (
        copies.next().expect("a first pistol"),
        copies.next().expect("a second pistol"),
    )
}

#[test]
fn merging_keeps_the_copy_chosen_and_the_wear_of_each_is_its_own() {
    let game = worn(90, 100, 2, 0);
    let (first, second) = pair(&game);
    // Make the second copy less worn through a hand-edited save, then merge the
    // first into it: the survivor is the second copy, not the first.
    let mut save = game.save();
    save.pack.weapons[1].lost = 50;
    let mut game = TEMPLATE.restore(&save).expect("loads");
    let before = game.copy_condition(second).expect("it wears");
    let after = game.merge_weapons(second, first).expect("merged");
    assert_eq!(game.inventory().item_count(&item("9mm_pistol")), 1);
    assert_eq!(game.copy_condition(second), Some(after));
    assert!(after > before);
    assert_eq!(game.copy_condition(first), None);
}

#[test]
fn merging_into_a_copy_that_cannot_improve_is_refused_and_changes_nothing() {
    let mut game = worn(90, 100, 2, 0);
    let (first, second) = pair(&game);
    let mut save = game.save();
    save.pack.weapons[1].lost = 0;
    game = TEMPLATE.restore(&save).expect("loads");
    let before = game.save();
    assert!(matches!(
        game.merge_weapons(first, second),
        Err(RepairError::NothingToGain { .. })
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
    let quote = game.repair_quote(&chet(), pistol(&game)).expect("a quote");
    assert!(quote.price > 0);
    let (player, vendor) = (game.caps(), game.shop(&chet()).expect("a shop").caps());
    let done = game.repair_at(&chet(), pistol(&game)).expect("mended");
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
        game.repair_at(&chet(), pistol(&game)),
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
        game.repair_quote(&trudy, pistol(&game)),
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
        game.repair_at(&chet(), pistol(&game)),
        Err(RepairError::Unavailable(_))
    ));
}

#[test]
fn a_vendor_does_not_lift_a_weapon_past_their_own_skill() {
    let mut game = worn(100, 140, 1, 0);
    give_caps(&mut game, 500);
    let quote = game.repair_quote(&chet(), pistol(&game)).expect("a quote");
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

fn reloading_bench() -> StationId {
    StationId::try_new("goodsprings_reloading_bench").expect("literal is valid")
}

fn recipe_id(text: &str) -> RecipeId {
    RecipeId::try_new(text).expect("literal is valid")
}

#[test]
fn a_round_taken_apart_at_the_reloading_bench_gives_back_its_parts() {
    let mut game = worn(30, 0, 0, 0);
    give(&mut game, "9mm_round", 3);
    let made = game
        .craft(&reloading_bench(), &recipe_id("9mm_round_breakdown"))
        .expect("the round is taken apart");
    assert_eq!(made.parts.len(), 4);
    let count = |name: &str| game.inventory().item_count(&item(name));
    assert_eq!(count("9mm_round"), 2, "one round was spent");
    assert_eq!(count("case_9mm"), 1);
    assert_eq!(count("lead"), 11);
    assert_eq!(count("powder_pistol"), 5);
    assert_eq!(count("primer_small_pistol"), 1);
}

#[test]
fn taking_apart_needs_the_item_in_the_pack_and_the_skill_and_changes_nothing_otherwise() {
    let mut game = worn(30, 0, 0, 0);
    let before = game.save();
    assert!(matches!(
        game.craft(&reloading_bench(), &recipe_id("9mm_round_breakdown")),
        Err(CraftError::Missing { .. })
    ));
    // The .22LR breakdown spends three rounds at once.
    give(&mut game, "22lr_round", 2);
    let held = game.save();
    assert!(matches!(
        game.craft(&reloading_bench(), &recipe_id("22lr_round_breakdown")),
        Err(CraftError::Missing {
            need: 3,
            have: 2,
            ..
        })
    ));
    assert_eq!(game.save(), held);
    assert_ne!(held, before);
    let mut untrained = worn(0, 0, 0, 0);
    give(&mut untrained, "50_mg", 1);
    let before = untrained.save();
    assert!(matches!(
        untrained.craft(&reloading_bench(), &recipe_id("50_mg_breakdown")),
        Err(CraftError::SkillTooLow { .. })
    ));
    assert_eq!(untrained.save(), before);
}

#[test]
fn making_a_round_and_taking_it_apart_never_leaves_more_than_was_put_in() {
    let mut game = worn(30, 0, 0, 0);
    for (name, n) in [
        ("case_9mm", 1),
        ("lead", 13),
        ("powder_pistol", 6),
        ("primer_small_pistol", 1),
    ] {
        give(&mut game, name, n);
    }
    game.craft(&reloading_bench(), &recipe_id("9mm_round_craft"))
        .expect("the round is made");
    game.craft(&reloading_bench(), &recipe_id("9mm_round_breakdown"))
        .expect("the round is taken apart");
    let count = |name: &str| game.inventory().item_count(&item(name));
    assert_eq!(count("9mm_round"), 0);
    assert!(count("lead") <= 13 && count("powder_pistol") <= 6);
    assert_eq!(count("case_9mm"), 1);
}

/// A game with one campfire recipe, tea, that needs two leaves and a pot to be
/// carried but not used up.
fn tea_game() -> Game {
    let ingredient = |name: &str, qty: u32, kept: bool| engine::shared::Ingredient {
        item: name.to_owned(),
        item_id: Some(item(name)),
        qty: std::num::NonZeroU32::new(qty).expect("a positive count"),
        kept,
    };
    let tea = engine::shared::Recipe {
        id: recipe_id("tea"),
        name: "Tea".to_owned(),
        mode: engine::shared::RecipeMode::Craft,
        table: None,
        output_qty: None,
        output_item_id: Some(item("tea")),
        skill: None,
        station: Some(engine::shared::StationKind::Campfire),
        ingredients: vec![ingredient("leaf", 2, false), ingredient("pot", 1, true)],
        on_craft: Vec::new(),
    };
    let station = engine::shared::Station {
        id: StationId::try_new("fire").expect("literal is valid"),
        name: "a fire".to_owned(),
        kind: engine::shared::StationKind::Campfire,
        place: place(),
    };
    let items = ItemBook::new([
        (item("leaf"), engine::inventory::ItemClass::Misc, 0.0),
        (item("pot"), engine::inventory::ItemClass::Misc, 1.0),
        (item("tea"), engine::inventory::ItemClass::Consumable, 0.0),
    ])
    .expect("the literal entries are valid");
    let recipes = RecipeBook::new([&tea], |_| true).expect("the book builds");
    let quests = QuestBook::new(std::iter::empty::<&engine::shared::Quest>())
        .expect("no quests is a valid book");
    Game::new(Arc::new(quests))
        .with_items(Arc::new(items))
        .with_recipes(Arc::new(recipes))
        .with_stations(Arc::new(
            StationBook::new([&station]).expect("the station builds"),
        ))
}

#[test]
fn a_tool_must_be_carried_and_is_still_there_after_the_craft() {
    let fire = StationId::try_new("fire").expect("literal is valid");
    let mut game = tea_game();
    give(&mut game, "leaf", 4);
    let before = game.save();
    assert!(matches!(
        game.craft(&fire, &recipe_id("tea")),
        Err(CraftError::Missing {
            have: 0,
            need: 1,
            ..
        })
    ));
    assert_eq!(game.save(), before, "no pot, so nothing changed");

    give(&mut game, "pot", 1);
    game.craft(&fire, &recipe_id("tea")).expect("tea is made");
    game.craft(&fire, &recipe_id("tea")).expect("and again");
    let count = |name: &str| game.inventory().item_count(&item(name));
    assert_eq!(count("tea"), 2);
    assert_eq!(count("leaf"), 0);
    assert_eq!(count("pot"), 1, "the pot is not used up");
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
        if game.repair_at(&chet(), pistol(&game)).is_ok() {
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
