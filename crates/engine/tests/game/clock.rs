//! The clock through `Game`: it starts in the morning of day 1 and only waiting
//! moves it.

use std::num::NonZeroU32;
use std::sync::Arc;

use engine::clock::{ClockError, GameClock, Span};
use engine::game::Game;
use engine::quests::QuestBook;
use engine::shared::Quest;

fn game() -> Game {
    let quests = QuestBook::new(std::iter::empty::<&Quest>()).expect("no quests is a valid book");
    Game::new(Arc::new(quests))
}

fn hours(n: u32) -> Span {
    Span::hours(NonZeroU32::new(n).expect("the literal is not zero"))
}

#[test]
fn a_new_game_starts_at_the_start_of_the_clock() {
    assert_eq!(game().clock(), GameClock::START);
}

#[test]
fn waiting_moves_the_clock_by_the_span() {
    let mut game = game();
    game.wait(hours(8)).expect("the player is up");
    assert_eq!(game.clock().to_string(), "Day 1, 16:00");
}

#[test]
fn a_clone_keeps_its_own_time() {
    let mut game = game();
    let before = game.clone();
    game.wait(hours(1)).expect("the player is up");
    assert_eq!(before.clock(), GameClock::START);
    assert_ne!(game.clock(), before.clock());
}

#[test]
fn a_player_who_is_down_cannot_wait() {
    let max = game().character().max_hp();
    let mut game = game().with_wounds(max);
    assert_eq!(game.wait(hours(1)), Err(ClockError::Down));
    assert_eq!(game.clock(), GameClock::START);
}
