//! Weapons are carried one by one: each copy has its own id and its own wear.

use std::collections::BTreeMap;

use engine::inventory::{CopyId, Inventory, InventoryError, Slot};
use engine::shared::{InventorySink, InventoryView};
use proptest::prelude::*;

use crate::support::{count, item, pack};

fn pistols(pack: &Inventory) -> Vec<CopyId> {
    pack.copies_of(&item("pistol")).collect()
}

fn with_pistols(n: u32) -> Inventory {
    let mut pack = pack();
    pack.give_item(&item("pistol"), count(n)).expect("given");
    pack
}

#[test]
fn every_weapon_given_is_its_own_new_copy() {
    let pack = with_pistols(3);
    let copies = pistols(&pack);
    assert_eq!(copies.len(), 3);
    assert_eq!(pack.item_count(&item("pistol")), 3);
    assert_eq!(pack.stacks().count(), 0, "weapons are not stacks");
    let mut ids = copies.clone();
    ids.dedup();
    assert_eq!(ids.len(), 3, "the ids differ");
    assert!(
        copies
            .iter()
            .all(|id| pack.weapon(*id).is_some_and(|c| c.lost() == 0))
    );
}

#[test]
fn wearing_one_copy_leaves_the_others_alone() {
    let mut pack = with_pistols(3);
    let copies = pistols(&pack);
    pack.set_lost(copies[1], 40).expect("held");
    let lost: Vec<u32> = copies
        .iter()
        .map(|id| pack.weapon(*id).map_or(u32::MAX, |c| c.lost()))
        .collect();
    assert_eq!(lost, [0, 40, 0]);
}

#[test]
fn equipping_a_kind_takes_the_least_worn_copy() {
    let mut pack = with_pistols(3);
    let copies = pistols(&pack);
    pack.set_lost(copies[0], 30).expect("held");
    pack.set_lost(copies[1], 10).expect("held");
    pack.set_lost(copies[2], 20).expect("held");
    assert_eq!(pack.equip(&item("pistol")), Ok(Slot::Weapon));
    assert_eq!(pack.wielded(), Some(copies[1]));
    assert_eq!(pack.equipped(Slot::Weapon), Some(&item("pistol")));
}

#[test]
fn taking_weapons_away_removes_the_most_worn_first_and_the_one_in_hand_last() {
    let mut pack = with_pistols(3);
    let copies = pistols(&pack);
    pack.set_lost(copies[0], 30).expect("held");
    pack.set_lost(copies[1], 10).expect("held");
    pack.set_lost(copies[2], 99).expect("held");
    pack.equip_copy(copies[2]).expect("held");
    pack.take_item(&item("pistol"), count(1)).expect("taken");
    assert!(pack.weapon(copies[0]).is_none(), "the most worn spare went");
    assert!(pack.weapon(copies[1]).is_some());
    assert_eq!(pack.wielded(), Some(copies[2]), "the one in hand stayed");
    pack.take_item(&item("pistol"), count(2)).expect("taken");
    assert_eq!(pack.wielded(), None, "taking the last one empties the hand");
}

#[test]
fn the_weapon_in_hand_cannot_be_thrown_away_but_a_spare_can() {
    let mut pack = with_pistols(2);
    let copies = pistols(&pack);
    pack.equip_copy(copies[0]).expect("held");
    assert_eq!(
        pack.discard_copy(copies[0]),
        Err(InventoryError::Equipped(item("pistol")))
    );
    assert_eq!(pack.discard_copy(copies[1]), Ok(()));
    assert_eq!(
        pack.discard_copy(copies[1]),
        Err(InventoryError::NoSuchCopy(copies[1]))
    );
    assert_eq!(
        pack.discard(&item("pistol"), count(1)),
        Err(InventoryError::Equipped(item("pistol")))
    );
}

#[test]
fn a_thrown_away_weapon_never_hands_its_id_to_the_next_one() {
    let mut pack = with_pistols(2);
    let gone = pistols(&pack)[1];
    pack.discard_copy(gone).expect("thrown");
    pack.give_item(&item("pistol"), count(1)).expect("given");
    assert!(
        pistols(&pack)
            .iter()
            .all(|id| *id != gone || pack.weapon(*id).is_none())
    );
    assert!(pack.weapon(gone).is_none());
}

#[test]
fn merging_removes_one_and_keeps_the_hand() {
    let mut pack = with_pistols(2);
    let copies = pistols(&pack);
    pack.equip_copy(copies[1]).expect("held");
    pack.merge_copies(copies[0], copies[1], 7).expect("merged");
    assert!(pack.weapon(copies[1]).is_none());
    assert_eq!(pack.weapon(copies[0]).map(|c| c.lost()), Some(7));
    assert_eq!(
        pack.wielded(),
        Some(copies[0]),
        "the survivor takes the hand"
    );
    assert_eq!(
        pack.merge_copies(copies[0], copies[0], 0),
        Err(InventoryError::NoSuchCopy(copies[0]))
    );
}

#[test]
fn a_saved_weapon_keeps_its_number_and_later_ones_come_after() {
    let mut pack = pack();
    let id = CopyId::from_saved(41);
    pack.restore_weapon(id, &item("pistol"), 5)
        .expect("restored");
    assert_eq!(
        pack.restore_weapon(id, &item("pistol"), 0),
        Err(InventoryError::CopyTaken(id))
    );
    assert_eq!(
        pack.restore_weapon(CopyId::from_saved(1), &item("armor"), 0),
        Err(InventoryError::Unknown(item("armor")))
    );
    pack.give_item(&item("pistol"), count(1)).expect("given");
    let newest = pistols(&pack).into_iter().max().expect("two pistols");
    assert!(newest > id);
}

#[test]
fn weapons_weigh_what_each_copy_weighs() {
    let pack = with_pistols(3);
    assert!((pack.weight() - 9.0).abs() < 1e-9);
}

#[derive(Debug, Clone)]
enum Op {
    Give(u32),
    Wear(usize, u32),
    Take(u32),
    Discard(usize),
    Equip(usize),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (1_u32..4).prop_map(Op::Give),
        (0..8_usize, 0_u32..100).prop_map(|(i, l)| Op::Wear(i, l)),
        (1_u32..4).prop_map(Op::Take),
        (0..8_usize).prop_map(Op::Discard),
        (0..8_usize).prop_map(Op::Equip),
    ]
}

proptest! {
    /// Against a plain model of (id, lost) pairs: wear stays with the copy it was
    /// written to, whatever else comes and goes, and the hand always holds a copy.
    #[test]
    fn each_copy_keeps_its_own_wear_through_any_sequence(
        ops in proptest::collection::vec(op(), 0..60)
    ) {
        let mut pack = pack();
        let mut model: BTreeMap<CopyId, u32> = BTreeMap::new();
        for op in ops {
            match op {
                Op::Give(n) => {
                    let before: Vec<CopyId> = pistols(&pack);
                    pack.give_item(&item("pistol"), count(n)).expect("given");
                    for id in pistols(&pack) {
                        if !before.contains(&id) {
                            prop_assert!(!model.contains_key(&id), "an id was reused");
                            model.insert(id, 0);
                        }
                    }
                }
                Op::Wear(i, lost) => {
                    let ids: Vec<CopyId> = model.keys().copied().collect();
                    if let Some(id) = ids.get(i) {
                        pack.set_lost(*id, lost).expect("held");
                        model.insert(*id, lost);
                    }
                }
                Op::Take(n) => {
                    let held = u32::try_from(model.len()).expect("small");
                    let result = pack.take_item(&item("pistol"), count(n));
                    prop_assert_eq!(result.is_ok(), held >= n);
                    if result.is_ok() {
                        model.retain(|id, _| pack.weapon(*id).is_some());
                    }
                }
                Op::Discard(i) => {
                    let ids: Vec<CopyId> = model.keys().copied().collect();
                    if let Some(id) = ids.get(i) {
                        let in_hand = pack.wielded() == Some(*id);
                        let result = pack.discard_copy(*id);
                        prop_assert_eq!(result.is_ok(), !in_hand);
                        if result.is_ok() {
                            model.remove(id);
                        }
                    }
                }
                Op::Equip(i) => {
                    let ids: Vec<CopyId> = model.keys().copied().collect();
                    if let Some(id) = ids.get(i) {
                        pack.equip_copy(*id).expect("held");
                    }
                }
            }
            let now: BTreeMap<CopyId, u32> = pack
                .weapons()
                .map(|(id, copy)| (id, copy.lost()))
                .collect();
            prop_assert_eq!(&now, &model);
            if let Some(hand) = pack.wielded() {
                prop_assert!(now.contains_key(&hand));
            }
        }
    }
}
