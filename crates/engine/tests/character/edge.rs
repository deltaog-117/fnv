//! What perks and traits add to a fight and to healing, on hand-built characters.

use engine::character::{Character, CharacterRules, Level, PerkBook, SpecialSet};
use engine::shared::{FoeKind, ItemId, Perk, PerkId, Skill, Special, Trait, TraitId};
use proptest::prelude::*;
use serde_json::json;

use super::sheet::level_of;

fn book(perk_modifiers: &serde_json::Value, trait_modifiers: &serde_json::Value) -> PerkBook {
    let perks: Vec<Perk> = serde_json::from_value(json!([
        {"id": "trained", "name": "Trained", "dlc": "base", "ranks": 10,
         "modifiers": perk_modifiers}
    ]))
    .expect("the perk parses");
    let traits: Vec<Trait> = serde_json::from_value(json!([
        {"id": "marked", "name": "Marked", "dlc": "base", "modifiers": trait_modifiers}
    ]))
    .expect("the trait parses");
    PerkBook::new(&perks, &traits, &[engine::shared::Dlc::Base])
}

/// A character with the trait `marked`, and `ranks` ranks of the perk `trained`.
fn character(book: &PerkBook, ranks: u32) -> Character {
    let special = SpecialSet::default().with(Special::Intelligence, level_of(10));
    let marked = [TraitId::try_new("marked").expect("literal is a valid id")];
    let mut character = Character::create(
        special,
        &[Skill::Guns, Skill::Speech, Skill::Sneak],
        &marked,
        CharacterRules::standard(),
        book,
    )
    .expect("a legal character");
    let rules = CharacterRules::standard();
    let top = Level::try_new(30).expect("a legal level");
    character.grant_xp(rules.xp_for_level(top));
    let perk = PerkId::try_new("trained").expect("literal is a valid id");
    for _ in 0..ranks {
        character
            .choose_perk(book, &perk)
            .expect("the perk can be taken");
    }
    character
}

fn item(text: &str) -> ItemId {
    ItemId::try_new(text).expect("literal is a valid id")
}

#[test]
fn a_modifier_with_no_condition_applies_to_every_weapon() {
    let book = book(&json!([]), &json!([{"type": "crit_chance", "percent": 3}]));
    let character = character(&book, 0);
    for skill in [None, Some(Skill::Guns), Some(Skill::MeleeWeapons)] {
        assert_eq!(character.edge(skill).crit_chance, 3, "{skill:?}");
    }
}

#[test]
fn a_modifier_for_a_skill_applies_only_to_a_weapon_of_that_skill() {
    let book = book(
        &json!([]),
        &json!([{"type": "ap_cost_percent", "percent": -20, "skill": "guns"}]),
    );
    let character = character(&book, 0);
    assert_eq!(character.edge(Some(Skill::Guns)).ap_cost_percent, -20);
    assert_eq!(
        character.edge(Some(Skill::EnergyWeapons)).ap_cost_percent,
        0
    );
    assert_eq!(
        character.edge(None).ap_cost_percent,
        0,
        "a weapon no skill governs is not a gun"
    );
}

#[test]
fn a_modifier_for_a_kind_of_foe_shows_up_only_against_that_kind() {
    let book = book(
        &json!([]),
        &json!([
            {"type": "damage_percent", "percent": 50, "foe": "mutated_insect"},
            {"type": "crit_damage_percent", "percent": 75, "foe": "animal"}
        ]),
    );
    let edge = character(&book, 0).edge(Some(Skill::Guns));
    assert_eq!(edge.damage_percent, 0, "nothing against everyone");
    let insect = edge
        .against(FoeKind::MutatedInsect)
        .expect("a bonus against insects");
    assert_eq!((insect.damage_percent, insect.crit_damage_percent), (50, 0));
    let animal = edge
        .against(FoeKind::Animal)
        .expect("a bonus against animals");
    assert_eq!((animal.damage_percent, animal.crit_damage_percent), (0, 75));
    assert!(edge.against(FoeKind::Robot).is_none());
}

#[test]
fn a_condition_on_both_the_skill_and_the_foe_needs_both() {
    let book = book(
        &json!([]),
        &json!([{"type": "damage_percent", "percent": 10, "skill": "guns", "foe": "robot"}]),
    );
    let character = character(&book, 0);
    assert!(
        character
            .edge(Some(Skill::Guns))
            .against(FoeKind::Robot)
            .is_some()
    );
    assert!(
        character
            .edge(Some(Skill::MeleeWeapons))
            .against(FoeKind::Robot)
            .is_none()
    );
}

#[test]
fn healing_and_wear_modifiers_name_their_item_or_apply_to_all() {
    let book = book(
        &json!([]),
        &json!([
            {"type": "heal_percent", "percent": 20, "item": "stimpak"},
            {"type": "heal_percent", "percent": 5},
            {"type": "weapon_wear_percent", "percent": 15}
        ]),
    );
    let character = character(&book, 0);
    assert_eq!(character.heal_percent(&item("stimpak")), 25);
    assert_eq!(character.heal_percent(&item("bitter_drink")), 5);
    assert_eq!(character.wear_percent(), 15);
}

#[test]
fn threshold_resistance_and_ignored_threshold_are_summed_apart() {
    let book = book(
        &json!([{"type": "damage_threshold", "amount": 3}]),
        &json!([
            {"type": "damage_threshold", "amount": -2},
            {"type": "damage_resistance", "percent": -15},
            {"type": "ignore_threshold", "amount": 15}
        ]),
    );
    let edge = character(&book, 2).edge(None);
    assert_eq!(edge.threshold, 4, "3 + 3 - 2");
    assert_eq!(edge.resistance, -15);
    assert_eq!(edge.ignored_threshold, 15);
}

proptest! {
    #[test]
    fn ranks_add_by_multiplication_and_a_skill_limit_never_widens_a_bonus(
        percent in any::<i8>(),
        ranks in 0u32..=10,
        only in proptest::option::of(proptest::sample::select(vec![
            Skill::Guns, Skill::EnergyWeapons, Skill::MeleeWeapons, Skill::Unarmed,
        ])),
        weapon in proptest::sample::select(vec![
            None, Some(Skill::Guns), Some(Skill::EnergyWeapons), Some(Skill::MeleeWeapons),
        ]),
    ) {
        let modifier = match only {
            Some(skill) => json!([{"type": "crit_chance", "percent": percent, "skill": skill}]),
            None => json!([{"type": "crit_chance", "percent": percent}]),
        };
        let book = book(&modifier, &json!([]));
        let edge = character(&book, ranks).edge(weapon);
        let applies = only.is_none_or(|skill| Some(skill) == weapon);
        let expected = if applies { i32::from(percent) * i32::try_from(ranks).expect("small") } else { 0 };
        prop_assert_eq!(edge.crit_chance, expected);
    }
}

mod armor_and_close_calls {
    use engine::shared::ArmorClass;

    use super::*;

    fn light_touch() -> serde_json::Value {
        json!([
            {"type": "crit_chance", "percent": 5, "armor": "light"},
            {"type": "crit_chance_against", "percent": -25, "armor": "light"}
        ])
    }

    #[test]
    fn light_touch_works_only_in_light_armor() {
        let book = book(&light_touch(), &json!([]));
        let character = character(&book, 1);
        let light = character.edge_in(None, Some(ArmorClass::Light));
        assert_eq!((light.crit_chance, light.crit_chance_against), (5, -25));
        for other in [None, Some(ArmorClass::Medium), Some(ArmorClass::Heavy)] {
            let edge = character.edge_in(None, other);
            assert_eq!(
                (edge.crit_chance, edge.crit_chance_against),
                (0, 0),
                "{other:?}"
            );
        }
        assert_eq!(character.edge(None), character.edge_in(None, None));
    }

    #[test]
    fn a_modifier_with_no_armor_named_applies_in_any_armor_or_none() {
        let book = book(
            &json!([{"type": "crit_chance_against", "percent": -100}]),
            &json!([]),
        );
        let character = character(&book, 1);
        for worn in [None, Some(ArmorClass::Light), Some(ArmorClass::Power)] {
            assert_eq!(character.edge_in(None, worn).crit_chance_against, -100);
        }
    }

    #[test]
    fn the_reward_for_a_close_fight_is_listed_once_per_rank() {
        let book = book(
            &json!([{"type": "after_fight_boost", "below_health": 25, "stat": "luck",
                     "amount": 4, "minutes": 3}]),
            &json!([]),
        );
        assert!(character(&book, 0).after_fight_rewards().is_empty());
        let rewards = character(&book, 2).after_fight_rewards();
        assert_eq!(rewards.len(), 2);
        assert_eq!(
            (rewards[0].stat, rewards[0].amount, rewards[0].minutes),
            (Special::Luck, 4, 3)
        );
        assert_eq!(rewards[0].below_health.get(), 25);
    }

    proptest! {
        /// Ranks add up, whatever the armor: the order and the count are all that
        /// matter, so the sum is the amount times the ranks.
        #[test]
        fn protection_from_criticals_adds_up_over_ranks(ranks in 0u32..6, percent in -50i8..0) {
            let book = book(
                &json!([{"type": "crit_chance_against", "percent": percent}]),
                &json!([]),
            );
            let character = character(&book, ranks);
            prop_assert_eq!(
                character.edge_in(None, Some(ArmorClass::Light)).crit_chance_against,
                i32::from(percent) * i32::try_from(ranks).expect("a few ranks")
            );
        }
    }
}
