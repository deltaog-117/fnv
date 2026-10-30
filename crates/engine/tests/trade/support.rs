//! A small market: one shopkeeper at one place, three items.

use std::num::NonZeroU32;
use std::sync::Arc;

use engine::character::{Character, CharacterRules, SpecialSet};
use engine::game::Game;
use engine::inventory::{ItemBook, ItemClass};
use engine::quests::QuestBook;
use engine::shared::{
    InventorySink, ItemId, LocationId, NpcId, Quest, Special, SpecialLevel, Vendor, VendorId,
};
use engine::trade::VendorBook;
use engine::world::WorldBook;
use serde_json::json;

pub fn item(text: &str) -> ItemId {
    ItemId::try_new(text.to_owned()).expect("literal is a valid id")
}

pub fn vendor(text: &str) -> VendorId {
    VendorId::try_new(text.to_owned()).expect("literal is a valid id")
}

pub fn npc(text: &str) -> NpcId {
    NpcId::try_new(text.to_owned()).expect("literal is a valid id")
}

pub fn place(text: &str) -> LocationId {
    LocationId::try_new(text.to_owned()).expect("literal is a valid id")
}

pub fn count(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).expect("the literal is not zero")
}

/// The items: ammo worth 10 caps, a pistol worth 100 and a hat worth 40.
pub fn items() -> Arc<ItemBook> {
    let book = ItemBook::new([
        (item("ammo"), ItemClass::Ammo, 0.1),
        (item("pistol"), ItemClass::Weapon, 3.0),
        (
            item("hat"),
            ItemClass::Armor(engine::shared::ArmorSlot::Head),
            1.0,
        ),
    ])
    .expect("the literal entries are valid")
    .with_values([
        (item("ammo"), 10.0),
        (item("pistol"), 100.0),
        (item("hat"), 40.0),
    ])
    .expect("the literal values are valid");
    Arc::new(book)
}

pub fn record(value: serde_json::Value) -> Vendor {
    serde_json::from_value(value).expect("the literal is a valid vendor record")
}

/// The shopkeeper has 20 ammo and one pistol, and 300 caps.
pub fn shopkeeper() -> Vendor {
    record(json!({
        "id": "shopkeeper", "name": "The shop", "npc": "keeper", "place": "town", "caps": 300,
        "stock": [{"item": "ammo", "count": 20}, {"item": "pistol", "count": 1}],
    }))
}

pub fn game_with(character: Character) -> Game {
    let quests = QuestBook::new(std::iter::empty::<&Quest>()).expect("no quests is a valid book");
    let keeper = npc("keeper");
    Game::with_world(Arc::new(quests), Arc::new(WorldBook::new([&keeper])))
        .with_items(items())
        .with_character(character)
        .with_vendors(Arc::new(
            VendorBook::new([&shopkeeper()]).expect("the literal vendor is valid"),
        ))
        .expect("every stocked item has a value")
}

/// A character whose Barter is `2 + 2 * charisma + 1`, with Luck 1.
pub fn character(charisma: u32) -> Character {
    let low = SpecialSet::uniform(SpecialLevel::try_new(1).expect("legal"));
    let level = SpecialLevel::try_new(charisma).expect("legal");
    Character::new(
        low.with(Special::Charisma, level),
        CharacterRules::standard(),
    )
}

/// A game where the player starts with `caps`.
pub fn game(charisma: u32, caps: u32) -> Game {
    let mut game = game_with(character(charisma));
    game.add_caps(f64::from(caps)).expect("caps can be added");
    game
}
