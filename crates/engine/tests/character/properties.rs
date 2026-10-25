//! The character laws, checked over random attributes and XP.

use engine::character::{Character, CharacterRules, SpecialSet};
use engine::shared::{CharacterView, Skill, Special, SpecialLevel};
use proptest::prelude::*;
use strum::IntoEnumIterator;

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
