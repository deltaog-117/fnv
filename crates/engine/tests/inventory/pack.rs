//! Each inventory rule on a small hand-built pack.

use engine::inventory::{Inventory, InventoryError, ItemBook, ItemClass, Slot};
use engine::shared::{EffectError, InventorySink, InventoryView};

use crate::support::{count, item, pack};

fn give(pack: &mut Inventory, name: &str, n: u32) {
    pack.give_item(&item(name), count(n))
        .expect("the item exists");
}

#[test]
fn an_empty_pack_holds_nothing() {
    let pack = pack();
    assert_eq!(pack.caps(), 0);
    assert_eq!(pack.item_count(&item("stimpak")), 0);
    assert_eq!(pack.stacks().count(), 0);
    assert!(pack.weight().abs() < f64::EPSILON);
}

#[test]
fn an_item_outside_the_book_is_refused() {
    let mut pack = pack();
    let error = pack
        .give_item(&item("ghost"), count(1))
        .expect_err("unknown");
    assert!(matches!(error, EffectError::Unknown { kind: "item", .. }));
    assert_eq!(pack.stacks().count(), 0);
}

#[test]
fn copies_of_an_item_share_one_stack() {
    let mut pack = pack();
    give(&mut pack, "stimpak", 2);
    give(&mut pack, "stimpak", 3);
    assert_eq!(pack.item_count(&item("stimpak")), 5);
    assert_eq!(pack.stacks().count(), 1);
}

#[test]
fn taking_more_than_is_held_is_refused_and_changes_nothing() {
    let mut pack = pack();
    give(&mut pack, "stimpak", 2);
    let error = pack
        .take_item(&item("stimpak"), count(3))
        .expect_err("too many");
    assert!(matches!(
        error,
        EffectError::NotEnoughItems {
            have: 2,
            need: 3,
            ..
        }
    ));
    assert_eq!(pack.item_count(&item("stimpak")), 2);
}

#[test]
fn taking_the_last_copy_removes_the_stack() {
    let mut pack = pack();
    give(&mut pack, "stimpak", 2);
    pack.take_item(&item("stimpak"), count(2)).expect("held");
    assert_eq!(pack.stacks().count(), 0);
}

#[test]
fn a_stack_that_would_overflow_is_refused() {
    let mut pack = pack();
    give(&mut pack, "ammo", u32::MAX);
    assert!(matches!(
        pack.give_item(&item("ammo"), count(1)),
        Err(EffectError::Refused {
            effect: "give_item",
            ..
        })
    ));
    assert_eq!(pack.item_count(&item("ammo")), u32::MAX);
}

#[test]
fn caps_go_up_and_down_but_never_below_zero() {
    let mut pack = pack();
    pack.add_caps(18.0).expect("a gain");
    pack.add_caps(-8.0).expect("affordable");
    assert_eq!(pack.caps(), 10);
    let error = pack.add_caps(-11.0).expect_err("too poor");
    assert!(matches!(error, EffectError::NotEnoughCaps { have: 10, .. }));
    assert_eq!(pack.caps(), 10);
}

#[test]
fn caps_must_be_whole_finite_and_carriable() {
    let mut pack = pack();
    assert!(matches!(
        pack.add_caps(f64::INFINITY),
        Err(EffectError::NotFinite { effect: "caps" })
    ));
    assert!(matches!(
        pack.add_caps(0.5),
        Err(EffectError::Refused { .. })
    ));
    pack.add_caps(f64::from(u32::MAX))
        .expect("the most that fits");
    assert!(matches!(
        pack.add_caps(1.0),
        Err(EffectError::Refused { .. })
    ));
    assert_eq!(pack.caps(), u32::MAX);
}

#[test]
fn each_item_goes_in_the_slot_it_fits() {
    let mut pack = pack();
    for name in ["pistol", "armor", "hat"] {
        give(&mut pack, name, 1);
    }
    assert_eq!(pack.equip(&item("pistol")), Ok(Slot::Weapon));
    assert_eq!(pack.equip(&item("armor")), Ok(Slot::Outfit));
    assert_eq!(pack.equip(&item("hat")), Ok(Slot::Head));
    assert!(pack.is_equipped(&item("hat")));
    assert_eq!(pack.equipped(Slot::Outfit), Some(&item("armor")));
}

#[test]
fn equipping_replaces_what_was_in_the_slot() {
    let mut pack = pack();
    give(&mut pack, "armor", 1);
    give(&mut pack, "shirt", 1);
    pack.equip(&item("armor")).expect("held");
    pack.equip(&item("shirt")).expect("held");
    assert!(!pack.is_equipped(&item("armor")));
    assert!(pack.is_equipped(&item("shirt")));
}

#[test]
fn only_carried_wearable_items_can_be_equipped() {
    let mut pack = pack();
    assert_eq!(
        pack.equip(&item("pistol")),
        Err(InventoryError::NotHeld(item("pistol")))
    );
    give(&mut pack, "stimpak", 1);
    assert_eq!(
        pack.equip(&item("stimpak")),
        Err(InventoryError::NotEquippable(item("stimpak")))
    );
}

#[test]
fn an_equipped_item_is_unequipped_when_its_last_copy_leaves() {
    let mut pack = pack();
    give(&mut pack, "pistol", 2);
    pack.equip(&item("pistol")).expect("held");
    pack.take_item(&item("pistol"), count(1)).expect("held");
    assert!(
        pack.is_equipped(&item("pistol")),
        "one copy is still carried"
    );
    pack.take_item(&item("pistol"), count(1)).expect("held");
    assert!(!pack.is_equipped(&item("pistol")));
    assert_eq!(pack.equipped(Slot::Weapon), None);
}

#[test]
fn unequipping_returns_the_item_and_keeps_it_carried() {
    let mut pack = pack();
    give(&mut pack, "hat", 1);
    pack.equip(&item("hat")).expect("held");
    assert_eq!(pack.unequip(Slot::Head), Some(item("hat")));
    assert_eq!(pack.unequip(Slot::Head), None);
    assert_eq!(pack.item_count(&item("hat")), 1);
}

#[test]
fn weight_is_the_sum_of_the_stacks() {
    let mut pack = pack();
    give(&mut pack, "pistol", 1);
    give(&mut pack, "stimpak", 4);
    assert!((pack.weight() - 5.0).abs() < 1e-9);
}

#[test]
fn a_book_refuses_impossible_weights_and_repeated_ids() {
    for weight in [-1.0, f64::NAN, f64::INFINITY] {
        let result = ItemBook::new([(item("stimpak"), ItemClass::Consumable, weight)]);
        assert!(
            matches!(result, Err(InventoryError::BadWeight { .. })),
            "{weight}"
        );
    }
    let result = ItemBook::new([
        (item("stimpak"), ItemClass::Consumable, 0.5),
        (item("stimpak"), ItemClass::Ammo, 0.1),
    ]);
    assert_eq!(
        result.err(),
        Some(InventoryError::DuplicateItem(item("stimpak")))
    );
}
