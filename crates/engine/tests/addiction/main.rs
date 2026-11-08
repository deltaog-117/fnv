//! Integration tests for addiction: the book of risks, the addictions held and
//! withdrawal. Chems through the composed `Game` are in `tests/game/`.

use engine::addiction::{
    AddictionBook, AddictionError, AddictionKind, Addictions, KindError, WITHDRAWAL_MINUTES,
};
use engine::shared::{ItemId, Special};
use proptest::prelude::*;

fn kind(text: &str) -> AddictionKind {
    AddictionKind::try_new(text).expect("a valid kind")
}

fn item(text: &str) -> ItemId {
    ItemId::try_new(text).expect("a valid id")
}

#[test]
fn a_kind_is_trimmed_lowercased_and_checked() {
    assert_eq!(kind("  Med-X ").as_str(), "med-x");
    assert_eq!(kind("Nuka-Cola Quantum").as_str(), "nuka-cola quantum");
    assert_eq!(AddictionKind::try_new("  "), Err(KindError::Empty));
    assert_eq!(
        AddictionKind::try_new(&"a".repeat(41)),
        Err(KindError::TooLong)
    );
    assert_eq!(
        AddictionKind::try_new("a/b"),
        Err(KindError::BadCharacter('/'))
    );
}

#[test]
fn the_book_holds_the_chance_as_a_whole_percent_and_refuses_bad_ones() {
    let (jet, bad) = (item("jet"), item("bad"));
    let book = AddictionBook::new([(&jet, "jet", 20.0)]).expect("builds");
    let risk = book.risk(&jet).expect("jet can addict");
    assert_eq!((risk.kind().as_str(), risk.chance()), ("jet", 20));
    assert!(book.risk(&item("stimpak")).is_none());
    for chance in [-1.0, 100.5, f64::NAN, f64::INFINITY] {
        assert!(
            matches!(
                AddictionBook::new([(&bad, "jet", chance)]),
                Err(AddictionError::BadChance { .. })
            ),
            "{chance}"
        );
    }
    assert!(matches!(
        AddictionBook::new([(&bad, "", 10.0)]),
        Err(AddictionError::BadKind { .. })
    ));
}

#[test]
fn a_roll_at_or_below_the_chance_addicts() {
    let jet = item("jet");
    let book = AddictionBook::new([(&jet, "jet", 20.0)]).expect("builds");
    let risk = book.risk(&jet).expect("risk");
    assert!(risk.strikes(1) && risk.strikes(20));
    assert!(!risk.strikes(21) && !risk.strikes(100));
}

#[test]
fn a_new_addiction_starts_only_when_it_is_rolled() {
    let mut held = Addictions::default();
    assert!(!held.dose(&kind("jet"), 10, false));
    assert!(held.is_empty());
    assert!(held.dose(&kind("jet"), 10, true));
    assert!(held.has(&kind("jet")) && held.any());
}

#[test]
fn a_dose_while_addicted_moves_the_last_dose_and_starts_nothing_new() {
    let mut held = Addictions::from_entries([(kind("jet"), 100)]);
    assert!(!held.dose(&kind("jet"), 500, true));
    assert_eq!(held.iter().collect::<Vec<_>>(), vec![(&kind("jet"), 500)]);
    held.dose(&kind("jet"), 200, false);
    assert_eq!(
        held.iter().next().map(|(_, last)| last),
        Some(500),
        "never backwards"
    );
    assert_eq!(held.len(), 1);
}

#[test]
fn an_addiction_wears_off_after_three_days_without_a_dose() {
    let mut held = Addictions::from_entries([(kind("jet"), 0), (kind("buffout"), 1000)]);
    assert!(!held.expire(WITHDRAWAL_MINUTES - 1));
    assert!(held.expire(WITHDRAWAL_MINUTES));
    assert!(!held.has(&kind("jet")) && held.has(&kind("buffout")));
}

#[test]
fn curing_ends_everything_and_says_how_many() {
    let mut held = Addictions::from_entries([(kind("jet"), 0), (kind("alcohol"), 5)]);
    assert_eq!(held.cure(), 2);
    assert!(held.is_empty() && held.withdrawal().is_empty());
    assert_eq!(held.cure(), 0);
}

#[test]
fn withdrawal_takes_strength_and_agility_for_each_addiction() {
    let held = Addictions::from_entries([(kind("jet"), 0), (kind("alcohol"), 5)]);
    let tally = held.withdrawal();
    assert_eq!(tally.special(Special::Strength), -2);
    assert_eq!(tally.special(Special::Agility), -2);
    assert_eq!(tally.special(Special::Endurance), 0);
}

proptest! {
    #[test]
    fn zero_percent_never_and_a_hundred_always(roll in 1u8..=100) {
        let (a, b) = (item("a"), item("b"));
        let book = AddictionBook::new([(&a, "x", 0.0), (&b, "x", 100.0)]).expect("builds");
        prop_assert!(!book.risk(&a).expect("risk").strikes(roll));
        prop_assert!(book.risk(&b).expect("risk").strikes(roll));
    }

    #[test]
    fn a_higher_chance_never_strikes_less(chance in 0u8..=100, more in 0u8..=100, roll in 1u8..=100) {
        let (a, b) = (item("a"), item("b"));
        let high = chance.saturating_add(more).min(100);
        let book = AddictionBook::new([(&a, "x", f64::from(chance)), (&b, "x", f64::from(high))])
            .expect("builds");
        prop_assert!(!book.risk(&a).expect("risk").strikes(roll) || book.risk(&b).expect("risk").strikes(roll));
    }

    #[test]
    fn after_expiry_nothing_held_has_gone_three_days_without_a_dose(
        entries in prop::collection::btree_map("[a-z]{1,6}", 0u64..10_000, 0..6),
        now in 0u64..20_000,
    ) {
        let mut held = Addictions::from_entries(
            entries.iter().map(|(name, last)| (kind(name), *last)),
        );
        held.expire(now);
        for (_, last) in held.iter() {
            prop_assert!(last + WITHDRAWAL_MINUTES > now);
        }
    }

    #[test]
    fn more_addictions_never_mean_less_withdrawal(
        names in prop::collection::btree_set("[a-z]{1,6}", 0..6),
    ) {
        let all: Vec<_> = names.iter().map(|n| (kind(n), 0)).collect();
        let fewer = Addictions::from_entries(all.iter().take(all.len() / 2).cloned());
        let more = Addictions::from_entries(all);
        prop_assert!(more.withdrawal().special(Special::Strength) <= fewer.withdrawal().special(Special::Strength));
    }

    #[test]
    fn the_same_dose_twice_is_the_same_as_once(minute in 0u64..10_000, addicting in any::<bool>()) {
        let mut once = Addictions::default();
        once.dose(&kind("jet"), minute, addicting);
        let mut twice = once.clone();
        twice.dose(&kind("jet"), minute, addicting);
        prop_assert_eq!(once.len(), twice.len());
    }
}
