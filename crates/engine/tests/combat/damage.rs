//! The damage steps and the bounded numbers they take.

use engine::combat::{
    AttackProfile, Multiplier, Percent, Points, Power, Rules, damage_after_armor,
    damage_before_armor,
};
use std::num::NonZeroU32;

fn points(value: f64) -> Points {
    Points::try_new(value).expect("valid points")
}

fn profile(base: f64, crit: f64, skill: f64, condition: f64, bonus: f64) -> AttackProfile {
    AttackProfile {
        ap_cost: NonZeroU32::MIN,
        base_damage: points(base),
        crit_damage: points(crit),
        skill: Multiplier::try_new(skill).expect("valid skill"),
        condition: Multiplier::try_new(condition).expect("valid condition"),
        bonus: points(bonus),
    }
}

#[test]
fn damage_before_armor_follows_the_formula() {
    let p = profile(26.0, 10.0, 0.5, 0.8, 3.0);
    assert!(
        (damage_before_armor(&p, Power::Normal, false) - (26.0 * 0.5 * 0.8 + 3.0)).abs() < 1e-9
    );
    assert!(
        (damage_before_armor(&p, Power::Boosted, false) - (26.0 * 0.5 * 0.8 * 2.0 + 3.0)).abs()
            < 1e-9
    );
}

#[test]
fn a_critical_hit_adds_the_critical_damage_to_the_base() {
    let p = profile(20.0, 10.0, 1.0, 1.0, 0.0);
    assert_eq!(damage_before_armor(&p, Power::Normal, true), 30.0);
}

#[test]
fn the_threshold_comes_off_after_resistance() {
    let rules = Rules::standard();
    // 40 at 50% resistance is 20, then a threshold of 10 leaves 10. Taking the
    // threshold first would leave 15, so this pins the order.
    assert_eq!(
        damage_after_armor(&rules, 40.0, points(10.0), points(50.0)),
        10.0
    );
}

#[test]
fn a_fifth_of_the_damage_always_gets_through() {
    let rules = Rules::standard();
    assert_eq!(
        damage_after_armor(&rules, 10.0, points(50.0), points(0.0)),
        2.0
    );
}

#[test]
fn resistance_above_the_cap_counts_as_the_cap() {
    let rules = Rules::standard();
    let capped = damage_after_armor(&rules, 100.0, points(0.0), points(85.0));
    assert_eq!(capped, 20.0);
    assert_eq!(
        damage_after_armor(&rules, 100.0, points(0.0), points(100.0)),
        capped
    );
}

#[test]
fn bounded_numbers_reject_what_is_out_of_range() {
    assert!(Multiplier::try_new(0.49).is_err());
    assert!(Multiplier::try_new(1.01).is_err());
    assert!(Multiplier::try_new(f64::NAN).is_err());
    assert!(Points::try_new(-0.1).is_err());
    assert!(Points::try_new(f64::NAN).is_err());
    assert!(Points::try_new(f64::INFINITY).is_err());
    assert!(Percent::try_new(101).is_err());
    assert!(Rules::new(101.0, 0.2).is_err());
    assert!(Rules::new(85.0, 0.0).is_err());
    assert!(Rules::new(85.0, 1.5).is_err());
    assert!(Rules::new(85.0, f64::NAN).is_err());
}

#[test]
fn bounded_numbers_accept_their_edges() {
    assert!(Multiplier::try_new(0.5).is_ok());
    assert!(Multiplier::try_new(1.0).is_ok());
    assert!(Points::try_new(0.0).is_ok());
    assert_eq!(Percent::try_new(100).map(Percent::get), Ok(100));
    assert!(Rules::new(0.0, 1.0).is_ok());
}
