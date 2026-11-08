//! The item book against the real data in `data/engine/`.

use std::path::Path;
use std::sync::Arc;

use engine::inventory::{Inventory, ItemBook, Slot};
use engine::shared::{ArmorSlot, GameData, InventorySink, ItemUse, Scope};

use crate::support::{count, item};

fn data() -> GameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    GameData::load(&dir, &Scope::Everywhere).expect("the engine data loads")
}

#[test]
fn the_book_holds_every_weapon_armor_ammo_consumable_and_miscellany() {
    let data = data();
    let book = ItemBook::from_data(&data).expect("weights are valid and ids are unique");
    let expected = data.weapons.len()
        + data.armor.len()
        + data.ammo.len()
        + data.consumables.len()
        + data.misc.len();
    assert_eq!(book.len(), expected);
    assert!(book.contains(&item("pip_boy_3000")));
    assert!(book.contains(&item("duct_tape")));
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

#[test]
fn the_authored_uses_reach_the_book() {
    let data = data();
    let book = ItemBook::from_data(&data).expect("the book builds");
    assert_eq!(
        book.item_use(&item("stimpak")),
        Some(ItemUse::MedicineHeal(count(30)))
    );
    let modeled = data
        .consumables
        .iter()
        .filter(|c| c.on_use.is_some())
        .count();
    assert_eq!(
        modeled, 94,
        "uses.json lists the heals, boosts and irradiated meals"
    );
    for consumable in data.consumables.iter() {
        assert_eq!(book.item_use(&consumable.id), consumable.on_use);
    }
    // Healing powder was unlisted while its Perception penalty was not carried.
    let Some(ItemUse::Boost(powder)) = book.item_use(&item("healing_powder")) else {
        panic!("healing powder is a boost item now");
    };
    assert_eq!(powder.heal.map(std::num::NonZeroU32::get), Some(18));
    assert_eq!(powder.effects.len(), 1);
    // Anything that poisons or cures a limb stays unlisted.
    assert_eq!(book.item_use(&item("dark_datura")), None);
    assert_eq!(
        book.item_use(&item("doctor_s_bag")),
        None,
        "limbs are not modeled"
    );
}

/// An item changes radiation in the game exactly when the wiki's text for it
/// mentions radiation, and in the direction the text gives.
#[test]
fn radiation_in_a_use_matches_the_wiki_text() {
    let data = data();
    let mut with_rads = 0;
    for consumable in data.consumables.iter() {
        let Some(ItemUse::Boost(boost)) = &consumable.on_use else {
            continue;
        };
        let text = consumable
            .effects_text
            .split("Hardcore mode only")
            .next()
            .unwrap_or_default();
        let mentions = text.contains("Radiation") || text.contains("RAD");
        assert_eq!(boost.rads != 0, mentions, "{}: {text}", consumable.id);
        if boost.rads != 0 {
            with_rads += 1;
            let removes = text.contains("-50 RAD") || text.contains("-10 Radiation");
            assert_eq!(boost.rads < 0, removes, "{}: {text}", consumable.id);
        }
    }
    assert!(with_rads >= 50, "only {with_rads} items touch radiation");
}

#[test]
fn rad_x_radaway_and_the_super_stimpak_are_usable_now() {
    let book = ItemBook::from_data(&data()).expect("the book builds");
    let Some(ItemUse::Boost(rad_x)) = book.item_use(&item("rad_x")) else {
        panic!("Rad-X is a boost item");
    };
    assert_eq!(rad_x.effects.len(), 1);
    let Some(ItemUse::Boost(radaway)) = book.item_use(&item("radaway")) else {
        panic!("RadAway is a boost item");
    };
    assert_eq!(radaway.rads, -50);
    let Some(ItemUse::Boost(super_stimpak)) = book.item_use(&item("super_stimpak")) else {
        panic!("the super stimpak is a boost item");
    };
    assert_eq!(super_stimpak.heal.map(std::num::NonZeroU32::get), Some(60));
}
