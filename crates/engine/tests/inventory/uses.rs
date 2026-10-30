//! Item uses in the book, and throwing items away from the pack.

use std::collections::BTreeMap;

use engine::inventory::{Inventory, InventoryError, ItemBook, ItemClass, Slot};
use engine::shared::{InventorySink, InventoryView, ItemId, ItemUse};
use proptest::prelude::*;

use crate::support::{count, item, pack};

fn heal(hp: u32) -> ItemUse {
    ItemUse::Heal(count(hp))
}

fn stocked(items: &[(&str, u32)]) -> Inventory {
    let mut pack = pack();
    for (name, n) in items {
        pack.give_item(&item(name), count(*n)).expect("in the book");
    }
    pack
}

#[test]
fn a_consumable_can_be_given_a_use_and_nothing_else_can() {
    let unused = ItemBook::new([
        (item("stimpak"), ItemClass::Consumable, 0.5),
        (item("pistol"), ItemClass::Weapon, 3.0),
    ])
    .expect("valid entries");
    let with = |name: &str| {
        ItemBook::new([
            (item("stimpak"), ItemClass::Consumable, 0.5),
            (item("pistol"), ItemClass::Weapon, 3.0),
        ])
        .expect("valid entries")
        .with_uses([(item(name), heal(30))])
    };
    assert_eq!(unused.item_use(&item("stimpak")), None);
    let book = with("stimpak").expect("a consumable takes a use");
    assert_eq!(book.item_use(&item("stimpak")), Some(heal(30)));
    assert_eq!(
        with("pistol").unwrap_err(),
        InventoryError::NotConsumable(item("pistol"))
    );
    assert_eq!(
        with("ghost").unwrap_err(),
        InventoryError::Unknown(item("ghost"))
    );
}

#[test]
fn the_pack_reads_the_use_from_its_book() {
    let book = ItemBook::new([(item("stimpak"), ItemClass::Consumable, 0.5)])
        .and_then(|book| book.with_uses([(item("stimpak"), heal(30))]))
        .expect("valid entries");
    let pack = Inventory::new(std::sync::Arc::new(book));
    assert_eq!(pack.item_use(&item("stimpak")), Some(heal(30)));
    assert_eq!(pack.item_use(&item("ghost")), None);
}

#[test]
fn discarding_part_of_a_stack_leaves_the_rest() {
    let mut pack = stocked(&[("ammo", 10)]);
    pack.discard(&item("ammo"), count(4)).expect("ten are held");
    assert_eq!(pack.item_count(&item("ammo")), 6);
}

#[test]
fn discarding_the_whole_stack_removes_it() {
    let mut pack = stocked(&[("ammo", 3)]);
    pack.discard(&item("ammo"), count(3))
        .expect("three are held");
    assert_eq!(pack.item_count(&item("ammo")), 0);
    assert_eq!(pack.stacks().count(), 0);
}

#[test]
fn discarding_more_than_is_carried_is_refused_and_changes_nothing() {
    let mut pack = stocked(&[("ammo", 3)]);
    assert_eq!(
        pack.discard(&item("ammo"), count(4)),
        Err(InventoryError::NotEnough {
            item: item("ammo"),
            have: 3,
            need: 4
        })
    );
    assert_eq!(pack.item_count(&item("ammo")), 3);
}

#[test]
fn discarding_an_item_that_is_not_carried_is_refused() {
    assert_eq!(
        pack().discard(&item("pistol"), count(1)),
        Err(InventoryError::NotHeld(item("pistol")))
    );
}

#[test]
fn the_last_copy_of_an_equipped_item_cannot_be_thrown_away() {
    let mut pack = stocked(&[("pistol", 2)]);
    pack.equip(&item("pistol")).expect("a weapon equips");
    pack.discard(&item("pistol"), count(1))
        .expect("a spare copy can go");
    assert_eq!(
        pack.discard(&item("pistol"), count(1)),
        Err(InventoryError::Equipped(item("pistol")))
    );
    assert_eq!(pack.equipped(Slot::Weapon), Some(&item("pistol")));
    pack.unequip(Slot::Weapon);
    pack.discard(&item("pistol"), count(1))
        .expect("an unequipped copy can go");
}

const NAMES: [&str; 3] = ["stimpak", "ammo", "ghost"];

proptest! {
    #[test]
    fn discarding_matches_a_plain_count_and_a_refusal_changes_nothing(
        start in 0_u32..20,
        throws in proptest::collection::vec((0..NAMES.len(), 1_u32..8), 0..30)
    ) {
        let mut pack = pack();
        let mut model: BTreeMap<ItemId, u32> = BTreeMap::new();
        for name in &NAMES[..2] {
            if start > 0 {
                pack.give_item(&item(name), count(start)).expect("in the book");
                model.insert(item(name), start);
            }
        }
        for (which, n) in throws {
            let id = item(NAMES[which]);
            let held = model.get(&id).copied().unwrap_or(0);
            let result = pack.discard(&id, count(n));
            if held >= n {
                prop_assert_eq!(result, Ok(()));
                if held == n {
                    model.remove(&id);
                } else {
                    model.insert(id.clone(), held - n);
                }
            } else {
                prop_assert!(result.is_err());
            }
            let carried: BTreeMap<ItemId, u32> =
                pack.stacks().map(|(i, c)| (i.clone(), c.get())).collect();
            prop_assert_eq!(&carried, &model);
        }
    }
}
