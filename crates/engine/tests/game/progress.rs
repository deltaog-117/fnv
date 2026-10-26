//! Leveling through `Game`: the book is held by the game, and what the character
//! earns reaches conditions and hit points.

use std::path::Path;
use std::sync::Arc;

use engine::character::{Character, CharacterRules, PerkBook, ProgressError, SpecialSet};
use engine::game::Game;
use engine::quests::QuestBook;
use engine::shared::{
    CharacterView, Condition, Effect, GameData, PerkId, Quest, Scope, Skill, Special, SpecialLevel,
    apply, evaluate,
};
use serde_json::json;

fn perk(id: &str) -> PerkId {
    PerkId::try_new(id).expect("literal is a valid id")
}

fn game() -> Game {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    let data = GameData::load(&dir, &Scope::Everywhere).expect("the engine data loads");
    let quests = QuestBook::new(std::iter::empty::<&Quest>()).expect("no quests is a valid book");
    Game::new(Arc::new(quests)).with_perks(Arc::new(PerkBook::base_game(&data)))
}

fn earn(game: &mut Game, amount: f64) {
    apply(&Effect::Xp { amount }, game).expect("xp is built");
}

#[test]
fn xp_from_an_effect_levels_the_character_and_banks_what_the_level_gives() {
    let mut game = game();
    earn(&mut game, 200.0);
    assert_eq!(game.character().level().get(), 2);
    assert_eq!(game.character().skill_points(), 12);
    assert_eq!(game.character().perk_picks(), 1);
}

#[test]
fn a_point_spent_through_the_game_is_seen_by_a_condition() {
    let mut game = game();
    earn(&mut game, 200.0);
    let condition: Condition =
        serde_json::from_value(json!({"type": "skill", "skill": "guns", "min": 16}))
            .expect("the literal is a valid condition");
    assert!(!evaluate(&condition, &game));
    game.spend_skill_point(Skill::Guns).expect("a point");
    assert!(evaluate(&condition, &game));
}

#[test]
fn a_perk_taken_through_the_game_is_seen_by_a_condition_and_raises_the_hit_points() {
    // Life Giver asks for level 12 and Endurance 6; the default character has 5.
    let special =
        SpecialSet::default().with(Special::Endurance, SpecialLevel::try_new(6).expect("legal"));
    let mut character = Character::new(special, CharacterRules::standard());
    character.grant_xp(200_000);
    let mut game = game().with_character(character);
    let before = game.hp();
    assert!(!game.character().has_perk(&perk("life_giver")));
    game.choose_perk(&perk("life_giver")).expect("open");
    let condition: Condition =
        serde_json::from_value(json!({"type": "perk", "perk": "life_giver"}))
            .expect("the literal is a valid condition");
    assert!(evaluate(&condition, &game));
    assert_eq!(game.hp(), before + 30);
}

#[test]
fn a_game_with_no_book_offers_no_perk() {
    let quests = QuestBook::new(std::iter::empty::<&Quest>()).expect("no quests is a valid book");
    let mut game = Game::new(Arc::new(quests));
    earn(&mut game, 200.0);
    assert_eq!(
        game.choose_perk(&perk("educated")),
        Err(ProgressError::UnknownPerk(perk("educated")))
    );
    assert_eq!(game.character().perk_picks(), 1, "the pick is kept");
}
