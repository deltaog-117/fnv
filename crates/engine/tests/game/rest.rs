//! Sleep through `Game`: it needs a bed, closes wounds and moves the clock, and
//! plain waiting still restores nothing.

use std::num::NonZeroU32;
use std::sync::Arc;

use engine::clock::{ClockError, GameClock, Span};
use engine::game::Game;
use engine::quests::QuestBook;
use engine::rest::{BedBook, RestError};
use engine::shared::{Bed, BedId, LocationId, Quest};
use proptest::prelude::*;

fn bed_id() -> BedId {
    BedId::try_new("cot").expect("literal is a valid id")
}

fn house() -> LocationId {
    LocationId::try_new("house").expect("literal is a valid id")
}

fn game() -> Game {
    let quests = QuestBook::new(std::iter::empty::<&Quest>()).expect("no quests is a valid book");
    let cot = Bed {
        id: bed_id(),
        name: "A cot".to_owned(),
        place: house(),
    };
    Game::new(Arc::new(quests)).with_beds(Arc::new(BedBook::new([&cot]).expect("one bed builds")))
}

fn hours(n: u32) -> Span {
    Span::hours(NonZeroU32::new(n).expect("the literal is not zero"))
}

#[test]
fn a_full_sleep_heals_every_wound_and_moves_the_clock() {
    let mut game = game().with_wounds(30);
    let healed = game.sleep(&bed_id(), hours(8)).expect("the bed exists");
    assert_eq!(healed, 30);
    assert_eq!(game.hp(), game.character().max_hp());
    assert_eq!(game.clock().to_string(), "Day 1, 16:00");
}

#[test]
fn the_report_is_what_was_healed_not_what_the_sleep_could_have_healed() {
    let mut game = game().with_wounds(2);
    assert_eq!(game.sleep(&bed_id(), hours(1)), Ok(2));
}

#[test]
fn a_short_sleep_heals_only_a_share() {
    let max = game().character().max_hp();
    // One hit point left: as wounded as a player can be and still sleep.
    let wounds = max - 1;
    let mut game = game().with_wounds(wounds);
    let healed = game.sleep(&bed_id(), hours(1)).expect("the bed exists");
    assert!(healed > 0 && healed < wounds, "healed {healed} of {wounds}");
    assert_eq!(game.hp(), 1 + healed);
}

#[test]
fn a_bed_that_does_not_exist_is_refused_and_changes_nothing() {
    let mut game = game().with_wounds(30);
    let nobody = BedId::try_new("nobody").expect("literal is a valid id");
    assert_eq!(
        game.sleep(&nobody, hours(8)),
        Err(RestError::UnknownBed(nobody))
    );
    assert_eq!(game.clock(), GameClock::START);
    assert_eq!(game.hp(), game.character().max_hp() - 30);
}

#[test]
fn a_player_who_is_down_cannot_sleep() {
    let max = game().character().max_hp();
    let mut game = game().with_wounds(max);
    assert_eq!(
        game.sleep(&bed_id(), hours(8)),
        Err(RestError::Clock(ClockError::Down))
    );
    assert_eq!(game.hp(), 0);
    assert_eq!(game.clock(), GameClock::START);
}

#[test]
fn waiting_restores_nothing() {
    let mut game = game().with_wounds(30);
    game.wait(hours(8)).expect("the player is up");
    assert_eq!(game.hp(), game.character().max_hp() - 30);
}

#[test]
fn beds_are_offered_at_their_place_only_and_not_to_a_player_who_is_down() {
    let game = game();
    assert_eq!(game.beds_at(&house()).count(), 1);
    assert_eq!(
        game.beds_at(&LocationId::try_new("street").expect("literal is a valid id"))
            .count(),
        0
    );
    let down = game.clone().with_wounds(game.character().max_hp());
    assert_eq!(down.beds_at(&house()).count(), 0);
}

proptest! {
    #[test]
    fn sleeping_never_pushes_hit_points_past_the_maximum(
        wounds in 0..200u32, sleep_hours in 1..=48u32,
    ) {
        let mut game = game().with_wounds(wounds);
        let max = game.character().max_hp();
        let before = game.hp();
        if game.is_alive() {
            let healed = game.sleep(&bed_id(), hours(sleep_hours)).expect("the bed exists");
            prop_assert_eq!(game.hp(), before + healed);
            prop_assert!(game.hp() <= max);
        }
    }
}
