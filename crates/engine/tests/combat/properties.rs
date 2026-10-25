//! The laws of combat, checked over random numbers and random fights.

use engine::combat::{
    Combat, FighterId, FlatHitChance, Points, Power, Rules, Side, Status, damage_after_armor,
};
use proptest::prelude::*;

use crate::support::{Cycle, Plain, percent};

fn points(value: f64) -> Points {
    Points::try_new(value).expect("valid points")
}

proptest! {
    #[test]
    fn armor_never_adds_damage_and_never_blocks_a_fifth(
        damage in 0.0..2000.0f64,
        dt in 0.0..500.0f64,
        dr in 0.0..200.0f64,
    ) {
        let rules = Rules::standard();
        let through = damage_after_armor(&rules, damage, points(dt), points(dr));
        prop_assert!(through <= damage + 1e-9, "armor must not add damage");
        prop_assert!(through >= damage * rules.min_damage_fraction() - 1e-9);
        prop_assert!(through >= 0.0);
    }

    #[test]
    fn more_armor_never_lets_more_through(
        damage in 0.0..2000.0f64,
        (dt_low, dt_high) in (0.0..500.0f64, 0.0..500.0f64).prop_map(|(a, b)| (a.min(b), a.max(b))),
        (dr_low, dr_high) in (0.0..100.0f64, 0.0..100.0f64).prop_map(|(a, b)| (a.min(b), a.max(b))),
    ) {
        let rules = Rules::standard();
        let loose = damage_after_armor(&rules, damage, points(dt_low), points(dr_low));
        let tight = damage_after_armor(&rules, damage, points(dt_high), points(dr_high));
        prop_assert!(tight <= loose + 1e-9);
    }

    #[test]
    fn resistance_past_the_cap_changes_nothing(
        damage in 0.0..2000.0f64,
        dt in 0.0..100.0f64,
        extra in 0.0..15.0f64,
    ) {
        let rules = Rules::standard();
        let at_cap = damage_after_armor(&rules, damage, points(dt), points(rules.dr_cap()));
        let past = damage_after_armor(&rules, damage, points(dt), points(rules.dr_cap() + extra));
        prop_assert_eq!(at_cap, past);
    }
}

/// One random fighter: hit points, action points, armor, crit chance and weapon.
fn plain() -> impl Strategy<Value = Plain> {
    (
        1u32..200,
        0u32..100,
        0.0..50.0f64,
        0.0..100.0f64,
        0u32..=100,
        0.0..60.0f64,
        1u32..40,
    )
        .prop_map(|(hp, ap, dt, dr, crit, damage, cost)| {
            Plain::new(hp, damage)
                .armor(dt, dr)
                .crit(crit, damage / 2.0)
                .ap(ap, cost)
        })
}

/// Random fighters with a player first and an enemy second, so both sides exist.
fn roster() -> impl Strategy<Value = Vec<(Side, Plain)>> {
    (
        plain(),
        plain(),
        prop::collection::vec((any::<bool>(), plain()), 0..4),
    )
        .prop_map(|(first, second, rest)| {
            let mut all = vec![(Side::Player, first), (Side::Enemy, second)];
            all.extend(
                rest.into_iter()
                    .map(|(p, f)| (if p { Side::Player } else { Side::Enemy }, f)),
            );
            all
        })
}

type Fight = Combat<Plain, FlatHitChance>;

fn hps(c: &Fight) -> Vec<u32> {
    c.ids().map(|i| c.hp(i).expect("own id")).collect()
}

fn snapshot(c: &Fight) -> (Vec<u32>, u32, FighterId, u32, Status) {
    (hps(c), c.ap_left(), c.active(), c.round(), c.status())
}

proptest! {
    /// Whatever is tried, hit points only go down and stay within the maximum,
    /// action points never exceed the active fighter's maximum, a refused move
    /// changes nothing, and the fight is won exactly when one side is gone.
    #[test]
    fn random_play_keeps_the_fight_consistent(
        roster in roster(),
        hit in 0u32..=100,
        rolls in prop::collection::vec(any::<u8>(), 1..64),
        moves in prop::collection::vec((any::<bool>(), 0usize..6, any::<bool>()), 0..80),
    ) {
        let mut c: Fight = Combat::new(roster, Rules::standard(), FlatHitChance(percent(hit)))
            .expect("roster has both sides");
        let ids: Vec<FighterId> = c.ids().collect();
        let max: Vec<u32> = ids.iter().map(|i| c.combatant(*i).expect("own id").hp).collect();
        let mut dice = Cycle::new(rolls);

        for (attack, target, boosted) in moves {
            let before = snapshot(&c);
            let result = if attack {
                let power = if boosted { Power::Boosted } else { Power::Normal };
                c.attack(ids[target % ids.len()], power, &mut dice).map(|_| ())
            } else {
                c.end_turn()
            };
            if result.is_err() {
                prop_assert_eq!(snapshot(&c), before.clone(), "a refused move must change nothing");
            }
            for (i, id) in ids.iter().enumerate() {
                let hp = c.hp(*id).expect("own id");
                prop_assert!(hp <= max[i]);
                prop_assert!(hp <= before.0[i], "hit points never rise");
            }
            let active_max = c.combatant(c.active()).expect("own id").ap;
            prop_assert!(c.ap_left() <= active_max);
            if c.status() == Status::Ongoing {
                prop_assert!(c.is_standing(c.active()), "a downed fighter never has the turn");
            }
            let standing = |side| ids.iter().any(|i| c.side(*i) == Some(side) && c.is_standing(*i));
            let expected = match (standing(Side::Player), standing(Side::Enemy)) {
                (true, false) => Status::Won(Side::Player),
                (false, true) => Status::Won(Side::Enemy),
                _ => Status::Ongoing,
            };
            prop_assert_eq!(c.status(), expected);
        }
    }

    /// With every attack landing for at least 1, a fight where each fighter can
    /// afford its weapon always ends: hit points are finite and every hit takes some.
    #[test]
    fn a_fight_of_sure_hits_always_ends(
        hps in prop::collection::vec(1u32..200, 2..6),
        damage in 1.0..60.0f64,
        cost in 1u32..40,
    ) {
        let fighters: Vec<(Side, Plain)> = hps
            .iter()
            .enumerate()
            .map(|(i, hp)| {
                let side = if i % 2 == 0 { Side::Player } else { Side::Enemy };
                (side, Plain::new(*hp, damage).ap(cost, cost))
            })
            .collect();
        let mut c: Fight = Combat::new(fighters, Rules::standard(), FlatHitChance(percent(100)))
            .expect("alternating sides gives both");
        let mut dice = Cycle::new(vec![1, 100]);
        let ids: Vec<FighterId> = c.ids().collect();

        for _ in 0..10_000 {
            if c.status() != Status::Ongoing {
                break;
            }
            let me = c.side(c.active()).expect("own id");
            let foe = ids.iter().find(|i| c.side(**i) != Some(me) && c.is_standing(**i));
            let foe = *foe.expect("an ongoing fight has a foe standing");
            if c.attack(foe, Power::Normal, &mut dice).is_err() {
                c.end_turn().expect("the fight is on");
            }
        }
        prop_assert!(matches!(c.status(), Status::Won(_)));
    }
}
