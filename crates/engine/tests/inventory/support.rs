//! A small item book and builders shared by the inventory tests.

use std::num::NonZeroU32;
use std::sync::Arc;

use engine::inventory::{Inventory, ItemBook, ItemClass};
use engine::shared::{ArmorSlot, ItemId};

pub fn item(text: &str) -> ItemId {
    ItemId::try_new(text.to_owned()).expect("literal is a valid id")
}

pub fn count(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).expect("literal is not zero")
}

/// Six items: a stimpak (0.5 lb), a pistol (3 lb), two outfits (8 and 2 lb), a
/// hat (1 lb) and ammo (0.1 lb). `ghost` is not in the book.
pub fn book() -> Arc<ItemBook> {
    Arc::new(
        ItemBook::new([
            (item("stimpak"), ItemClass::Consumable, 0.5),
            (item("pistol"), ItemClass::Weapon, 3.0),
            (item("armor"), ItemClass::Armor(ArmorSlot::Body), 8.0),
            (item("shirt"), ItemClass::Armor(ArmorSlot::Clothing), 2.0),
            (item("hat"), ItemClass::Armor(ArmorSlot::Head), 1.0),
            (item("ammo"), ItemClass::Ammo, 0.1),
        ])
        .expect("the literal entries are valid"),
    )
}

pub fn pack() -> Inventory {
    Inventory::new(book())
}
