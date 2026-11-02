//! Builders shared by the travel tests.

use std::sync::Arc;

use engine::game::Game;
use engine::quests::QuestBook;
use engine::shared::{Location, LocationId, Quest};
use engine::travel::TravelBook;
use serde_json::json;

pub fn id(text: &str) -> LocationId {
    LocationId::try_new(text.to_owned()).expect("literal is a valid id")
}

/// A location record named after its id, with links to `neighbors` by distance.
pub fn location(text: &str, neighbors: &[(&str, f64)]) -> Location {
    let links: Vec<_> = neighbors
        .iter()
        .map(|(to, distance)| json!({"id": to, "distance": distance}))
        .collect();
    serde_json::from_value(json!({
        "id": text, "name": text.to_uppercase(), "kind": "settlement", "dlc": "base",
        "neighbors": links,
    }))
    .expect("the literal is a valid location record")
}

/// A line of three places: `a` 10 pixels from `b`, `b` 30 pixels from `c`.
pub fn line() -> Vec<Location> {
    vec![
        location("a", &[("b", 10.0)]),
        location("b", &[("c", 30.0)]),
        location("c", &[]),
    ]
}

pub fn book(records: &[Location]) -> Arc<TravelBook> {
    Arc::new(TravelBook::new(records).expect("the literal map is valid"))
}

/// A game standing at `start` on the map of `records`.
pub fn game(records: &[Location], start: &str) -> Game {
    let quests = QuestBook::new(std::iter::empty::<&Quest>()).expect("no quests is a valid book");
    Game::new(Arc::new(quests))
        .with_map(book(records), &id(start))
        .expect("the start is on the map")
}
