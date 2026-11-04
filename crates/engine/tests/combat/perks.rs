//! What training does in a fight: damage against a kind of foe, and threshold ignored.

use engine::combat::{Against, AttackProfile, Multiplier, Points, Power, Side, Strike};
use engine::shared::FoeKind;
use proptest::prelude::*;
use std::num::NonZeroU32;

use crate::support::{Plain, Rolls, duel};

/// The damage of one sure, non-critical hit from `player` on `enemy`.
fn hit(player: Plain, enemy: Plain) -> u32 {
    let mut fight = duel(player, enemy, 100);
    let target = fight
        .ids()
        .find(|&id| fight.side(id) == Some(Side::Enemy))
        .expect("an enemy");
    let strike = fight
        .attack(target, Power::Normal, &mut Rolls::new(&[1, 100]))
        .expect("the attack lands")
        .strike;
    match strike {
        Strike::Hit { damage, .. } => damage,
        Strike::Missed => panic!("a sure hit missed"),
    }
}

#[test]
fn a_bonus_against_a_kind_of_foe_applies_only_to_that_kind() {
    let player = || Plain::new(100, 20.0).against(FoeKind::Robot, 50, 0);
    assert_eq!(hit(player(), Plain::new(100, 1.0).kind(FoeKind::Robot)), 30);
    assert_eq!(hit(player(), Plain::new(100, 1.0).kind(FoeKind::Human)), 20);
}

#[test]
fn a_penalty_cannot_take_the_damage_below_zero() {
    let player = Plain::new(100, 20.0).against(FoeKind::Robot, -300, 0);
    let damage = hit(player, Plain::new(100, 1.0).kind(FoeKind::Robot));
    assert!(damage <= 1, "at worst the hit barely lands: {damage}");
}

#[test]
fn ignoring_threshold_works_as_if_the_target_wore_less() {
    let armored = || Plain::new(100, 20.0).armor(12.0, 0.0);
    let plain = hit(Plain::new(100, 20.0), armored());
    let piercing = hit(Plain::new(100, 20.0).ignores(10.0), armored());
    assert_eq!(plain, 8);
    assert_eq!(piercing, 18);
}

#[test]
fn ignoring_more_threshold_than_there_is_adds_nothing_further() {
    let armored = || Plain::new(100, 20.0).armor(5.0, 0.0);
    assert_eq!(
        hit(Plain::new(100, 20.0).ignores(5.0), armored()),
        hit(Plain::new(100, 20.0).ignores(500.0), armored())
    );
}

#[test]
fn a_critical_bonus_against_a_kind_scales_only_the_critical_part() {
    let profile = AttackProfile {
        ap_cost: NonZeroU32::MIN,
        base_damage: Points::try_new(20.0).expect("valid"),
        crit_damage: Points::try_new(10.0).expect("valid"),
        skill: Multiplier::FULL,
        condition: Multiplier::FULL,
        bonus: Points::ZERO,
    };
    let scaled = Against {
        damage_percent: 0,
        crit_damage_percent: 50,
    }
    .applied(&profile);
    assert_eq!(scaled.base_damage, profile.base_damage);
    assert_eq!(scaled.crit_damage.get(), 15.0);
}

proptest! {
    #[test]
    fn more_threshold_ignored_never_lowers_the_damage(
        damage in 1.0f64..200.0,
        dt in 0.0f64..100.0,
        dr in 0.0f64..90.0,
        low in 0.0f64..60.0,
        extra in 0.0f64..60.0,
    ) {
        let enemy = || Plain::new(10_000, 1.0).armor(dt, dr);
        let less = hit(Plain::new(100, damage).ignores(low), enemy());
        let more = hit(Plain::new(100, damage).ignores(low + extra), enemy());
        prop_assert!(more >= less, "{more} < {less}");
    }

    #[test]
    fn a_larger_bonus_against_a_kind_never_lowers_the_damage(
        damage in 1.0f64..200.0,
        low in -100i32..100,
        extra in 0i32..100,
    ) {
        let foe = || Plain::new(10_000, 1.0).kind(FoeKind::Abomination);
        let less = hit(Plain::new(100, damage).against(FoeKind::Abomination, low, 0), foe());
        let more = hit(
            Plain::new(100, damage).against(FoeKind::Abomination, low + extra, 0),
            foe(),
        );
        prop_assert!(more >= less, "{more} < {less}");
    }
}
