//! Who acts first, and running away.

use engine::combat::{CombatError, FighterId, Flight, Side, Status};
use proptest::prelude::*;

use crate::support::{Plain, Rolls, fight};

fn ids(combat: &engine::combat::Combat<Plain, engine::combat::FlatHitChance>) -> Vec<FighterId> {
    combat.ids().collect()
}

#[test]
fn the_highest_sequence_acts_first_whoever_was_given_first() {
    let c = fight(
        vec![
            (Side::Player, Plain::new(50, 5.0).sequence(6)),
            (Side::Enemy, Plain::new(50, 5.0).sequence(14)),
            (Side::Enemy, Plain::new(50, 5.0).sequence(10)),
        ],
        50,
    );
    let ids = ids(&c);
    assert_eq!(c.active(), ids[1], "the quickest foe goes first");
    assert_eq!(c.round(), 1);
}

#[test]
fn equal_sequences_keep_the_order_given() {
    let mut c = fight(
        vec![
            (Side::Player, Plain::new(50, 5.0).sequence(8)),
            (Side::Enemy, Plain::new(50, 5.0).sequence(8)),
            (Side::Enemy, Plain::new(50, 5.0).sequence(8)),
        ],
        50,
    );
    let ids = ids(&c);
    assert_eq!(c.active(), ids[0]);
    c.end_turn().expect("goes on");
    assert_eq!(c.active(), ids[1]);
    c.end_turn().expect("goes on");
    assert_eq!(c.active(), ids[2]);
}

#[test]
fn the_turns_go_round_in_sequence_and_a_new_round_starts_after_the_slowest() {
    let mut c = fight(
        vec![
            (Side::Player, Plain::new(50, 5.0).sequence(2)),
            (Side::Enemy, Plain::new(50, 5.0).sequence(12)),
            (Side::Enemy, Plain::new(50, 5.0).sequence(7)),
        ],
        50,
    );
    let ids = ids(&c);
    let mut seen = Vec::new();
    for _ in 0..6 {
        seen.push((c.active(), c.round()));
        c.end_turn().expect("goes on");
    }
    assert_eq!(
        seen,
        [
            (ids[1], 1),
            (ids[2], 1),
            (ids[0], 1),
            (ids[1], 2),
            (ids[2], 2),
            (ids[0], 2),
        ]
    );
}

#[test]
fn a_fighter_who_is_down_loses_their_turn() {
    let mut c = fight(
        vec![
            (Side::Player, Plain::new(50, 100.0).sequence(5)),
            (Side::Enemy, Plain::new(1, 1.0).sequence(9)),
            (Side::Enemy, Plain::new(50, 1.0).sequence(3)),
        ],
        100,
    );
    let ids = ids(&c);
    // The quick foe goes first and passes; the player then drops them.
    c.end_turn().expect("goes on");
    assert_eq!(c.active(), ids[0]);
    c.attack(
        ids[1],
        engine::combat::Power::Normal,
        &mut Rolls::new(&[1, 100]),
    )
    .expect("the shot lands");
    c.end_turn().expect("goes on");
    assert_eq!(c.active(), ids[2], "the fallen foe is skipped");
}

#[test]
fn running_away_ends_the_fight_on_a_good_roll() {
    let mut c = fight(
        vec![
            (Side::Player, Plain::new(50, 5.0).flee(55)),
            (Side::Enemy, Plain::new(50, 5.0)),
        ],
        50,
    );
    let mut dice = Rolls::new(&[55]);
    assert_eq!(c.flee(&mut dice), Ok(Flight::Escaped));
    assert_eq!(dice.left(), 0, "one roll is all it takes");
    assert_eq!(c.status(), Status::Fled);
    assert_eq!(c.end_turn(), Err(CombatError::Over));
    assert!(matches!(
        c.flee(&mut Rolls::new(&[])),
        Err(CombatError::Over)
    ));
}

#[test]
fn a_failed_run_costs_the_rest_of_the_turn_and_the_fight_goes_on() {
    let mut c = fight(
        vec![
            (Side::Player, Plain::new(50, 5.0).flee(55)),
            (Side::Enemy, Plain::new(50, 5.0)),
        ],
        50,
    );
    assert_eq!(c.flee(&mut Rolls::new(&[56])), Ok(Flight::Blocked));
    assert_eq!(c.status(), Status::Ongoing);
    assert_eq!(c.ap_left(), 0);
}

#[test]
fn a_fighter_who_cannot_run_never_escapes_and_one_who_always_can_always_does() {
    let mut never = fight(
        vec![
            (Side::Player, Plain::new(50, 5.0)),
            (Side::Enemy, Plain::new(50, 5.0)),
        ],
        50,
    );
    assert_eq!(never.flee(&mut Rolls::new(&[1])), Ok(Flight::Blocked));
    let mut always = fight(
        vec![
            (Side::Player, Plain::new(50, 5.0).flee(100)),
            (Side::Enemy, Plain::new(50, 5.0)),
        ],
        50,
    );
    assert_eq!(always.flee(&mut Rolls::new(&[100])), Ok(Flight::Escaped));
}

#[test]
fn an_enemy_cannot_run_and_the_refusal_rolls_nothing() {
    let mut c = fight(
        vec![
            (Side::Player, Plain::new(50, 5.0).sequence(1)),
            (Side::Enemy, Plain::new(50, 5.0).sequence(9).flee(100)),
        ],
        50,
    );
    let mut dice = Rolls::new(&[1]);
    assert_eq!(c.flee(&mut dice), Err(CombatError::OnlyThePlayerFlees));
    assert_eq!(dice.left(), 1);
    assert_eq!(c.status(), Status::Ongoing);
}

proptest! {
    /// Whatever the sequences, every fighter standing gets exactly one turn per
    /// round and the order never changes.
    #[test]
    fn turn_order_is_a_fixed_cycle(
        sequences in proptest::collection::vec(0_u32..30, 2..7),
        turns in 1_usize..40,
    ) {
        let fighters: Vec<(Side, Plain)> = sequences
            .iter()
            .enumerate()
            .map(|(i, seq)| {
                let side = if i == 0 { Side::Player } else { Side::Enemy };
                (side, Plain::new(50, 1.0).sequence(*seq))
            })
            .collect();
        let count = fighters.len();
        let mut c = fight(fighters, 50);
        let mut first_round = Vec::new();
        let mut all = Vec::new();
        for _ in 0..turns {
            all.push(c.active());
            if c.round() == 1 {
                first_round.push(c.active());
            }
            c.end_turn().expect("goes on");
        }
        prop_assert_eq!(first_round.iter().collect::<std::collections::HashSet<_>>().len(), first_round.len());
        prop_assert!(first_round.len() <= count);
        for (turn, who) in all.iter().enumerate() {
            prop_assert_eq!(*who, all[turn % count], "the cycle repeats");
        }
    }
}
