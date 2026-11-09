//! Integration tests for the party: the laws that hold whatever order people join
//! and leave in. Willingness, perks and fights through the composed `Game` are in
//! `tests/game/companions.rs`.

use engine::companions::{PARTY_LIMIT, Party, PartyError};
use engine::shared::CompanionId;
use proptest::prelude::*;

fn who(name: &str) -> CompanionId {
    CompanionId::try_new(name.to_owned()).expect("literal is a valid id")
}

/// Five people, so a party of two is often full and the same person often repeats.
fn person() -> impl Strategy<Value = CompanionId> {
    prop::sample::select(vec!["a", "b", "c", "d", "e"]).prop_map(who)
}

#[derive(Debug, Clone)]
enum Move {
    Join(CompanionId),
    Leave(CompanionId),
}

fn moves() -> impl Strategy<Value = Vec<Move>> {
    prop::collection::vec(
        prop_oneof![
            person().prop_map(Move::Join),
            person().prop_map(Move::Leave)
        ],
        0..40,
    )
}

proptest! {
    #[test]
    fn the_party_never_exceeds_its_limit_or_holds_anyone_twice(moves in moves()) {
        let mut party = Party::default();
        for step in moves {
            let _ = match step {
                Move::Join(id) => party.join(&id),
                Move::Leave(id) => party.leave(&id),
            };
            let members = party.members();
            prop_assert!(members.len() <= PARTY_LIMIT);
            let mut sorted = members.to_vec();
            sorted.sort();
            sorted.dedup();
            prop_assert_eq!(sorted.len(), members.len());
        }
    }

    #[test]
    fn a_refused_change_leaves_the_party_as_it_was(moves in moves(), id in person()) {
        let mut party = Party::default();
        for step in moves {
            let _ = match step {
                Move::Join(id) => party.join(&id),
                Move::Leave(id) => party.leave(&id),
            };
        }
        let before = party.clone();
        if party.join(&id).is_err() {
            prop_assert_eq!(&party, &before);
        }
        let mut other = before.clone();
        if other.leave(&id).is_err() {
            prop_assert_eq!(&other, &before);
        }
    }

    #[test]
    fn leaving_puts_the_party_back_as_it_was_before_joining(moves in moves(), id in person()) {
        let mut party = Party::default();
        for step in moves {
            let _ = match step {
                Move::Join(id) => party.join(&id),
                Move::Leave(id) => party.leave(&id),
            };
        }
        let before = party.clone();
        if party.join(&id).is_ok() {
            party.leave(&id).expect("they just joined");
            prop_assert_eq!(party, before);
        }
    }
}

#[test]
fn a_third_person_is_refused_and_so_is_a_repeat() {
    let mut party = Party::default();
    party.join(&who("a")).expect("first");
    party.join(&who("b")).expect("second");
    assert_eq!(party.join(&who("c")), Err(PartyError::Full));
    assert_eq!(party.join(&who("a")), Err(PartyError::AlreadyIn(who("a"))));
    assert_eq!(party.leave(&who("c")), Err(PartyError::NotIn(who("c"))));
}

#[test]
fn a_saved_party_is_checked_like_joins() {
    assert!(Party::from_members([who("a"), who("b")]).is_ok());
    assert_eq!(
        Party::from_members([who("a"), who("b"), who("c")]),
        Err(PartyError::Full)
    );
    assert_eq!(
        Party::from_members([who("a"), who("a")]),
        Err(PartyError::AlreadyIn(who("a")))
    );
}
