//! The recipe book and the station book, built from records.

use std::num::NonZeroU32;

use engine::craft::{CraftError, RecipeBook, StationBook, Unavailable, Work};
use engine::shared::{
    Ingredient, ItemId, LocationId, Recipe, RecipeId, RecipeMode, Station, StationId, StationKind,
};
use proptest::prelude::*;

fn item(text: &str) -> ItemId {
    ItemId::try_new(text).expect("literal is a valid id")
}

fn qty(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).expect("a positive count")
}

fn line(text: &str, qty: u32) -> Ingredient {
    Ingredient {
        item: text.to_owned(),
        item_id: Some(item(text)),
        qty: self::qty(qty),
        kept: false,
    }
}

fn tool(text: &str, qty: u32) -> Ingredient {
    Ingredient {
        kept: true,
        ..line(text, qty)
    }
}

fn recipe(id: &str, output: &str, inputs: Vec<Ingredient>) -> Recipe {
    Recipe {
        id: RecipeId::try_new(id).expect("literal is a valid id"),
        name: id.to_owned(),
        mode: RecipeMode::Craft,
        table: None,
        output_qty: None,
        output_item_id: Some(item(output)),
        skill: None,
        station: Some(StationKind::Campfire),
        ingredients: inputs,
        on_craft: Vec::new(),
    }
}

fn everything_exists(_: &ItemId) -> bool {
    true
}

#[test]
fn a_complete_recipe_becomes_a_formula() {
    let records = [recipe(
        "powder",
        "powder",
        vec![line("flower", 1), line("root", 2)],
    )];
    let book = RecipeBook::new(&records, everything_exists).expect("the book builds");
    let formula = book
        .get(&RecipeId::try_new("powder").expect("valid id"))
        .expect("a formula");
    assert_eq!(formula.kind(), StationKind::Campfire);
    assert_eq!(formula.output(), (&item("powder"), qty(1)));
    let inputs: Vec<_> = formula
        .inputs()
        .map(|(i, n)| (i.as_str(), n.get()))
        .collect();
    assert_eq!(inputs, [("flower", 1), ("root", 2)]);
    assert_eq!(book.unavailable().count(), 0);
}

#[test]
fn an_item_listed_twice_is_one_ingredient_of_the_sum() {
    let records = [recipe(
        "mix",
        "mix",
        vec![line("a", 1), line("b", 1), line("a", 2)],
    )];
    let book = RecipeBook::new(&records, everything_exists).expect("the book builds");
    let formula = book
        .get(&RecipeId::try_new("mix").expect("valid id"))
        .expect("a formula");
    let inputs: Vec<_> = formula
        .inputs()
        .map(|(i, n)| (i.as_str(), n.get()))
        .collect();
    assert_eq!(inputs, [("a", 3), ("b", 1)]);
}

#[test]
fn taking_apart_spends_the_item_named_and_gives_the_listed_parts() {
    let mut apart = recipe("apart", "round", vec![line("case", 1), line("lead", 11)]);
    apart.mode = RecipeMode::Breakdown;
    apart.output_qty = Some(qty(3));
    let book = RecipeBook::new([&apart], everything_exists).expect("the book builds");
    let formula = book
        .get(&RecipeId::try_new("apart").expect("valid id"))
        .expect("a formula");
    assert_eq!(formula.work(), Work::TakeApart);
    let spent: Vec<_> = formula
        .inputs()
        .map(|(i, n)| (i.as_str(), n.get()))
        .collect();
    assert_eq!(spent, [("round", 3)]);
    let given: Vec<_> = formula
        .outputs()
        .map(|(i, n)| (i.as_str(), n.get()))
        .collect();
    assert_eq!(given, [("case", 1), ("lead", 11)]);
}

#[test]
fn a_tool_is_carried_and_not_listed_among_what_is_used_up() {
    let records = [recipe(
        "brew",
        "tea",
        vec![line("leaf", 2), tool("pot", 1), tool("pot", 1)],
    )];
    let book = RecipeBook::new(&records, everything_exists).expect("the book builds");
    let formula = book
        .get(&RecipeId::try_new("brew").expect("valid id"))
        .expect("a formula");
    assert_eq!(formula.work(), Work::Make);
    let used: Vec<_> = formula
        .inputs()
        .map(|(i, n)| (i.as_str(), n.get()))
        .collect();
    assert_eq!(used, [("leaf", 2)]);
    let carried: Vec<_> = formula
        .tools()
        .map(|(i, n)| (i.as_str(), n.get()))
        .collect();
    assert_eq!(
        carried,
        [("pot", 2)],
        "a tool listed twice is one tool of the sum"
    );
}

#[test]
fn each_missing_part_gives_its_own_reason() {
    let mut no_subject = recipe("nosubject", "x", vec![line("a", 1)]);
    no_subject.mode = RecipeMode::Breakdown;
    no_subject.output_item_id = None;
    let tools_only = recipe("toolsonly", "x", vec![tool("a", 1)]);
    let mut no_station = recipe("nostation", "x", vec![line("a", 1)]);
    no_station.station = None;
    let mut no_output = recipe("nooutput", "x", vec![line("a", 1)]);
    no_output.output_item_id = None;
    let unknown_output = recipe("unknownout", "ghost", vec![line("a", 1)]);
    let unknown_input = recipe("unknownin", "x", vec![line("ghost", 1)]);
    let mut unmatched = recipe("unmatched", "x", vec![line("a", 1)]);
    unmatched.ingredients[0].item_id = None;
    let empty = recipe("empty", "x", vec![]);
    let records = [
        no_subject,
        tools_only,
        no_station,
        no_output,
        unknown_output,
        unknown_input,
        unmatched,
        empty,
    ];
    let book = RecipeBook::new(&records, |id| id.as_str() != "ghost").expect("the book builds");
    assert!(book.is_empty());
    let reasons: std::collections::BTreeMap<_, _> = book
        .unavailable()
        .map(|(id, why)| (id.as_str().to_owned(), why.clone()))
        .collect();
    assert_eq!(reasons["nosubject"], Unavailable::NoSubject);
    assert_eq!(reasons["toolsonly"], Unavailable::NoIngredients);
    assert_eq!(reasons["nostation"], Unavailable::NoStation);
    assert_eq!(reasons["nooutput"], Unavailable::NoOutput);
    assert_eq!(
        reasons["unknownout"],
        Unavailable::UnknownOutput(item("ghost"))
    );
    assert_eq!(
        reasons["unknownin"],
        Unavailable::UnknownIngredient(item("ghost"))
    );
    assert_eq!(
        reasons["unmatched"],
        Unavailable::UnmatchedIngredient("a".to_owned())
    );
    assert_eq!(reasons["empty"], Unavailable::NoIngredients);
}

#[test]
fn a_repeated_recipe_id_is_refused() {
    let records = [
        recipe("a", "x", vec![line("a", 1)]),
        recipe("a", "y", vec![line("a", 1)]),
    ];
    assert_eq!(
        RecipeBook::new(&records, everything_exists).expect_err("the id repeats"),
        CraftError::DuplicateRecipe(RecipeId::try_new("a").expect("valid id"))
    );
}

#[test]
fn a_station_is_found_at_its_place_only_and_a_repeated_id_is_refused() {
    let station = |id: &str, place: &str| Station {
        id: StationId::try_new(id).expect("valid id"),
        name: id.to_owned(),
        kind: StationKind::Workbench,
        place: LocationId::try_new(place).expect("valid id"),
    };
    let records = [station("a", "home"), station("b", "inn")];
    let book = StationBook::new(&records).expect("distinct ids build");
    let home = LocationId::try_new("home").expect("valid id");
    let found: Vec<&str> = book.at(&home).map(|(id, _)| id.as_str()).collect();
    assert_eq!(found, ["a"]);
    let repeated = [station("a", "home"), station("a", "inn")];
    assert!(matches!(
        StationBook::new(&repeated),
        Err(CraftError::DuplicateStation(_))
    ));
}

/// A recipe with random gaps in it.
fn random_recipe() -> impl Strategy<Value = Recipe> {
    (
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        prop::collection::vec((0..4usize, 1..5u32, any::<bool>()), 0..4),
    )
        .prop_map(|(breakdown, station, output, known_output, lines)| {
            let mut r = recipe(
                "r",
                if known_output { "a" } else { "ghost" },
                lines
                    .iter()
                    .map(|&(which, n, matched)| {
                        let mut l = line(["a", "b", "c", "ghost"][which], n);
                        if !matched {
                            l.item_id = None;
                        }
                        l
                    })
                    .collect(),
            );
            if breakdown {
                r.mode = RecipeMode::Breakdown;
            }
            if !station {
                r.station = None;
            }
            if !output {
                r.output_item_id = None;
            }
            r
        })
}

proptest! {
    /// Every recipe is a formula or has a reason, never both and never neither; a
    /// formula has every part resolved and a positive amount of every ingredient.
    #[test]
    fn every_recipe_is_a_formula_or_has_a_reason(records in prop::collection::vec(random_recipe(), 0..6)) {
        let records: Vec<Recipe> = records
            .into_iter()
            .enumerate()
            .map(|(i, mut r)| {
                r.id = RecipeId::try_new(format!("r{i}")).expect("valid id");
                r
            })
            .collect();
        let book = RecipeBook::new(&records, |id| id.as_str() != "ghost").expect("ids are distinct");
        for record in &records {
            let formula = book.get(&record.id);
            let reason = book.unavailable().find(|(id, _)| *id == &record.id);
            prop_assert_eq!(formula.is_some(), reason.is_none());
            if let Some(formula) = formula {
                prop_assert!(formula.inputs().count() > 0);
                prop_assert!(formula.inputs().all(|(item, n)| item.as_str() != "ghost" && n.get() > 0));
                prop_assert!(formula.outputs().count() > 0);
                prop_assert!(formula.outputs().all(|(item, n)| item.as_str() != "ghost" && n.get() > 0));
            }
        }
        prop_assert_eq!(book.len() + book.unavailable().count(), records.len());
    }
}
