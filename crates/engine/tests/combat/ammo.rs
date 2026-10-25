//! Ammunition: the clip, the reserve and the reload.

use engine::combat::{CombatError, Power, Rounds, enemy_turns};

use crate::support::{Plain, Rolls, duel};

/// A shooter with a 3-round clip and 5 rounds in all, against a sturdy target.
fn shooter() -> Plain {
    Plain::new(50, 1.0).rounds(3, Some(5))
}

fn target() -> Plain {
    Plain::new(500, 1.0)
}

#[test]
fn a_weapon_starts_loaded_and_the_rest_is_the_reserve() {
    let c = duel(shooter(), target(), 50);
    let you = c.active();
    assert_eq!(
        c.rounds(you),
        Some(Rounds {
            loaded: 3,
            reserve: Some(2)
        })
    );
}

#[test]
fn a_pack_smaller_than_the_clip_loads_what_there_is() {
    let c = duel(Plain::new(50, 1.0).rounds(13, Some(4)), target(), 50);
    let rounds = c.rounds(c.active()).expect("a magazine");
    assert_eq!((rounds.loaded, rounds.reserve), (4, Some(0)));
}

#[test]
fn a_weapon_without_a_magazine_has_no_rounds() {
    let c = duel(Plain::new(50, 1.0), target(), 50);
    assert_eq!(c.rounds(c.active()), None);
}

#[test]
fn every_shot_uses_a_round_hit_or_miss() {
    let mut c = duel(shooter(), target(), 50);
    let (you, foe) = (c.active(), c.ids().nth(1).expect("two fighters"));
    // A hit (roll 1, then a failed critical roll) and then a miss (roll 100).
    c.attack(foe, Power::Normal, &mut Rolls::new(&[1, 100]))
        .expect("a shot");
    c.attack(foe, Power::Normal, &mut Rolls::new(&[100]))
        .expect("a shot");
    assert_eq!(c.rounds(you).expect("a magazine").loaded, 1);
}

#[test]
fn an_empty_weapon_refuses_the_shot_and_changes_nothing() {
    let mut c = duel(Plain::new(50, 1.0).rounds(1, Some(1)), target(), 50);
    let (you, foe) = (c.active(), c.ids().nth(1).expect("two fighters"));
    c.attack(foe, Power::Normal, &mut Rolls::new(&[100]))
        .expect("the one round");
    let (ap, rounds) = (c.ap_left(), c.rounds(you));
    // No dice are scripted: a refused shot must not roll.
    assert_eq!(
        c.attack(foe, Power::Normal, &mut Rolls::new(&[])),
        Err(CombatError::OutOfAmmo {
            needed: 1,
            loaded: 0
        })
    );
    assert_eq!((c.ap_left(), c.rounds(you)), (ap, rounds));
}

#[test]
fn a_reload_moves_rounds_from_the_reserve_and_costs_action_points() {
    let mut c = duel(shooter(), target(), 50);
    let (you, foe) = (c.active(), c.ids().nth(1).expect("two fighters"));
    for _ in 0..3 {
        c.attack(foe, Power::Normal, &mut Rolls::new(&[100]))
            .expect("a shot");
    }
    let ap = c.ap_left();
    let reload = c.reload().expect("rounds are left to load");
    assert_eq!((reload.added, reload.loaded), (2, 2));
    assert_eq!(c.ap_left(), ap - 5);
    assert_eq!(
        c.rounds(you),
        Some(Rounds {
            loaded: 2,
            reserve: Some(0)
        })
    );
}

#[test]
fn nothing_to_reload_when_full_empty_handed_or_out_of_rounds() {
    let mut full = duel(shooter(), target(), 50);
    assert_eq!(full.reload(), Err(CombatError::NothingToReload));

    let mut bare = duel(Plain::new(50, 1.0), target(), 50);
    assert_eq!(bare.reload(), Err(CombatError::NothingToReload));

    let mut spent = duel(Plain::new(50, 1.0).rounds(1, Some(1)), target(), 50);
    let foe = spent.ids().nth(1).expect("two fighters");
    spent
        .attack(foe, Power::Normal, &mut Rolls::new(&[100]))
        .expect("the one round");
    assert_eq!(spent.reload(), Err(CombatError::NothingToReload));
}

#[test]
fn a_reload_that_cannot_be_paid_is_refused_whole() {
    let mut c = duel(
        Plain::new(50, 1.0).rounds(2, Some(4)).ap(21, 20),
        target(),
        50,
    );
    let (you, foe) = (c.active(), c.ids().nth(1).expect("two fighters"));
    c.attack(foe, Power::Normal, &mut Rolls::new(&[100]))
        .expect("a shot");
    let rounds = c.rounds(you);
    assert_eq!(
        c.reload(),
        Err(CombatError::NotEnoughAp { needed: 5, left: 1 })
    );
    assert_eq!((c.ap_left(), c.rounds(you)), (1, rounds));
}

#[test]
fn an_endless_supply_never_runs_dry() {
    let mut c = duel(Plain::new(50, 1.0).rounds(2, None), target(), 50);
    let (you, foe) = (c.active(), c.ids().nth(1).expect("two fighters"));
    for _ in 0..2 {
        c.attack(foe, Power::Normal, &mut Rolls::new(&[100]))
            .expect("a shot");
    }
    c.reload().expect("endless rounds");
    assert_eq!(c.rounds(you).expect("a magazine").loaded, 2);
    assert_eq!(c.rounds(you).and_then(Rounds::total), None);
}

#[test]
fn an_enemy_reloads_when_empty_and_still_shoots_in_the_same_turn() {
    let mut c = duel(
        Plain::new(500, 1.0),
        Plain::new(50, 1.0).rounds(1, None).ap(65, 20),
        50,
    );
    c.end_turn().expect("the player passes");
    // Shot (20 AP), reload (5), shot (20): 20 AP are left, one short of a third.
    let log = enemy_turns(&mut c, &mut Rolls::new(&[100, 100]));
    assert_eq!(log.len(), 2);
    assert_eq!(c.active(), c.ids().next().expect("the player is first"));
}

#[test]
fn an_enemy_with_no_rounds_left_passes_the_turn() {
    let mut c = duel(
        Plain::new(500, 1.0),
        Plain::new(50, 1.0).rounds(1, Some(1)),
        50,
    );
    c.end_turn().expect("the player passes");
    let log = enemy_turns(&mut c, &mut Rolls::new(&[100]));
    assert_eq!(log.len(), 1, "one round, one shot, then nothing to shoot");
    assert_eq!(c.active(), c.ids().next().expect("the player is first"));
}

mod laws {
    use engine::combat::{Combat, FlatHitChance, Power};
    use proptest::prelude::*;

    use crate::support::{Cycle, Plain, percent};

    proptest! {
        /// Whatever is tried, the weapon never holds more than its clip, the rounds
        /// carried only ever go down (a reload moves them, a shot spends them), and
        /// a refused move changes nothing.
        #[test]
        fn rounds_are_conserved_and_bounded(
            clip in 1u32..20,
            stock in prop::option::of(0u32..60),
            hit in 0u32..=100,
            rolls in prop::collection::vec(1u8..=100, 1..32),
            moves in prop::collection::vec(0u8..3, 0..60),
        ) {
            let player = Plain::new(100, 1.0).rounds(clip, stock).ap(200, 10);
            let mut c: Combat<Plain, FlatHitChance> = Combat::new(
                vec![
                    (engine::combat::Side::Player, player),
                    (engine::combat::Side::Enemy, Plain::new(10_000, 1.0)),
                ],
                engine::combat::Rules::standard(),
                FlatHitChance(percent(hit)),
            ).expect("both sides");
            let (you, foe) = (c.active(), c.ids().nth(1).expect("two fighters"));
            let mut dice = Cycle::new(rolls);
            for step in moves {
                if c.active() != you {
                    break;
                }
                let before = (c.rounds(you), c.ap_left());
                let result = match step {
                    0 => c.attack(foe, Power::Normal, &mut dice).map(|_| ()),
                    1 => c.reload().map(|_| ()),
                    _ => break,
                };
                let rounds = c.rounds(you).expect("a magazine");
                prop_assert!(rounds.loaded <= clip);
                if result.is_err() {
                    prop_assert_eq!((c.rounds(you), c.ap_left()), before);
                }
                if let (Some(now), Some(then)) = (rounds.total(), before.0.and_then(|r| r.total())) {
                    prop_assert!(now <= then, "rounds never appear from nowhere");
                }
            }
        }
    }
}
