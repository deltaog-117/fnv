//! Allies: player-side fighters the computer plays, and the lead whose fall loses the
//! fight for the player's side.

use engine::combat::{CombatError, Power, Side, Status, computer_turns};
use proptest::prelude::*;

use crate::support::{Cycle, Plain, Rolls, fight};

/// The player (a 100 hit point lead), an ally and an enemy, in that order.
fn trio(ally: Plain, enemy: Plain) -> engine::combat::Combat<Plain, engine::combat::FlatHitChance> {
    let combat = fight(
        vec![
            (Side::Player, Plain::new(100, 1.0)),
            (Side::Player, ally),
            (Side::Enemy, enemy),
        ],
        100,
    );
    let lead = combat.ids().next().expect("the lead");
    combat
        .with_lead(lead)
        .expect("the lead is on the player's side")
}

#[test]
fn naming_a_lead_checks_the_side_and_the_fighter() {
    let combat = fight(
        vec![
            (Side::Player, Plain::new(10, 1.0)),
            (Side::Enemy, Plain::new(10, 1.0)),
        ],
        100,
    );
    let ids: Vec<_> = combat.ids().collect();
    assert_eq!(
        combat.clone().with_lead(ids[1]).err(),
        Some(CombatError::LeadOnTheEnemySide(1))
    );
    let lead = combat.with_lead(ids[0]).expect("the player's side");
    assert_eq!(lead.lead(), Some(ids[0]));
}

#[test]
fn the_fight_is_lost_when_the_lead_falls_even_with_an_ally_standing() {
    let mut c = fight(
        vec![
            (Side::Player, Plain::new(5, 1.0)),
            (Side::Player, Plain::new(100, 1.0)),
            (Side::Enemy, Plain::new(50, 10.0)),
        ],
        100,
    );
    let ids: Vec<_> = c.ids().collect();
    c = c.with_lead(ids[0]).expect("lead");
    // Sequences are equal, so the order is as given: lead, ally, enemy.
    c.end_turn().expect("the lead passes");
    c.end_turn().expect("the ally passes");
    let target = ids[0];
    c.attack(target, Power::Normal, &mut Rolls::new(&[1, 100]))
        .expect("the enemy shoots");
    assert!(!c.is_standing(ids[0]) && c.is_standing(ids[1]));
    assert_eq!(c.status(), Status::Won(Side::Enemy));
}

#[test]
fn without_a_lead_a_side_loses_only_when_all_of_it_has_fallen() {
    let mut c = fight(
        vec![
            (Side::Player, Plain::new(5, 1.0)),
            (Side::Player, Plain::new(100, 1.0)),
            (Side::Enemy, Plain::new(50, 10.0)),
        ],
        100,
    );
    c.end_turn().expect("pass");
    c.end_turn().expect("pass");
    let first = c.ids().next().expect("first");
    c.attack(first, Power::Normal, &mut Rolls::new(&[1, 100]))
        .expect("shoots");
    assert_eq!(c.status(), Status::Ongoing, "an ally still stands");
}

#[test]
fn a_quicker_ally_acts_before_the_lead_and_the_computer_plays_it() {
    // The ally's higher sequence puts it first; 65 action points at 20 per shot is
    // three shots, each a hit that is not critical.
    let mut c = trio(Plain::new(100, 2.0).sequence(9), Plain::new(100, 1.0));
    let ally = c.active();
    assert_ne!(Some(ally), c.lead(), "the ally is first in the order");
    let done = computer_turns(&mut c, &mut Rolls::new(&[1, 100, 1, 100, 1, 100]));
    assert_eq!(done.len(), 3);
    assert!(done.iter().all(|r| r.attacker == ally));
    assert_eq!(c.active(), c.lead().expect("a lead"));
}

#[test]
fn it_does_nothing_on_the_leads_own_turn() {
    let mut c = trio(Plain::new(100, 2.0), Plain::new(100, 1.0));
    assert_eq!(Some(c.active()), c.lead());
    assert!(computer_turns(&mut c, &mut Rolls::new(&[])).is_empty());
}

#[test]
fn the_computer_plays_allies_against_the_first_enemy_standing() {
    let mut c = trio(Plain::new(100, 2.0), Plain::new(100, 1.0));
    c.end_turn().expect("the lead passes");
    let ally = c.active();
    let done = computer_turns(&mut c, &mut Cycle::new(vec![1, 100]));
    assert!(!done.is_empty());
    assert!(done.iter().any(|r| r.attacker == ally), "the ally shot");
    for resolution in &done {
        let (a, t) = (c.side(resolution.attacker), c.side(resolution.target));
        assert_ne!(a, t, "nobody shoots their own side");
    }
    assert_eq!(
        c.active(),
        c.lead().expect("a lead"),
        "the lead's turn is next"
    );
}

#[test]
fn with_no_lead_the_computer_plays_only_the_enemy_side_like_enemy_turns() {
    let mut c = fight(
        vec![
            (Side::Player, Plain::new(100, 1.0)),
            (Side::Enemy, Plain::new(50, 5.0)),
        ],
        100,
    );
    assert!(computer_turns(&mut c, &mut Rolls::new(&[])).is_empty());
    c.end_turn().expect("pass");
    let done = computer_turns(&mut c, &mut Rolls::new(&[1, 100, 1, 100, 1, 100]));
    assert_eq!(done.len(), 3);
}

proptest! {
    /// However the dice fall, the computer's turns end, nobody shoots their own
    /// side, and the lead is never skipped: when they stop, the fight is over or it
    /// is the lead's turn.
    #[test]
    fn the_computer_always_hands_back_to_the_lead_or_ends_the_fight(
        ally_hp in 1u32..200,
        enemy_hp in 1u32..200,
        ally_damage in 1u32..20,
        enemy_damage in 1u32..20,
        rolls in prop::collection::vec(1u8..=100, 1..32),
        rounds in 1usize..12,
    ) {
        let mut c = trio(
            Plain::new(ally_hp, f64::from(ally_damage)),
            Plain::new(enemy_hp, f64::from(enemy_damage)),
        );
        let mut dice = Cycle::new(rolls);
        for _ in 0..rounds {
            if c.status() != Status::Ongoing { break; }
            let _ = c.end_turn();
            for resolution in computer_turns(&mut c, &mut dice) {
                prop_assert_ne!(c.side(resolution.attacker), c.side(resolution.target));
            }
            prop_assert!(c.status() != Status::Ongoing || c.active() == c.lead().expect("a lead"));
        }
    }

    /// The side the lead is on is lost exactly when the lead is down or the enemies
    /// all are: the status never says the player's side won while the lead is down.
    #[test]
    fn the_status_follows_the_lead(
        lead_hp in 1u32..60,
        enemy_damage in 1u32..40,
        rolls in prop::collection::vec(1u8..=100, 1..16),
    ) {
        let mut c = fight(
            vec![
                (Side::Player, Plain::new(lead_hp, 1.0)),
                (Side::Player, Plain::new(500, 1.0)),
                (Side::Enemy, Plain::new(500, f64::from(enemy_damage))),
            ],
            100,
        );
        let lead = c.ids().next().expect("lead");
        c = c.with_lead(lead).expect("lead");
        let mut dice = Cycle::new(rolls);
        for _ in 0..8 {
            if c.status() != Status::Ongoing { break; }
            let _ = c.end_turn();
            computer_turns(&mut c, &mut dice);
        }
        if !c.is_standing(lead) {
            prop_assert_eq!(c.status(), Status::Won(Side::Enemy));
        } else {
            prop_assert_ne!(c.status(), Status::Won(Side::Enemy));
        }
    }
}
