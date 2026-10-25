//! The rules of a fight, pinned with small hand-built fights.

use engine::combat::{Combat, CombatError, FighterId, FlatHitChance, Power, Side, Status, Strike};

use crate::support::{Plain, Rolls, duel, fight};

/// Ids are handed out by the fight, so tests read them back in turn order.
fn ids(combat: &Combat<Plain, FlatHitChance>) -> Vec<FighterId> {
    combat.ids().collect()
}

#[test]
fn a_fight_needs_a_fighter_on_each_side() {
    let only_players = engine::combat::Combat::new(
        vec![(Side::Player, Plain::new(10, 5.0))],
        engine::combat::Rules::standard(),
        FlatHitChance(crate::support::percent(100)),
    );
    assert!(matches!(only_players, Err(CombatError::NeedsBothSides)));
}

#[test]
fn the_first_fighter_acts_first_with_full_action_points() {
    let c = duel(Plain::new(50, 10.0), Plain::new(50, 10.0), 100);
    assert_eq!(c.active(), ids(&c)[0]);
    assert_eq!(c.ap_left(), 65);
    assert_eq!(c.round(), 1);
    assert_eq!(c.status(), Status::Ongoing);
}

#[test]
fn a_hit_spends_action_points_and_takes_off_hit_points() {
    let mut c = duel(
        Plain::new(50, 20.0),
        Plain::new(50, 1.0).armor(5.0, 0.0),
        100,
    );
    let [_, enemy] = ids(&c)[..] else {
        panic!("two fighters")
    };
    // Hit roll 1, then a critical roll that fails (no crit chance).
    let r = c
        .attack(enemy, Power::Normal, &mut Rolls::new(&[1, 100]))
        .expect("legal attack");
    assert_eq!(
        r.strike,
        Strike::Hit {
            critical: false,
            damage: 15
        }
    );
    assert_eq!((r.target_hp, r.ap_left, r.defeated), (35, 45, false));
    assert_eq!(c.hp(enemy), Some(35));
}

#[test]
fn a_miss_spends_the_action_points_and_draws_one_roll() {
    let mut c = duel(Plain::new(50, 20.0), Plain::new(50, 1.0), 50);
    let [_, enemy] = ids(&c)[..] else {
        panic!("two fighters")
    };
    let mut dice = Rolls::new(&[51, 1]);
    let r = c
        .attack(enemy, Power::Normal, &mut dice)
        .expect("legal attack");
    assert_eq!(r.strike, Strike::Missed);
    assert_eq!((r.target_hp, r.ap_left), (50, 45));
    assert_eq!(dice.left(), 1);
}

#[test]
fn a_hit_draws_two_rolls_and_a_critical_adds_its_damage() {
    let mut c = duel(
        Plain::new(50, 20.0).crit(10, 10.0),
        Plain::new(50, 1.0),
        100,
    );
    let [_, enemy] = ids(&c)[..] else {
        panic!("two fighters")
    };
    let mut dice = Rolls::new(&[10, 10]);
    let r = c
        .attack(enemy, Power::Normal, &mut dice)
        .expect("legal attack");
    assert_eq!(
        r.strike,
        Strike::Hit {
            critical: true,
            damage: 30
        }
    );
    assert_eq!(dice.left(), 0);
}

#[test]
fn a_roll_of_zero_is_clamped_so_it_cannot_force_a_hit() {
    let mut c = duel(Plain::new(50, 20.0), Plain::new(50, 1.0), 0);
    let [_, enemy] = ids(&c)[..] else {
        panic!("two fighters")
    };
    let r = c
        .attack(enemy, Power::Normal, &mut Rolls::new(&[0]))
        .expect("legal attack");
    assert_eq!(r.strike, Strike::Missed);
}

#[test]
fn a_hit_that_gets_through_costs_at_least_one_hit_point() {
    let mut c = duel(
        Plain::new(50, 1.0),
        Plain::new(50, 1.0).armor(100.0, 0.0),
        100,
    );
    let [_, enemy] = ids(&c)[..] else {
        panic!("two fighters")
    };
    let r = c
        .attack(enemy, Power::Normal, &mut Rolls::new(&[1, 100]))
        .expect("legal attack");
    assert_eq!(
        r.strike,
        Strike::Hit {
            critical: false,
            damage: 1
        }
    );
}

#[test]
fn a_boosted_attack_doubles_the_weapon_damage() {
    let mut c = duel(
        Plain::new(50, 20.0),
        Plain::new(50, 1.0).armor(5.0, 0.0),
        100,
    );
    let [_, enemy] = ids(&c)[..] else {
        panic!("two fighters")
    };
    let r = c
        .attack(enemy, Power::Boosted, &mut Rolls::new(&[1, 100]))
        .expect("legal attack");
    assert_eq!(
        r.strike,
        Strike::Hit {
            critical: false,
            damage: 35
        }
    );
}

#[test]
fn a_refused_attack_changes_nothing_and_rolls_nothing() {
    let mut c = duel(Plain::new(50, 20.0).ap(30, 20), Plain::new(50, 1.0), 100);
    let [player, enemy] = ids(&c)[..] else {
        panic!("two fighters")
    };
    let mut dice = Rolls::new(&[1, 100, 1, 100]);
    c.attack(enemy, Power::Normal, &mut dice)
        .expect("first attack is paid for");
    let before = (c.ap_left(), c.hp(enemy), dice.left());
    assert_eq!(
        c.attack(enemy, Power::Normal, &mut dice),
        Err(CombatError::NotEnoughAp {
            needed: 20,
            left: 10
        })
    );
    assert_eq!(
        c.attack(player, Power::Normal, &mut dice),
        Err(CombatError::FriendlyFire(0))
    );
    assert_eq!(
        (c.ap_left(), c.hp(enemy), dice.left()),
        before,
        "refused moves must not spend anything"
    );
}

#[test]
fn only_a_standing_enemy_in_the_fight_can_be_attacked() {
    let mut c = fight(
        vec![
            (Side::Player, Plain::new(50, 100.0)),
            (Side::Enemy, Plain::new(10, 1.0)),
            (Side::Enemy, Plain::new(10, 1.0)),
        ],
        100,
    );
    let [_, first, second] = ids(&c)[..] else {
        panic!("three fighters")
    };
    let mut dice = Rolls::new(&[1, 100, 1, 100]);
    let r = c
        .attack(first, Power::Normal, &mut dice)
        .expect("legal attack");
    assert!(r.defeated);
    assert_eq!(
        c.attack(first, Power::Normal, &mut dice),
        Err(CombatError::TargetDown(1))
    );
    assert_eq!(c.status(), Status::Ongoing);
    assert!(c.is_standing(second));
}

#[test]
fn ending_a_turn_hands_over_in_order_and_counts_the_round_on_wrap() {
    let mut c = duel(Plain::new(50, 1.0), Plain::new(50, 1.0).ap(40, 20), 100);
    let [player, enemy] = ids(&c)[..] else {
        panic!("two fighters")
    };
    c.end_turn().expect("fight is on");
    assert_eq!((c.active(), c.ap_left(), c.round()), (enemy, 40, 1));
    c.end_turn().expect("fight is on");
    assert_eq!((c.active(), c.ap_left(), c.round()), (player, 65, 2));
}

#[test]
fn a_downed_fighter_is_skipped() {
    let mut c = fight(
        vec![
            (Side::Player, Plain::new(50, 100.0)),
            (Side::Enemy, Plain::new(10, 1.0)),
            (Side::Enemy, Plain::new(10, 1.0)),
        ],
        100,
    );
    let [player, first, second] = ids(&c)[..] else {
        panic!("three fighters")
    };
    c.attack(first, Power::Normal, &mut Rolls::new(&[1, 100]))
        .expect("legal attack");
    c.end_turn().expect("fight is on");
    assert_eq!(c.active(), second, "the downed enemy is skipped");
    c.end_turn().expect("fight is on");
    assert_eq!((c.active(), c.round()), (player, 2));
}

#[test]
fn the_fight_ends_when_a_side_falls_and_nothing_can_follow() {
    let mut c = duel(Plain::new(50, 100.0), Plain::new(10, 1.0), 100);
    let [_, enemy] = ids(&c)[..] else {
        panic!("two fighters")
    };
    let r = c
        .attack(enemy, Power::Normal, &mut Rolls::new(&[1, 100]))
        .expect("legal attack");
    assert!(r.defeated);
    assert_eq!(c.status(), Status::Won(Side::Player));
    assert_eq!(
        c.attack(enemy, Power::Normal, &mut Rolls::new(&[1, 100])),
        Err(CombatError::Over)
    );
    assert_eq!(c.end_turn(), Err(CombatError::Over));
}

#[test]
fn an_unknown_target_is_refused() {
    let mut c = duel(Plain::new(50, 1.0), Plain::new(50, 1.0), 100);
    let three = fight(
        vec![
            (Side::Player, Plain::new(5, 1.0)),
            (Side::Enemy, Plain::new(5, 1.0)),
            (Side::Enemy, Plain::new(5, 1.0)),
        ],
        100,
    );
    let outsider = three.ids().nth(2).expect("three fighters");
    assert_eq!(
        c.attack(outsider, Power::Normal, &mut Rolls::new(&[])),
        Err(CombatError::NoSuchFighter(2))
    );
}
