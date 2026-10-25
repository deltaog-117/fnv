//! The item book against the real data in `data/engine/`.

use std::path::Path;
use std::sync::Arc;

use engine::inventory::{Inventory, ItemBook, Slot};
use engine::shared::{ArmorSlot, GameData, InventorySink, Scope};

use crate::support::{count, item};

fn data() -> GameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    GameData::load(&dir, &Scope::Everywhere).expect("the engine data loads")
}

#[test]
fn the_book_holds_every_weapon_armor_ammo_and_consumable() {
    let data = data();
    let book = ItemBook::from_data(&data).expect("weights are valid and ids are unique");
    let expected = data.weapons.len() + data.armor.len() + data.ammo.len() + data.consumables.len();
    assert_eq!(book.len(), expected);
    assert!(book.contains(&item("pip_boy_3000")));
}

#[test]
fn every_weapon_and_piece_of_armor_equips_in_its_slot() {
    let data = data();
    let book = Arc::new(ItemBook::from_data(&data).expect("the book builds"));
    let mut pack = Inventory::new(book);
    for weapon in data.weapons.iter() {
        pack.give_item(&weapon.id, count(1)).expect("in the book");
        assert_eq!(pack.equip(&weapon.id), Ok(Slot::Weapon), "{}", weapon.name);
    }
    for armor in data.armor.iter() {
        pack.give_item(&armor.id, count(1)).expect("in the book");
        let slot = match armor.slot {
            ArmorSlot::Head => Slot::Head,
            ArmorSlot::Body | ArmorSlot::Clothing => Slot::Outfit,
        };
        assert_eq!(pack.equip(&armor.id), Ok(slot), "{}", armor.name);
    }
}
