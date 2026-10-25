//! Derived numbers and XP handling on hand-built characters.

use std::sync::Arc;

use engine::character::{Character, CharacterRules, Level, SpecialSet};
use engine::game::Game;
use engine::quests::QuestBook;
use engine::shared::{
    CharacterSink, CharacterView, Condition, EffectError, Quest, Skill, Special, SpecialLevel,
    evaluate,
};
use serde_json::json;

pub fn level_of(value: u32) -> SpecialLevel {
    SpecialLevel::try_new(value).expect("literal is a legal level")
}

fn with(stat: Special, value: u32) -> Character {
    Character::new(
        SpecialSet::default().with(stat, level_of(value)),
        CharacterRules::standard(),
    )
}

fn earn(character: &mut Character, xp: u32) {
    character.grant_xp(xp);
}

#[test]
fn a_new_character_is_level_one_with_nothing_earned() {
    let character = Character::default();
    assert_eq!(character.level(), Level::FIRST);
    assert_eq!(character.xp(), 0);
    assert_eq!(character.xp_to_next(), Some(200));
}

#[test]
fn the_level_turns_over_exactly_at_the_xp_total() {
    let mut character = Character::default();
    earn(&mut character, 199);
    assert_eq!(character.level().get(), 1);
    assert_eq!(character.xp_to_next(), Some(1));
    earn(&mut character, 1);
    assert_eq!(character.level().get(), 2);
    assert_eq!(character.xp_to_next(), Some(350));
}

#[test]
fn xp_keeps_counting_past_the_cap_without_raising_the_level() {
    let mut character = Character::default();
    earn(&mut character, u32::MAX);
    assert_eq!(character.level(), CharacterRules::standard().level_cap());
    assert_eq!(character.xp_to_next(), None);
    earn(&mut character, 1);
    assert_eq!(character.xp(), u32::MAX, "saturates instead of wrapping");
}

#[test]
fn a_five_in_everything_gets_the_documented_numbers() {
    let character = Character::default();
    assert_eq!(character.max_hp(), 200);
    assert_eq!(character.action_points(), 80);
    assert_eq!(character.carry_weight(), 200);
    // 2 + 2 * 5 + ceil(5 / 2)
    assert_eq!(character.skill(Skill::Guns).get(), 15);
}

#[test]
fn hit_points_grow_five_a_level() {
    let mut character = Character::default();
    earn(&mut character, 200);
    assert_eq!(character.max_hp(), 205);
}

#[test]
fn action_points_stop_at_the_base_cap() {
    assert_eq!(with(Special::Agility, 10).action_points(), 95);
    let capped = Character::new(
        SpecialSet::default().with(Special::Agility, level_of(10)),
        CharacterRules::new(200, 150, 30, 90).expect("valid rules"),
    );
    assert_eq!(capped.action_points(), 90);
}

#[test]
fn each_skill_starts_from_its_own_attribute() {
    assert_eq!(
        with(Special::Strength, 10).skill(Skill::MeleeWeapons).get(),
        25
    );
    assert_eq!(with(Special::Strength, 10).skill(Skill::Guns).get(), 15);
    assert_eq!(with(Special::Charisma, 1).skill(Skill::Speech).get(), 7);
}

/// A game over no quests, for conditions: `evaluate` needs the whole state.
fn game_of(character: Character) -> Game {
    let book = QuestBook::new(std::iter::empty::<&Quest>()).expect("no quests is a valid book");
    Game::new(Arc::new(book)).with_character(character)
}

#[test]
fn a_condition_reads_the_character() {
    let condition: Condition =
        serde_json::from_value(json!({"type": "skill", "skill": "guns", "min": 15}))
            .expect("the literal is a valid condition");
    assert!(evaluate(&condition, &game_of(Character::default())));
    assert!(!evaluate(&condition, &game_of(with(Special::Agility, 1))));
}

#[test]
fn xp_effects_add_whole_amounts() {
    let mut character = Character::default();
    character.add_xp(150.0).expect("a whole amount");
    assert_eq!(character.xp(), 150);
}

#[test]
fn xp_effects_refuse_what_is_not_a_gain_of_whole_points() {
    let mut character = Character::default();
    assert!(matches!(
        character.add_xp(f64::NAN),
        Err(EffectError::NotFinite { effect: "xp" })
    ));
    for bad in [-1.0, 0.5, 1e30] {
        assert!(
            matches!(character.add_xp(bad), Err(EffectError::Refused { .. })),
            "{bad}"
        );
    }
    assert_eq!(character.xp(), 0, "a refusal changes nothing");
}

#[test]
fn levels_outside_the_range_cannot_be_built() {
    assert!(Level::try_new(0).is_err());
    assert!(Level::try_new(1).is_ok());
    assert!(Level::try_new(50).is_ok());
    assert!(Level::try_new(51).is_err());
}

#[test]
fn rules_that_would_make_leveling_meaningless_are_rejected() {
    assert!(CharacterRules::new(0, 150, 30, 95).is_err());
    assert!(CharacterRules::new(200, 150, 0, 95).is_err());
    assert!(CharacterRules::new(200, 150, 51, 95).is_err());
    assert!(CharacterRules::new(200, 150, 30, 0).is_err());
}
