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
fn the_book_holds_every_weapon_armor_ammo_consumable_mod_and_miscellany() {
    let data = data();
    let book = ItemBook::from_data(&data).expect("weights are valid and ids are unique");
    let expected = data.weapons.len()
        + data.armor.len()
        + data.ammo.len()
        + data.consumables.len()
        + data.weapon_mods.len()
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
        modeled, 170,
        "uses.json lists the heals, boosts, meals, drinks and the poison items"
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
    // What cures or hurts a limb, or needs vision, stealth or a damage type the game
    // does not have, stays unlisted.
    for unmodeled in [
        "cloud_kiss",
        "atomic_cocktail",
        "fire_ant_nectar",
        "nuka_cola_quartz",
    ] {
        assert_eq!(book.item_use(&item(unmodeled)), None, "{unmodeled}");
    }
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
        let mentions = text.contains("Radiation") || text.contains("RAD") || text.contains("Rads");
        assert_eq!(boost.rads != 0, mentions, "{}: {text}", consumable.id);
        if boost.rads != 0 {
            with_rads += 1;
            let removes = ["-50 RAD", "-10 Radiation", "-5 Radiation"]
                .iter()
                .any(|phrase| text.contains(phrase));
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

/// Every timed change a use makes, and every heal it gives, is a number the wiki
/// states: an attribute, resistance or action-point boost names an amount and a
/// length that one of the item's parsed effects has, and a heal is the total of the
/// item's hit-point effects.
#[test]
fn boosts_and_heals_in_a_use_match_the_wiki_numbers() {
    const HIT_POINTS: [&str; 3] = ["HP", "Hit Point", "Hit Points"];
    // Authored from the wiki's words before this check existed, where the import
    // parsed the effect into another shape (a resistance with no length, a heal
    // written as "+60 HP" without a duration, or as the range "+72-180 Hit Points").
    const READ_FROM_TEXT: [&str; 5] = [
        "rad_x",
        "super_stimpak",
        "psycho",
        "auto_inject_super_stimpak",
        // "+85 Poison Resistance (30s)": the import parsed no effect from it.
        "snakebite_tourniquet",
    ];
    let data = data();
    let mut checked = 0;
    for consumable in data.consumables.iter() {
        let Some(ItemUse::Boost(boost)) = &consumable.on_use else {
            continue;
        };
        if READ_FROM_TEXT.contains(&consumable.id.as_str()) {
            continue;
        }
        for effect in &boost.effects {
            let minutes = effect.minutes.get();
            let found = consumable.effects.iter().any(|parsed| {
                let seconds = parsed.duration_s.unwrap_or(0.0);
                (parsed.amount - f64::from(effect.amount.get())).abs() < f64::EPSILON
                    && (seconds / 60.0).ceil().max(1.0) == f64::from(minutes)
            });
            assert!(found, "{}: no wiki effect gives {effect:?}", consumable.id);
            checked += 1;
        }
        if let Some(heal) = boost.heal {
            let total: f64 = consumable
                .effects
                .iter()
                .filter(|parsed| HIT_POINTS.contains(&parsed.stat.as_str()))
                .map(|parsed| {
                    if parsed.per_second {
                        parsed.amount * parsed.duration_s.unwrap_or(1.0)
                    } else {
                        parsed.amount
                    }
                })
                .sum();
            assert!(
                (total - f64::from(heal.get())).abs() < f64::EPSILON,
                "{}: heals {heal} but the wiki totals {total}",
                consumable.id
            );
        }
    }
    assert!(checked >= 100, "only {checked} boost effects were checked");
}

/// The damage-over-time items turn the wiki's hit points per second into the total
/// poison they put in the blood, before resistance.
#[test]
fn damage_over_time_in_a_use_is_the_wiki_total() {
    let data = data();
    for id in ["dark_datura", "mother_darkness", "silver_sting", "tremble"] {
        let consumable = data
            .consumables
            .iter()
            .find(|c| c.id.as_str() == id)
            .expect("the item exists");
        let Some(ItemUse::Boost(boost)) = &consumable.on_use else {
            panic!("{id} is a boost item");
        };
        let total: f64 = consumable
            .effects
            .iter()
            .filter(|parsed| parsed.stat == "Hit Points" && parsed.per_second)
            .map(|parsed| -parsed.amount * parsed.duration_s.unwrap_or(1.0))
            .sum();
        assert!(
            (total - f64::from(boost.poison)).abs() < f64::EPSILON,
            "{id}: poisons {} but the wiki totals {total}",
            boost.poison
        );
    }
}

/// An item works by itself exactly when the wiki's text says "Automatic use", and
/// at the health the text names.
#[test]
fn the_items_that_work_by_themselves_match_the_wiki_text() {
    use engine::shared::AutoUse;

    let data = data();
    let mut found = Vec::new();
    for consumable in data.consumables.iter() {
        let said = consumable.effects_text.contains("Automatic use");
        assert_eq!(
            consumable.auto_use.is_some(),
            said && consumable.on_use.is_some(),
            "{}: {}",
            consumable.id,
            consumable.effects_text
        );
        let Some(when) = consumable.auto_use else {
            continue;
        };
        if let AutoUse::HealthAtOrBelow(limit) = when {
            let words = format!("Automatic use at {}% health", limit.get());
            assert!(
                consumable.effects_text.contains(&words),
                "{}",
                consumable.id
            );
        }
        found.push(consumable.id.as_str());
    }
    assert_eq!(
        found,
        [
            "auto_inject_stimpak",
            "auto_inject_super_stimpak",
            "snakebite_tourniquet"
        ]
    );
}

#[test]
fn an_item_that_works_by_itself_must_have_a_use_and_be_a_consumable() {
    use engine::inventory::{InventoryError, ItemClass};
    use engine::shared::AutoUse;

    let book = ItemBook::new([
        (item("tonic"), ItemClass::Consumable, 0.0),
        (item("club"), ItemClass::Weapon, 1.0),
    ])
    .expect("the book builds");
    let by_itself = |name: &str| [(item(name), AutoUse::Poisoned)];
    assert!(matches!(
        ItemBook::new([(item("tonic"), ItemClass::Consumable, 0.0)])
            .expect("the book builds")
            .with_autos(by_itself("tonic")),
        Err(InventoryError::NoEffect(_))
    ));
    assert!(matches!(
        book.with_autos(by_itself("club")),
        Err(InventoryError::NotConsumable(_))
    ));
    assert!(matches!(
        ItemBook::default().with_autos(by_itself("ghost")),
        Err(InventoryError::Unknown(_))
    ));
}
