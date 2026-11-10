//! Starting, replacing and ending boosts.

use engine::effects::{ActiveEffects, BoostSource};
use engine::shared::{Boost, BoostStat, ItemId, Skill, Special};
use proptest::prelude::*;
use serde_json::json;

fn item(text: &str) -> ItemId {
    ItemId::try_new(text.to_owned()).expect("literal is a valid id")
}

fn boost(stat: &str, amount: i16, minutes: u32) -> Boost {
    serde_json::from_value(json!({"stat": stat, "amount": amount, "minutes": minutes}))
        .expect("the literal is a valid boost")
}

#[test]
fn a_boost_is_in_force_until_the_minute_it_ends_and_gone_at_that_minute() {
    let mut effects = ActiveEffects::default();
    effects.start(&item("mentats"), &[boost("intelligence", 2, 4)], 100);
    assert!(
        !effects.expire(103),
        "still in force a minute before the end"
    );
    assert_eq!(effects.tally().special(Special::Intelligence), 2);
    assert!(effects.expire(104), "gone at the minute it ends");
    assert!(effects.is_empty());
    assert!(effects.tally().is_empty());
}

#[test]
fn each_boost_of_one_dose_lasts_its_own_length() {
    let mut effects = ActiveEffects::default();
    let dose = [boost("strength", 2, 1), boost("endurance", 3, 4)];
    effects.start(&item("buffout"), &dose, 0);
    assert!(effects.expire(1));
    assert_eq!(effects.len(), 1);
    assert_eq!(effects.tally().special(Special::Strength), 0);
    assert_eq!(effects.tally().special(Special::Endurance), 3);
}

#[test]
fn a_new_dose_of_the_same_item_replaces_the_old_one_instead_of_stacking() {
    let mut effects = ActiveEffects::default();
    effects.start(&item("buffout"), &[boost("strength", 2, 4)], 0);
    effects.start(&item("buffout"), &[boost("strength", 2, 4)], 3);
    assert_eq!(effects.tally().special(Special::Strength), 2);
    assert!(!effects.expire(5), "the second dose runs to minute 7");
    assert!(effects.expire(7));
}

#[test]
fn different_items_stack() {
    let mut effects = ActiveEffects::default();
    effects.start(&item("buffout"), &[boost("strength", 2, 4)], 0);
    effects.start(&item("ant_nectar"), &[boost("strength", 4, 4)], 0);
    assert_eq!(effects.tally().special(Special::Strength), 6);
}

#[test]
fn a_penalty_and_a_bonus_to_one_stat_cancel() {
    let mut effects = ActiveEffects::default();
    effects.start(&item("a"), &[boost("guns", 10, 4)], 0);
    effects.start(&item("b"), &[boost("guns", -10, 4)], 0);
    assert_eq!(effects.tally().skill(Skill::Guns), 0);
}

#[test]
fn a_length_that_would_pass_the_end_of_time_saturates() {
    let mut effects = ActiveEffects::default();
    effects.start(&item("a"), &[boost("luck", 1, u32::MAX)], u64::MAX - 5);
    let until = effects.iter().next().expect("one boost").until;
    assert_eq!(until, u64::MAX);
}

fn stat_name() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec![
        "strength",
        "luck",
        "guns",
        "max_hp",
        "damage_resistance",
    ])
}

fn dose() -> impl Strategy<Value = (usize, Vec<Boost>, u64)> {
    (
        0..3_usize,
        prop::collection::vec(
            (stat_name(), prop_oneof![-30_i16..-1, 1_i16..30], 1_u32..20)
                .prop_map(|(stat, amount, minutes)| boost(stat, amount, minutes)),
            1..4,
        ),
        0_u64..30,
    )
}

proptest! {
    #[test]
    fn after_expiring_nothing_in_force_ends_at_or_before_now(
        doses in prop::collection::vec(dose(), 0..12), now in 0_u64..80,
    ) {
        let names = ["a", "b", "c"];
        let mut effects = ActiveEffects::default();
        for (who, boosts, at) in &doses {
            effects.start(&item(names[*who]), boosts, *at);
        }
        effects.expire(now);
        prop_assert!(effects.iter().all(|active| active.until > now));
    }

    #[test]
    fn the_tally_is_the_sum_of_what_is_in_force(
        doses in prop::collection::vec(dose(), 0..12), now in 0_u64..80,
    ) {
        let names = ["a", "b", "c"];
        let mut effects = ActiveEffects::default();
        for (who, boosts, at) in &doses {
            effects.start(&item(names[*who]), boosts, *at);
        }
        effects.expire(now);
        let sum = |stat: BoostStat| -> i32 {
            effects
                .iter()
                .filter(|active| active.stat == stat)
                .map(|active| i32::from(active.amount))
                .sum()
        };
        let tally = effects.tally();
        prop_assert_eq!(tally.special(Special::Strength), sum(BoostStat::Special(Special::Strength)));
        prop_assert_eq!(tally.special(Special::Luck), sum(BoostStat::Special(Special::Luck)));
        prop_assert_eq!(tally.skill(Skill::Guns), sum(BoostStat::Skill(Skill::Guns)));
    }

    #[test]
    fn each_item_has_at_most_one_dose_in_force(doses in prop::collection::vec(dose(), 0..12)) {
        let names = ["a", "b", "c"];
        let mut effects = ActiveEffects::default();
        for (who, boosts, at) in &doses {
            effects.start(&item(names[*who]), boosts, *at);
        }
        for name in names {
            let count = effects.iter().filter(|a| a.source == BoostSource::Item(item(name))).count();
            let last = doses.iter().rev().find(|(who, _, _)| names[*who] == name);
            prop_assert_eq!(count, last.map_or(0, |(_, boosts, _)| boosts.len()));
        }
    }
}
