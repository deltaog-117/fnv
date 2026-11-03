//! The find book and the record of what was taken.

use std::num::NonZeroU32;
use std::path::Path;

use engine::gather::{FindBook, GatherError, Gathered};
use engine::inventory::ItemBook;
use engine::shared::{Find, FindId, FindLine, GameData, ItemId, LocationId, NonEmpty, Scope};
use proptest::prelude::*;

fn find_id(text: &str) -> FindId {
    FindId::try_new(text).expect("literal is a valid id")
}

fn line(item: &str, count: u32) -> FindLine {
    FindLine {
        item: ItemId::try_new(item).expect("valid id"),
        count: NonZeroU32::new(count).expect("a positive count"),
    }
}

fn find(id: &str, place: &str, lines: Vec<FindLine>) -> Find {
    Find {
        id: find_id(id),
        name: id.to_owned(),
        place: LocationId::try_new(place).expect("valid id"),
        items: NonEmpty::try_new(lines).expect("at least one line"),
    }
}

#[test]
fn a_find_is_listed_at_its_place_only() {
    let records = [
        find("a", "field", vec![line("flower", 1)]),
        find("b", "town", vec![line("root", 2)]),
    ];
    let book = FindBook::new(&records).expect("distinct ids build");
    let field = LocationId::try_new("field").expect("valid id");
    let found: Vec<&str> = book.at(&field).map(|(id, _)| id.as_str()).collect();
    assert_eq!(found, ["a"]);
}

#[test]
fn a_repeated_find_id_or_item_is_refused() {
    let twice = [
        find("a", "field", vec![line("x", 1)]),
        find("a", "town", vec![line("y", 1)]),
    ];
    assert_eq!(
        FindBook::new(&twice).expect_err("the id repeats"),
        GatherError::DuplicateFind(find_id("a"))
    );
    let same_item = [find("a", "field", vec![line("x", 1), line("x", 2)])];
    assert!(matches!(
        FindBook::new(&same_item),
        Err(GatherError::DuplicateItem { .. })
    ));
}

#[test]
fn nothing_is_taken_twice() {
    let mut taken = Gathered::default();
    taken.take(&find_id("a")).expect("the first time");
    assert_eq!(
        taken.take(&find_id("a")),
        Err(GatherError::AlreadyTaken(find_id("a")))
    );
    assert!(taken.has_taken(&find_id("a")));
    assert!(!taken.has_taken(&find_id("b")));
}

proptest! {
    /// However often finds are taken, each is taken once and the record is sorted.
    #[test]
    fn each_find_is_taken_at_most_once(picks in prop::collection::vec(0..5u8, 0..30)) {
        let mut taken = Gathered::default();
        let mut first_time = 0;
        for pick in &picks {
            if taken.take(&find_id(&format!("f{pick}"))).is_ok() {
                first_time += 1;
            }
        }
        prop_assert_eq!(taken.len(), first_time);
        let listed: Vec<_> = taken.iter().cloned().collect();
        let mut sorted = listed.clone();
        sorted.sort();
        prop_assert_eq!(listed, sorted);
    }
}

#[test]
fn the_goodsprings_finds_hold_the_ingredients_of_healing_powder() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    let data = GameData::load(&dir, &Scope::Everywhere).expect("the engine data loads");
    let items = ItemBook::from_data(&data).expect("the item book builds");
    let book = FindBook::from_data(&data).expect("the find book builds");
    let carried: Vec<String> = data
        .finds
        .iter()
        .flat_map(|record| {
            book.get(&record.id)
                .into_iter()
                .flat_map(FindInfoItems::items)
        })
        .collect();
    assert!(carried.contains(&"broc_flower".to_owned()));
    assert!(carried.contains(&"xander_root".to_owned()));
    for record in data.finds.iter() {
        assert!(data.locations.contains(&record.place), "{}", record.id);
        for line in record.items.iter() {
            assert!(items.contains(&line.item), "{}: {}", record.id, line.item);
        }
    }
}

/// The names of the items in a find, for the assertion above.
trait FindInfoItems {
    fn items(&self) -> Vec<String>;
}

impl FindInfoItems for engine::gather::FindInfo {
    fn items(&self) -> Vec<String> {
        engine::gather::FindInfo::items(self)
            .map(|(item, _)| item.as_str().to_owned())
            .collect()
    }
}
