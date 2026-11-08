//! Perks and traits that depend on wounds and the hour, through `Game`: the game
//! tells the character each time either changes.

use std::num::NonZeroU32;
use std::path::Path;
use std::sync::Arc;

use engine::character::{Character, CharacterRules, PerkBook, SpecialSet};
use engine::clock::Span;
use engine::game::Game;
use engine::quests::QuestBook;
use engine::shared::{
    CharacterView, Dlc, GameData, Quest, Scope, Skill, Special, SpecialLevel, TraitId,
};

fn game_with(trait_id: &str) -> Game {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    let data = GameData::load(&dir, &Scope::Everywhere).expect("the engine data loads");
    // Both traits belong to Old World Blues, which the real game does not offer yet.
    let book = PerkBook::new(
        data.perks.iter(),
        data.traits.iter(),
        &[Dlc::Base, Dlc::OldWorldBlues],
    );
    let five = SpecialLevel::try_new(5).expect("legal");
    let special = SpecialSet::uniform(five)
        .with(Special::Strength, SpecialLevel::try_new(10).expect("legal"));
    let character = Character::create(
        special,
        &[Skill::Guns, Skill::Speech, Skill::Sneak],
        &[TraitId::try_new(trait_id).expect("a valid id")],
        CharacterRules::standard(),
        &book,
    )
    .expect("a legal character");
    let quests = QuestBook::new(std::iter::empty::<&Quest>()).expect("no quests is a valid book");
    Game::new(Arc::new(quests)).with_character(character)
}

fn hours(n: u32) -> Span {
    Span::hours(NonZeroU32::new(n).expect("the literal is not zero"))
}

#[test]
fn hot_blooded_switches_on_with_the_wounds_and_off_with_the_healing() {
    let healthy = game_with("hot_blooded");
    let max = healthy.character().max_hp();
    let agility = healthy.special(Special::Agility).get();

    let mut hurt = game_with("hot_blooded").with_wounds(max * 6 / 10);
    assert_eq!(hurt.special(Special::Agility).get(), agility - 2);
    assert_eq!(hurt.character().edge(None).damage_percent, 10);

    hurt.heal(max);
    assert_eq!(hurt.special(Special::Agility).get(), agility);
    assert_eq!(hurt.character().edge(None).damage_percent, 0);
}

#[test]
fn early_bird_follows_the_clock_and_the_maximum_hit_points_follow_it() {
    let mut game = game_with("early_bird");
    // A new game starts at 08:00: morning, +2 to every attribute.
    let morning_max = game.character().max_hp();
    game.wait(hours(6)).expect("the player is up");
    let afternoon_max = game.character().max_hp();
    assert_eq!(
        morning_max - afternoon_max,
        40,
        "two points of Endurance, 20 each"
    );
    game.wait(hours(4)).expect("the player is up");
    assert_eq!(
        afternoon_max - game.character().max_hp(),
        20,
        "18:00 takes one point of Endurance away"
    );
}

#[test]
fn a_falling_maximum_never_leaves_a_living_player_down() {
    let mut game = game_with("early_bird");
    let max = game.character().max_hp();
    // One hit point left, in the morning, when the maximum is highest.
    game = game.with_wounds(max - 1);
    assert_eq!(game.hp(), 1);
    game.wait(hours(10)).expect("the player is up");
    assert!(game.is_alive(), "the evening's lower maximum must not kill");
    assert_eq!(game.hp(), 1);
}

#[test]
fn a_player_who_is_down_stays_down_when_the_hour_changes() {
    let mut game = game_with("early_bird");
    let max = game.character().max_hp();
    game = game.with_wounds(max);
    assert!(!game.is_alive());
    // Waiting is refused while down, and the situation does not revive anyone.
    assert!(game.wait(hours(1)).is_err());
    assert!(!game.is_alive());
}
