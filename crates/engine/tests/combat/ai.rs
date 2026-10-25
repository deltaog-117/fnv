//! The enemy side's turns, and starting a fighter wounded.

use std::num::NonZeroU32;

use engine::combat::{CombatError, Side, Status, enemy_turns};
use proptest::prelude::*;

use crate::support::{Cycle, Plain, Rolls, duel, fight};

fn hp(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("tests start fighters with hit points")
}

#[test]
fn an_enemy_shoots_until_its_action_points_run_out_then_the_turn_comes_back() {
    // 65 AP at 20 per shot is three shots; every roll hits and none is critical.
    let mut c = duel(Plain::new(100, 1.0), Plain::new(50, 5.0), 100);
    c.end_turn().expect("the player passes");
    let mut dice = Rolls::new(&[1, 100, 1, 100, 1, 100]);
    let done = enemy_turns(&mut c, &mut dice);
    assert_eq!(done.len(), 3);
    assert_eq!(dice.left(), 0);
    assert_eq!(c.hp(c.ids().next().expect("player")), Some(85));
    assert_eq!(c.side(c.active()), Some(Side::Player));
    assert_eq!(c.round(), 2);
}

#[test]
fn it_does_nothing_on_the_players_turn() {
    let mut c = duel(Plain::new(100, 1.0), Plain::new(50, 5.0), 100);
    assert!(enemy_turns(&mut c, &mut Rolls::new(&[])).is_empty());
    assert_eq!(c.side(c.active()), Some(Side::Player));
}

#[test]
fn it_stops_as_soon_as_the_last_player_falls() {
    let mut c = duel(Plain::new(5, 1.0), Plain::new(50, 10.0), 100);
    c.end_turn().expect("the player passes");
    let done = enemy_turns(&mut c, &mut Rolls::new(&[1, 100]));
    assert_eq!(done.len(), 1);
    assert!(done[0].defeated);
    assert_eq!(c.status(), Status::Won(Side::Enemy));
}

#[test]
fn every_enemy_in_the_order_takes_its_turn() {
    let mut c = fight(
        vec![
            (Side::Player, Plain::new(200, 1.0)),
            (Side::Enemy, Plain::new(30, 5.0).ap(20, 20)),
            (Side::Enemy, Plain::new(30, 5.0).ap(20, 20)),
        ],
        100,
    );
    c.end_turn().expect("the player passes");
    let done = enemy_turns(&mut c, &mut Rolls::new(&[1, 100, 1, 100]));
    let attackers: Vec<_> = done.iter().map(|r| r.attacker.index()).collect();
    assert_eq!(attackers, [1, 2]);
}

#[test]
fn a_missing_roll_budget_does_not_matter_when_nothing_can_attack() {
    // The enemy cannot pay for its weapon at all, so it passes without rolling.
    let mut c = duel(Plain::new(100, 1.0), Plain::new(50, 5.0).ap(10, 20), 100);
    c.end_turn().expect("the player passes");
    assert!(enemy_turns(&mut c, &mut Rolls::new(&[])).is_empty());
    assert_eq!(c.side(c.active()), Some(Side::Player));
}

#[test]
fn a_fighter_can_start_wounded_but_not_above_its_maximum() {
    let c = duel(Plain::new(50, 5.0), Plain::new(50, 5.0), 100);
    let [player, _] = c.ids().collect::<Vec<_>>()[..] else {
        panic!("two fighters")
    };
    let wounded = c.clone().with_hp(player, hp(12)).expect("12 of 50");
    assert_eq!(wounded.hp(player), Some(12));
    assert_eq!(
        c.with_hp(player, hp(51)).map(|_| ()),
        Err(CombatError::HpAboveMax {
            id: 0,
            hp: 51,
            max: 50
        })
    );
}

proptest! {
    #[test]
    fn the_enemy_turns_always_hand_the_turn_back_or_end_the_fight(
        rolls in proptest::collection::vec(1u8..=100, 1..40),
        player_hp in 1u32..400,
        enemies in 1usize..4,
        hit in 0u32..=100,
    ) {
        let mut fighters = vec![(Side::Player, Plain::new(player_hp, 4.0))];
        fighters.extend((0..enemies).map(|_| (Side::Enemy, Plain::new(60, 9.0))));
        let mut c = fight(fighters, hit);
        c.end_turn().expect("the player passes");
        let done = enemy_turns(&mut c, &mut Cycle::new(rolls));
        let over = c.status() != Status::Ongoing;
        prop_assert!(over || c.side(c.active()) == Some(Side::Player));
        prop_assert!(done.iter().all(|r| c.side(r.attacker) == Some(Side::Enemy)));
        // Nobody on the enemy side was hurt by its own turn.
        prop_assert!(c.ids().filter(|id| c.side(*id) == Some(Side::Enemy)).all(|id| c.hp(id) == Some(60)));
    }
}
