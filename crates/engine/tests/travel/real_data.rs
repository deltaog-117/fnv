//! The map built from `data/engine/`.

use std::path::Path;

use engine::shared::{GameData, Scope};
use engine::travel::TravelBook;

use crate::support::id;

fn data() -> GameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    GameData::load(&dir, &Scope::Around(id("goodsprings"))).expect("the engine data loads")
}

#[test]
fn the_whole_map_builds_with_every_location_on_it() {
    let data = data();
    let book = TravelBook::from_data(&data).expect("the map builds");
    assert_eq!(book.len(), data.locations.iter().count());
}

#[test]
fn goodsprings_has_its_three_neighbors_nearest_first() {
    let book = TravelBook::from_data(&data()).expect("the map builds");
    let routes = book.routes_from(&id("goodsprings"));
    let names: Vec<&str> = routes.iter().map(|r| r.to.as_str()).collect();
    for expected in [
        "goodsprings_cave",
        "the_devil_s_gullet",
        "goodsprings_cemetery",
    ] {
        assert!(
            names.contains(&expected),
            "{expected} missing from {names:?}"
        );
    }
    assert!(routes.windows(2).all(|pair| pair[0].span <= pair[1].span));
}

#[test]
fn every_link_in_the_data_can_be_walked_both_ways_in_the_same_time() {
    let data = data();
    let book = TravelBook::from_data(&data).expect("the map builds");
    for location in data.locations.iter() {
        for route in book.routes_from(&location.id) {
            assert_eq!(
                book.span_between(&route.to, &location.id),
                Some(route.span),
                "{} to {}",
                location.id,
                route.to
            );
            assert!(route.span.as_minutes() >= 1);
        }
    }
}
