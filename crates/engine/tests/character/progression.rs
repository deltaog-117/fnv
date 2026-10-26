//! Skill points, tags, traits and perks on hand-built characters and a small book.

use engine::character::{
    Character, CharacterRules, CreationError, Level, PerkBook, ProgressError, SpecialSet, Unmet,
};
use engine::shared::{CharacterView, Perk, PerkId, Skill, Special, Trait, TraitId};
use serde_json::json;

use super::sheet::level_of;

pub fn perk_id(text: &str) -> PerkId {
    PerkId::try_new(text).expect("literal is a valid id")
}

pub fn trait_id(text: &str) -> TraitId {
    TraitId::try_new(text).expect("literal is a valid id")
}

/// A book with one perk of each kind the tests need.
pub fn book() -> PerkBook {
    let perks: Vec<Perk> = serde_json::from_value(json!([
        {"id": "educated", "name": "Educated", "dlc": "base",
         "requirements": {"level": 4, "special": {"intelligence": 4}},
         "modifiers": [{"type": "skill_points_per_level", "amount": 2}]},
        {"id": "swift_learner", "name": "Swift Learner", "dlc": "base", "ranks": 3,
         "requirements": {"level": 2, "special": {"intelligence": 4}},
         "modifiers": [{"type": "xp_percent", "percent": 10}]},
        {"id": "strong_back", "name": "Strong Back", "dlc": "base",
         "modifiers": [{"type": "carry_weight", "pounds": 50}]},
        {"id": "life_giver", "name": "Life Giver", "dlc": "base",
         "modifiers": [{"type": "max_hp", "amount": 30}]},
        {"id": "action_boy", "name": "Action Boy", "dlc": "base", "ranks": 2,
         "modifiers": [{"type": "action_points", "amount": 15}]},
        {"id": "gun_nut", "name": "Gun Nut", "dlc": "base",
         "requirements": {"skills": {"guns": 70}}},
        {"id": "calm_heart", "name": "Calm Heart", "dlc": "base",
         "requirements": {"other": "Complete Heartache by the Number peacefully"}},
        {"id": "no_effect", "name": "No Effect", "dlc": "base"},
        {"id": "old_world", "name": "Old World", "dlc": "old_world_blues"}
    ]))
    .expect("the perks parse");
    let traits: Vec<Trait> = serde_json::from_value(json!([
        {"id": "good_natured", "name": "Good Natured", "dlc": "base", "modifiers": [
            {"type": "skill", "skill": "speech", "amount": 5},
            {"type": "skill", "skill": "guns", "amount": -5}]},
        {"id": "skilled", "name": "Skilled", "dlc": "base", "modifiers": [
            {"type": "all_skills", "amount": 5}, {"type": "xp_percent", "percent": -10}]},
        {"id": "kamikaze", "name": "Kamikaze", "dlc": "base"}
    ]))
    .expect("the traits parse");
    PerkBook::new(&perks, &traits, &[engine::shared::Dlc::Base])
}

/// Five in everything but intelligence, which has the five spare points.
fn bright() -> SpecialSet {
    SpecialSet::default().with(Special::Intelligence, level_of(10))
}

fn made(tags: &[Skill], traits: &[&str]) -> Result<Character, CreationError> {
    let traits: Vec<TraitId> = traits.iter().map(|t| trait_id(t)).collect();
    Character::create(bright(), tags, &traits, CharacterRules::standard(), &book())
}

const THREE: [Skill; 3] = [Skill::Guns, Skill::Speech, Skill::Sneak];

fn at_level(character: &mut Character, level: u32) {
    let level = Level::try_new(level).expect("legal level");
    character.grant_xp(CharacterRules::standard().xp_for_level(level) - character.xp());
}

#[test]
fn each_level_up_banks_ten_points_plus_half_the_intelligence_carrying_the_halves() {
    // Intelligence 5: 12.5 points a level.
    let mut character = Character::default();
    assert_eq!(character.skill_points(), 0, "level 1 pays nothing");
    at_level(&mut character, 2);
    assert_eq!(character.skill_points(), 12);
    at_level(&mut character, 3);
    assert_eq!(character.skill_points(), 25, "the two halves make a point");
}

#[test]
fn a_level_pays_once_however_often_the_xp_arrives() {
    let mut character = Character::default();
    at_level(&mut character, 2);
    let banked = character.skill_points();
    character.grant_xp(0);
    character.grant_xp(10);
    assert_eq!(character.skill_points(), banked);
}

#[test]
fn a_jump_over_several_levels_pays_every_level_crossed() {
    let mut character = Character::default();
    at_level(&mut character, 5);
    // Four level-ups of 12.5 points, and a perk at levels 2 and 4.
    assert_eq!(character.skill_points(), 50);
    assert_eq!(character.perk_picks(), 2);
}

#[test]
fn a_perk_is_offered_every_second_level() {
    let mut character = Character::default();
    for (level, picks) in [(2, 1), (3, 1), (4, 2), (5, 2), (6, 3)] {
        at_level(&mut character, level);
        assert_eq!(character.perk_picks(), picks, "level {level}");
    }
}

#[test]
fn spending_a_point_raises_the_skill_by_one_and_costs_one_point() {
    let mut character = Character::default();
    at_level(&mut character, 2);
    let before = character.skill(Skill::Guns).get();
    character.spend_skill_point(Skill::Guns).expect("a point");
    assert_eq!(character.skill(Skill::Guns).get(), before + 1);
    assert_eq!(character.skill_points(), 11);
    assert_eq!(character.points_spent(Skill::Guns), 1);
}

#[test]
fn spending_with_nothing_banked_is_refused_and_changes_nothing() {
    let mut character = Character::default();
    let before = character.clone();
    assert_eq!(
        character.spend_skill_point(Skill::Guns),
        Err(ProgressError::NoSkillPoints)
    );
    assert_eq!(character, before);
}

#[test]
fn a_skill_stops_at_one_hundred_and_the_point_is_kept() {
    let mut character = Character::default();
    at_level(&mut character, 30);
    let mut spent = 0;
    while character.spend_skill_point(Skill::Guns).is_ok() {
        spent += 1;
    }
    assert_eq!(character.skill(Skill::Guns).get(), 100);
    assert_eq!(spent, 100 - 15, "from the starting 15 to 100");
    let kept = character.skill_points();
    assert_eq!(
        character.spend_skill_point(Skill::Guns),
        Err(ProgressError::SkillAtMaximum(Skill::Guns))
    );
    assert_eq!(character.skill_points(), kept);
}

#[test]
fn a_made_character_has_three_tags_each_worth_fifteen() {
    let character = made(&THREE, &[]).expect("a legal character");
    // Guns: 2 + 2 * 5 + 3 = 15, tagged 30. Barter: 2 + 10 + 3 = 15, untagged.
    assert_eq!(character.skill(Skill::Guns).get(), 30);
    assert_eq!(character.skill(Skill::Barter).get(), 15);
    assert!(character.is_tagged(Skill::Sneak));
    assert!(!character.is_tagged(Skill::Barter));
}

#[test]
fn a_character_is_refused_for_the_wrong_points_tags_or_traits() {
    assert_eq!(
        Character::create(
            SpecialSet::default(),
            &THREE,
            &[],
            CharacterRules::standard(),
            &book()
        ),
        Err(CreationError::SpecialTotal {
            expected: 40,
            got: 35
        })
    );
    assert_eq!(
        made(&THREE[..2], &[]),
        Err(CreationError::TagCount {
            expected: 3,
            got: 2
        })
    );
    assert_eq!(
        made(&[Skill::Guns, Skill::Guns, Skill::Sneak], &[]),
        Err(CreationError::DuplicateTag(Skill::Guns))
    );
    assert_eq!(
        made(&THREE, &["good_natured", "skilled", "kamikaze"]),
        Err(CreationError::TooManyTraits { max: 2, got: 3 })
    );
    assert_eq!(
        made(&THREE, &["skilled", "skilled"]),
        Err(CreationError::DuplicateTrait(trait_id("skilled")))
    );
    assert_eq!(
        made(&THREE, &["nonesuch"]),
        Err(CreationError::UnknownTrait(trait_id("nonesuch")))
    );
}

#[test]
fn a_trait_changes_the_skills_it_names_and_the_xp_it_earns() {
    let character = made(&THREE, &["good_natured", "skilled"]).expect("a legal character");
    // Guns: 15 + 15 tag - 5 good natured + 5 skilled. Barter: 15 + 5 skilled.
    assert_eq!(character.skill(Skill::Guns).get(), 30);
    assert_eq!(character.skill(Skill::Barter).get(), 20);
    assert_eq!(character.skill(Skill::Speech).get(), 40);
    let mut character = character;
    character.grant_xp(100);
    assert_eq!(character.xp(), 90, "skilled costs ten percent");
}

#[test]
fn a_perk_needs_a_waiting_pick() {
    let mut character = Character::default();
    assert_eq!(
        character.choose_perk(&book(), &perk_id("strong_back")),
        Err(ProgressError::NoPerkPick)
    );
}

#[test]
fn taking_a_perk_uses_the_pick_and_records_the_rank() {
    let mut character = Character::default();
    at_level(&mut character, 2);
    character
        .choose_perk(&book(), &perk_id("strong_back"))
        .expect("open");
    assert_eq!(character.perk_picks(), 0);
    assert!(character.has_perk(&perk_id("strong_back")));
    assert_eq!(character.perk_rank(&perk_id("strong_back")), 1);
    assert_eq!(character.carry_weight(), 150 + 50 + 50);
}

#[test]
fn the_derived_numbers_follow_the_modifiers() {
    let mut character = Character::default();
    at_level(&mut character, 6);
    let hp = character.max_hp();
    let ap = character.action_points();
    for id in ["life_giver", "action_boy", "action_boy"] {
        character.choose_perk(&book(), &perk_id(id)).expect("open");
    }
    assert_eq!(character.max_hp(), hp + 30);
    // 65 + 15 = 80 is under the cap of 95; two ranks add 30, past it.
    assert_eq!(character.action_points(), ap + 30);
}

#[test]
fn a_perk_with_ranks_stops_at_its_highest_rank() {
    let mut character = Character::default();
    at_level(&mut character, 8);
    for _ in 0..3 {
        character
            .choose_perk(&book(), &perk_id("swift_learner"))
            .expect("open");
    }
    assert_eq!(
        character.choose_perk(&book(), &perk_id("swift_learner")),
        Err(ProgressError::PerkMaxed(perk_id("swift_learner")))
    );
    assert_eq!(character.perk_rank(&perk_id("swift_learner")), 3);
}

#[test]
fn swift_learner_adds_ten_percent_per_rank() {
    let mut character = Character::default();
    at_level(&mut character, 4);
    for _ in 0..2 {
        character
            .choose_perk(&book(), &perk_id("swift_learner"))
            .expect("open");
    }
    let before = character.xp();
    character.grant_xp(100);
    assert_eq!(character.xp() - before, 120);
}

#[test]
fn educated_adds_two_points_to_every_later_level_but_not_the_ones_before() {
    let mut character = Character::new(bright(), CharacterRules::standard());
    at_level(&mut character, 4);
    let before = character.skill_points();
    character
        .choose_perk(&book(), &perk_id("educated"))
        .expect("level 4 and intelligence 10");
    assert_eq!(character.skill_points(), before, "nothing is paid back");
    // Intelligence 10: 15 points a level, 17 with Educated.
    at_level(&mut character, 5);
    assert_eq!(character.skill_points(), before + 17);
}

#[test]
fn a_perk_is_refused_with_the_first_requirement_not_met() {
    let mut character = Character::default();
    at_level(&mut character, 2);
    let lock = |unmet| ProgressError::PerkLocked {
        perk: perk_id("educated"),
        unmet,
    };
    assert_eq!(
        character.choose_perk(&book(), &perk_id("educated")),
        Err(lock(Unmet::Level { need: 4 }))
    );
    assert_eq!(
        character.choose_perk(&book(), &perk_id("gun_nut")),
        Err(ProgressError::PerkLocked {
            perk: perk_id("gun_nut"),
            unmet: Unmet::Skill {
                skill: Skill::Guns,
                need: 70
            }
        })
    );
    assert_eq!(character.perk_picks(), 1, "a refusal keeps the pick");
}

#[test]
fn a_requirement_the_game_cannot_check_keeps_the_perk_shut() {
    let mut character = Character::default();
    at_level(&mut character, 30);
    assert!(matches!(
        character.choose_perk(&book(), &perk_id("calm_heart")),
        Err(ProgressError::PerkLocked {
            unmet: Unmet::Other(_),
            ..
        })
    ));
    let open: Vec<_> = book()
        .open_to(&character)
        .iter()
        .map(|perk| perk.id.to_string())
        .collect();
    assert!(!open.contains(&"calm_heart".to_owned()), "{open:?}");
    assert!(open.contains(&"no_effect".to_owned()), "{open:?}");
}

#[test]
fn a_perk_that_is_not_in_the_book_is_refused() {
    let mut character = Character::default();
    at_level(&mut character, 2);
    for id in ["nonesuch", "old_world"] {
        assert_eq!(
            character.choose_perk(&book(), &perk_id(id)),
            Err(ProgressError::UnknownPerk(perk_id(id))),
            "{id}: the add-on's perks are not on offer"
        );
    }
}

#[test]
fn a_perk_without_modifiers_is_recorded_and_changes_no_number() {
    let mut character = Character::default();
    at_level(&mut character, 2);
    let before = (
        character.max_hp(),
        character.action_points(),
        character.carry_weight(),
        character.skill(Skill::Guns),
    );
    character
        .choose_perk(&book(), &perk_id("no_effect"))
        .expect("open");
    assert!(character.has_perk(&perk_id("no_effect")));
    assert_eq!(
        before,
        (
            character.max_hp(),
            character.action_points(),
            character.carry_weight(),
            character.skill(Skill::Guns)
        )
    );
}
