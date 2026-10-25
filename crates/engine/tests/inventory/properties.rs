//! The inventory laws, checked over random sequences of effects against a model.

use std::collections::BTreeMap;

use engine::inventory::{Inventory, Slot};
use engine::shared::{InventorySink, InventoryView, ItemId};
use proptest::prelude::*;

use crate::support::{count, item, pack};

const NAMES: [&str; 7] = [
    "stimpak", "pistol", "armor", "shirt", "hat", "ammo", "ghost",
];

#[derive(Debug, Clone)]
enum Op {
    Give(usize, u32),
    Take(usize, u32),
    Caps(i32),
    Equip(usize),
    Unequip(usize),
}

fn op() -> impl Strategy<Value = Op> {
    let name = 0..NAMES.len();
    prop_oneof![
        (name.clone(), 1_u32..5).prop_map(|(n, c)| Op::Give(n, c)),
        (name.clone(), 1_u32..5).prop_map(|(n, c)| Op::Take(n, c)),
        (-30_i32..30).prop_map(Op::Caps),
        name.prop_map(Op::Equip),
        (0..3_usize).prop_map(Op::Unequip),
    ]
}

const SLOTS: [Slot; 3] = [Slot::Weapon, Slot::Outfit, Slot::Head];

type Snapshot = (BTreeMap<ItemId, u32>, u32, Vec<Option<ItemId>>);

fn snapshot(pack: &Inventory) -> Snapshot {
    (
        pack.stacks().map(|(i, c)| (i.clone(), c.get())).collect(),
        pack.caps(),
        SLOTS
            .iter()
            .map(|slot| pack.equipped(*slot).cloned())
            .collect(),
    )
}

proptest! {
    #[test]
    fn the_pack_keeps_its_laws_and_a_refusal_changes_nothing(
        ops in proptest::collection::vec(op(), 0..60)
    ) {
        let mut pack = pack();
        // The model: plain counts, and caps that may not go negative.
        let mut model: BTreeMap<ItemId, u32> = BTreeMap::new();
        let mut caps: i64 = 0;
        for op in ops {
            let before = snapshot(&pack);
            match op {
                Op::Give(n, c) => {
                    let ok = pack.give_item(&item(NAMES[n]), count(c)).is_ok();
                    prop_assert_eq!(ok, NAMES[n] != "ghost");
                    if ok {
                        *model.entry(item(NAMES[n])).or_default() += c;
                    } else {
                        prop_assert_eq!(snapshot(&pack), before);
                    }
                }
                Op::Take(n, c) => {
                    let held = model.get(&item(NAMES[n])).copied().unwrap_or(0);
                    let ok = pack.take_item(&item(NAMES[n]), count(c)).is_ok();
                    prop_assert_eq!(ok, held >= c);
                    if ok {
                        model.insert(item(NAMES[n]), held - c);
                        model.retain(|_, held| *held > 0);
                    } else {
                        prop_assert_eq!(snapshot(&pack), before);
                    }
                }
                Op::Caps(delta) => {
                    let ok = pack.add_caps(f64::from(delta)).is_ok();
                    prop_assert_eq!(ok, caps + i64::from(delta) >= 0);
                    if ok {
                        caps += i64::from(delta);
                    } else {
                        prop_assert_eq!(snapshot(&pack), before);
                    }
                }
                Op::Equip(n) => {
                    if pack.equip(&item(NAMES[n])).is_err() {
                        prop_assert_eq!(snapshot(&pack), before);
                    }
                }
                Op::Unequip(s) => { pack.unequip(SLOTS[s]); }
            }
            prop_assert_eq!(i64::from(pack.caps()), caps);
            prop_assert_eq!(
                pack.stacks().map(|(i, c)| (i.clone(), c.get())).collect::<BTreeMap<_, _>>(),
                model.clone()
            );
            // An equipped item is always carried.
            for slot in SLOTS {
                if let Some(worn) = pack.equipped(slot) {
                    prop_assert!(pack.item_count(worn) > 0);
                }
            }
        }
    }
}
