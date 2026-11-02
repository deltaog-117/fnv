//! Trips through `Game`: the player moves and the clock advances together.

use std::sync::Arc;

use engine::game::Game;
use engine::quests::QuestBook;
use engine::shared::Quest;
use engine::travel::{TravelBook, TravelError};

use crate::support::{book, game, id, line};

#[test]
fn a_game_with_no_map_has_no_place_and_cannot_travel() {
    let quests = QuestBook::new(std::iter::empty::<&Quest>()).expect("valid");
    let mut game = Game::new(Arc::new(quests));
    assert_eq!(game.at(), None);
    assert_eq!(game.travel(&id("b")), Err(TravelError::NoMap));
    assert!(game.routes().is_empty());
}

#[test]
fn the_start_must_be_on_the_map() {
    let quests = QuestBook::new(std::iter::empty::<&Quest>()).expect("valid");
    let result = Game::new(Arc::new(quests)).with_map(book(&line()), &id("nowhere"));
    assert_eq!(result.err(), Some(TravelError::NotOnMap(id("nowhere"))));
}

#[test]
fn travelling_moves_the_player_and_advances_the_clock_by_exactly_the_walk() {
    let records = line();
    let mut game = game(&records, "a");
    let before = game.clock().as_minutes();
    let span = game.travel(&id("b")).expect("a and b are linked");
    assert_eq!(game.at(), Some(&id("b")));
    assert_eq!(game.clock().as_minutes(), before + span.as_minutes());
    assert_eq!(
        Some(span),
        TravelBook::new(&records)
            .expect("valid")
            .span_between(&id("a"), &id("b"))
    );
}

#[test]
fn a_place_that_is_not_linked_is_refused_and_nothing_changes() {
    let mut game = game(&line(), "a");
    let clock = game.clock();
    assert_eq!(
        game.travel(&id("c")),
        Err(TravelError::NoRoute {
            from: id("a"),
            to: id("c")
        })
    );
    assert_eq!(game.at(), Some(&id("a")));
    assert_eq!(game.clock(), clock);
}

#[test]
fn a_place_that_does_not_exist_is_refused_too() {
    let mut game = game(&line(), "a");
    assert!(matches!(
        game.travel(&id("nowhere")),
        Err(TravelError::NoRoute { .. })
    ));
}

#[test]
fn a_trip_there_and_back_costs_twice_the_walk() {
    let mut game = game(&line(), "b");
    let start = game.clock().as_minutes();
    game.travel(&id("c")).expect("linked");
    game.travel(&id("b")).expect("linked back");
    let spent = game.clock().as_minutes() - start;
    let one_way = book(&line())
        .span_between(&id("b"), &id("c"))
        .expect("linked")
        .as_minutes();
    assert_eq!(spent, 2 * one_way);
    assert_eq!(game.at(), Some(&id("b")));
}

#[test]
fn a_player_who_is_down_cannot_travel_and_is_offered_no_routes() {
    let mut game = game(&line(), "a").with_wounds(u32::MAX);
    assert!(!game.is_alive());
    assert!(game.routes().is_empty());
    let clock = game.clock();
    assert_eq!(
        game.travel(&id("b")),
        Err(TravelError::Clock(engine::clock::ClockError::Down))
    );
    assert_eq!(game.at(), Some(&id("a")));
    assert_eq!(game.clock(), clock);
}

#[test]
fn the_routes_from_where_the_player_stands_follow_them_there() {
    let mut game = game(&line(), "a");
    let names = |game: &Game| -> Vec<String> {
        game.routes()
            .into_iter()
            .map(|r| r.to.to_string())
            .collect()
    };
    assert_eq!(names(&game), ["b"]);
    game.travel(&id("b")).expect("linked");
    assert_eq!(names(&game), ["a", "c"]);
}
