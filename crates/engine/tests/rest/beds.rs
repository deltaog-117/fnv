//! The bed book, built from records and from `data/engine/`.

use std::path::Path;

use engine::rest::{BedBook, RestError};
use engine::shared::{Bed, BedId, GameData, LocationId, Scope};

fn bed(id: &str, place: &str) -> Bed {
    Bed {
        id: BedId::try_new(id).expect("literal is a valid id"),
        name: format!("The {id}"),
        place: LocationId::try_new(place).expect("literal is a valid id"),
    }
}

#[test]
fn a_bed_is_found_at_its_place_only() {
    let records = [bed("a", "home"), bed("b", "inn")];
    let book = BedBook::new(&records).expect("distinct ids build");
    let home = LocationId::try_new("home").expect("valid id");
    let found: Vec<&str> = book.at(&home).map(|(id, _)| id.as_str()).collect();
    assert_eq!(found, ["a"]);
    let nowhere = LocationId::try_new("nowhere").expect("valid id");
    assert_eq!(book.at(&nowhere).count(), 0);
}

#[test]
fn a_repeated_bed_id_is_refused() {
    let records = [bed("a", "home"), bed("a", "inn")];
    assert_eq!(
        BedBook::new(&records).expect_err("the id repeats"),
        RestError::DuplicateBed(BedId::try_new("a").expect("valid id"))
    );
}

#[test]
fn the_goodsprings_bed_is_in_doc_mitchells_house() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    let data = GameData::load(&dir, &Scope::Everywhere).expect("the engine data loads");
    let book = BedBook::from_data(&data).expect("the bed book builds");
    let house = LocationId::try_new("doc_mitchell_s_house").expect("valid id");
    assert_eq!(book.at(&house).count(), 1);
    assert!(
        data.locations.contains(&house),
        "the bed must stand in a known place"
    );
}
