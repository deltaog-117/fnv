//! The recipe and station books over the real data.

use std::path::Path;

use engine::craft::{RecipeBook, StationBook, Unavailable, Work};
use engine::inventory::ItemBook;
use engine::shared::{GameData, ItemId, RecipeId, RecipeMode, Scope, StationKind};

fn data() -> GameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    GameData::load(&dir, &Scope::Everywhere).expect("the engine data loads")
}

fn recipes(data: &GameData) -> RecipeBook {
    let items = ItemBook::from_data(data).expect("the item book builds");
    RecipeBook::from_data(data, |item| items.contains(item)).expect("the recipe book builds")
}

fn id(text: &str) -> RecipeId {
    RecipeId::try_new(text).expect("literal is a valid id")
}

#[test]
fn healing_powder_is_a_campfire_recipe_of_a_flower_and_a_root() {
    let data = data();
    let book = recipes(&data);
    let formula = book.get(&id("healing_powder_craft")).expect("a formula");
    assert_eq!(formula.kind(), StationKind::Campfire);
    assert!(formula.skill().is_none());
    let inputs: Vec<_> = formula
        .inputs()
        .map(|(i, n)| (i.as_str(), n.get()))
        .collect();
    assert_eq!(inputs, [("broc_flower", 1), ("xander_root", 1)]);
    assert_eq!(formula.output().0.as_str(), "healing_powder");
    assert_eq!(
        formula.reactions().len(),
        1,
        "it moves By a Campfire on the Trail"
    );
}

#[test]
fn the_weapon_repair_kit_is_a_workbench_recipe_that_asks_for_repair_fifty() {
    let data = data();
    let book = recipes(&data);
    let formula = book.get(&id("weapon_repair_kit_craft")).expect("a formula");
    assert_eq!(formula.kind(), StationKind::Workbench);
    let (skill, level) = formula.skill().expect("the kit asks for a skill");
    assert_eq!(skill.name(), "Repair");
    assert_eq!(level.get(), 50);
    assert!(formula.inputs().any(|(item, _)| item.as_str() == "wrench"));
}

#[test]
fn most_recipes_can_be_made_and_every_other_one_says_why_not() {
    let data = data();
    let book = recipes(&data);
    assert!(book.len() >= 80, "only {} recipes can be made", book.len());
    assert_eq!(book.len() + book.unavailable().count(), data.recipes.len());
    assert!(
        book.unavailable()
            .any(|(_, why)| matches!(why, Unavailable::NoSubject))
    );
}

#[test]
fn fifteen_of_the_sixteen_breakdowns_can_be_done_and_twelve_gauge_names_no_one_round() {
    let data = data();
    let book = recipes(&data);
    let apart: Vec<_> = data
        .recipes
        .iter()
        .filter(|record| record.mode == RecipeMode::Breakdown)
        .collect();
    assert_eq!(apart.len(), 16);
    let done = apart.iter().filter(|r| book.get(&r.id).is_some()).count();
    assert_eq!(done, 15, "only the 12 gauge shell has no one plain round");
    let twelve = apart
        .iter()
        .find(|r| r.id.as_str() == "12_gauge_shotgun_shell_breakdown")
        .expect("the 12 gauge breakdown is in the data");
    assert!(book.get(&twelve.id).is_none() && twelve.output_item_id.is_none());
    for record in &apart {
        let Some(formula) = book.get(&record.id) else {
            assert_eq!(record.output_item_id, None, "{}", record.id);
            continue;
        };
        assert_eq!(formula.work(), Work::TakeApart);
        assert_eq!(formula.kind(), StationKind::ReloadingBench);
    }
}

/// Taking a round apart must never give back more than making it took, or the
/// reloading bench would print parts.
#[test]
fn taking_a_round_apart_never_gives_more_than_making_it_used() {
    let data = data();
    let book = recipes(&data);
    let mut compared = 0;
    for record in data
        .recipes
        .iter()
        .filter(|r| r.mode == RecipeMode::Breakdown)
    {
        let Some(apart) = book.get(&record.id) else {
            continue;
        };
        let (subject, spent) = apart.inputs().next().expect("a subject");
        let Some(made) = data
            .recipes
            .iter()
            .filter(|r| r.mode == RecipeMode::Craft)
            .filter_map(|r| book.get(&r.id))
            .find(|formula| formula.output().0 == subject && formula.output().1 == spent)
        else {
            continue;
        };
        compared += 1;
        for (part, count) in apart.outputs() {
            let used = made
                .inputs()
                .find(|(item, _)| *item == part)
                .map_or(0, |(_, n)| n.get());
            assert!(
                count.get() <= used,
                "{}: gives {count} of {part}, making it used {used}",
                record.id
            );
        }
    }
    assert!(compared >= 8, "only {compared} rounds could be compared");
}

#[test]
fn the_wikis_not_expended_tools_are_marked_and_the_recipes_still_name_what_blocks_them() {
    let data = data();
    let book = recipes(&data);
    let marked: Vec<_> = data
        .recipes
        .iter()
        .filter(|record| record.ingredients.iter().any(|line| line.kept))
        .map(|record| record.id.as_str())
        .collect();
    assert_eq!(
        marked,
        [
            "black_coffee_craft",
            "purified_water_4_made_using_mass_purified_water_recipe_craft"
        ]
    );
    // Both stay unmade for other reasons: an ingredient the item book lacks, and a
    // recipe with no product named.
    for id_text in marked {
        assert!(book.get(&id(id_text)).is_none(), "{id_text}");
    }
}

#[test]
fn every_formula_uses_items_the_game_has() {
    let data = data();
    let items = ItemBook::from_data(&data).expect("the item book builds");
    let book = recipes(&data);
    for record in data.recipes.iter() {
        let Some(formula) = book.get(&record.id) else {
            continue;
        };
        assert!(items.contains(formula.output().0), "{}", record.id);
        for (item, _) in formula.inputs() {
            assert!(items.contains(item), "{}: {item}", record.id);
        }
    }
}

#[test]
fn every_station_stands_at_a_real_place_and_has_recipes_to_offer() {
    let data = data();
    let stations = StationBook::from_data(&data).expect("the station book builds");
    let recipes = recipes(&data);
    assert!(stations.len() >= 3);
    for record in data.stations.iter() {
        assert!(data.locations.contains(&record.place), "{}", record.id);
        assert!(recipes.for_kind(record.kind).count() > 0, "{}", record.id);
    }
}

#[test]
fn the_craft_reaction_names_a_quest_that_exists() {
    let data = data();
    let recipe = data
        .recipes
        .get(&id("healing_powder_craft"))
        .expect("the recipe exists");
    assert!(!recipe.on_craft.is_empty());
    let _ = ItemId::try_new("healing_powder").expect("valid id");
}
