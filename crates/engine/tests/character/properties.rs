//! The character laws, checked over random attributes and XP.

use engine::character::{Character, CharacterRules, Level, SpecialSet};
use engine::shared::{CharacterView, Skill, Special, SpecialLevel};
use proptest::prelude::*;
use proptest::sample::select;
use strum::IntoEnumIterator;

use super::progression::{book, perk_id};

fn set() -> impl Strategy<Value = SpecialSet> {
    proptest::collection::vec(1_u32..=10, Special::iter().count()).prop_map(|values| {
        Special::iter()
            .zip(values)
            .fold(SpecialSet::default(), |set, (stat, value)| {
                set.with(
                    stat,
                    SpecialLevel::try_new(value).expect("1 to 10 is legal"),
                )
            })
    })
}

fn character(set: SpecialSet, xp: u32) -> Character {
    let mut character = Character::new(set, CharacterRules::standard());
    character.grant_xp(xp);
    character
}

proptest! {
    #[test]
    fn more_xp_never_lowers_the_level_or_the_hit_points(
        set in set(), low in 0_u32..200_000, extra in 0_u32..200_000
    ) {
        let before = character(set, low);
        let after = character(set, low + extra);
        prop_assert!(after.level() >= before.level());
        prop_assert!(after.max_hp() >= before.max_hp());
    }

    #[test]
    fn the_xp_total_of_the_level_is_reached_and_the_next_is_not(
        set in set(), xp in 0_u32..200_000
    ) {
        let rules = CharacterRules::standard();
        let character = character(set, xp);
        prop_assert!(rules.xp_for_level(character.level()) <= xp);
        match character.xp_to_next() {
            Some(remaining) => prop_assert!(remaining > 0),
            None => prop_assert_eq!(character.level(), rules.level_cap()),
        }
    }

    #[test]
    fn the_level_is_the_highest_whose_xp_total_is_reached(set in set(), xp in any::<u32>()) {
        let rules = CharacterRules::standard();
        let character = character(set, xp);
        // The plain definition, checked level by level.
        let scanned = (1..=rules.level_cap().get())
            .filter_map(|n| Level::try_new(n).ok())
            .filter(|level| rules.xp_for_level(*level) <= character.xp())
            .max();
        prop_assert_eq!(Some(character.level()), scanned);
    }

    #[test]
    fn every_skill_stays_within_zero_to_one_hundred(set in set(), xp in any::<u32>()) {
        let character = character(set, xp);
        for skill in Skill::iter() {
            prop_assert!(character.skill(skill).get() <= 100);
        }
    }

    #[test]
    fn the_derived_numbers_follow_the_attributes_up(set in set()) {
        let weaker = character(set.with(Special::Endurance, SpecialLevel::try_new(1).expect("legal")), 0);
        let stronger = character(set.with(Special::Endurance, SpecialLevel::try_new(10).expect("legal")), 0);
        prop_assert!(stronger.max_hp() > weaker.max_hp());
    }
}

/// One thing the player does with the character.
#[derive(Debug, Clone)]
enum Step {
    Earn(u32),
    Spend(Skill),
    Choose(&'static str),
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        (0_u32..4_000).prop_map(Step::Earn),
        select(Skill::iter().collect::<Vec<_>>()).prop_map(Step::Spend),
        select(vec![
            "educated",
            "swift_learner",
            "strong_back",
            "life_giver",
            "action_boy",
            "gun_nut",
            "calm_heart",
            "no_effect",
            "nonesuch",
        ])
        .prop_map(Step::Choose),
    ]
}

/// Plays the steps, checking after each one that a refused step changed nothing.
fn play(set: SpecialSet, steps: &[Step]) -> Result<Character, TestCaseError> {
    let book = book();
    let mut character = Character::new(set, CharacterRules::standard());
    for step in steps {
        let before = character.clone();
        let refused = match step {
            Step::Earn(xp) => {
                character.grant_xp(*xp);
                false
            }
            Step::Spend(skill) => character.spend_skill_point(*skill).is_err(),
            Step::Choose(id) => character.choose_perk(&book, &perk_id(id)).is_err(),
        };
        if refused {
            prop_assert_eq!(&character, &before, "a refusal must change nothing");
        }
    }
    Ok(character)
}

proptest! {
    #[test]
    fn a_refused_step_changes_nothing_and_every_skill_stays_in_range(
        set in set(), steps in proptest::collection::vec(step(), 0..40)
    ) {
        let character = play(set, &steps)?;
        for skill in Skill::iter() {
            prop_assert!(character.skill(skill).get() <= 100);
        }
    }

    #[test]
    fn nothing_is_spent_that_was_not_paid_out(
        set in set(), steps in proptest::collection::vec(step(), 0..40)
    ) {
        let character = play(set, &steps)?;
        let spent: u32 = Skill::iter().map(|skill| character.points_spent(skill)).sum();
        let taken: u32 = character.perks().map(|(_, rank)| u32::from(rank)).sum();
        let level = character.level().get();
        // Educated is the one perk that changes the pay, and it only counts from
        // the level after it was taken, so the bound is the best case.
        let intelligence = u32::from(set.get(Special::Intelligence).get());
        let ceiling = (20 + intelligence + 4) * (level - 1) / 2;
        prop_assert!(spent + character.skill_points() <= ceiling);
        prop_assert_eq!(taken + character.perk_picks(), level / 2);
    }

    #[test]
    fn without_perks_the_points_paid_are_exactly_the_formula(
        set in set(), xp in 0_u32..200_000
    ) {
        let character = character(set, xp);
        let intelligence = u32::from(set.get(Special::Intelligence).get());
        let halves = (20 + intelligence) * (character.level().get() - 1);
        prop_assert_eq!(character.skill_points(), halves / 2);
    }

    #[test]
    fn spending_raises_only_the_chosen_skill_by_one(
        set in set(), xp in 200_u32..200_000, skill in select(Skill::iter().collect::<Vec<_>>())
    ) {
        let mut character = character(set, xp);
        let before: Vec<u8> = Skill::iter().map(|s| character.skill(s).get()).collect();
        if character.spend_skill_point(skill).is_ok() {
            for (index, other) in Skill::iter().enumerate() {
                let expected = before[index] + u8::from(other == skill);
                prop_assert_eq!(character.skill(other).get(), expected);
            }
        }
    }
}
