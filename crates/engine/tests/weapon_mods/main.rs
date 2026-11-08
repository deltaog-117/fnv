//! Integration tests for weapon mods against the real data: which mod fits which
//! weapon, what they change together, and fitting them to a carried weapon through
//! the composed `Game`.

use std::num::NonZeroU32;
use std::path::Path;
use std::sync::Arc;

use engine::game::Game;
use engine::inventory::{InventoryError, ItemBook};
use engine::quests::QuestBook;
use engine::shared::{GameData, InventorySink, InventoryView, ItemId, Quest, Scope};
use engine::weapon_mods::{FitError, WeaponModBook};

fn id(text: &str) -> ItemId {
    ItemId::try_new(text.to_owned()).expect("literal is a valid id")
}

fn one() -> NonZeroU32 {
    NonZeroU32::MIN
}

fn data() -> GameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/engine");
    GameData::load(&dir, &Scope::Everywhere).expect("the engine data loads")
}

fn book() -> WeaponModBook {
    WeaponModBook::from_data(&data()).expect("the weapon mods build")
}

fn game() -> Game {
    let data = data();
    let quests = QuestBook::new(std::iter::empty::<&Quest>()).expect("no quests is a valid book");
    Game::new(Arc::new(quests))
        .with_items(Arc::new(ItemBook::from_data(&data).expect("items build")))
        .with_weapon_mods(Arc::new(
            WeaponModBook::from_data(&data).expect("the weapon mods build"),
        ))
        .with_arsenal(Arc::new(
            engine::game::Arsenal::from_data(&data).expect("the arsenal builds"),
        ))
}

#[test]
fn the_book_holds_every_mod_in_the_data() {
    assert_eq!(book().len(), data().weapon_mods.len());
}

#[test]
fn a_mod_fits_only_the_weapons_its_record_names() {
    let book = book();
    let barrel = id("357_magnum_revolver_long_barrel");
    assert!(book.fits(&barrel, &id("357_magnum_revolver")));
    assert!(!book.fits(&barrel, &id("10mm_pistol")));
    assert!(!book.fits(&id("no_such_mod"), &id("10mm_pistol")));
    assert!(book.mods_for(&id("357_magnum_revolver")).contains(&&barrel));
}

#[test]
fn fitted_mods_add_up_and_a_plain_weapon_changes_nothing() {
    let book = book();
    assert!(book.fitting(&[]).is_plain());
    let both = book.fitting(&[
        id("10mm_pistol_extended_mags"),
        id("10mm_pistol_laser_sight"),
    ]);
    assert_eq!(both.clip(), 4);
    assert!((both.spread() + 0.4).abs() < 1e-9);
    let reversed = book.fitting(&[
        id("10mm_pistol_laser_sight"),
        id("10mm_pistol_extended_mags"),
    ]);
    assert_eq!(
        both, reversed,
        "the order they were fitted in does not matter"
    );
}

#[test]
fn a_mod_the_game_cannot_act_on_changes_nothing() {
    let book = book();
    let scope = id("44_revolver_scope");
    assert!(book.contains(&scope));
    assert!(book.fitting(&[scope]).is_plain());
}

#[test]
fn condition_when_new_never_falls_below_the_weapons_own() {
    let book = book();
    let base = NonZeroU32::new(300).expect("not zero");
    let raised = book
        .fitting(&[id("357_revolver_hd_cylinder")])
        .condition_max(base);
    assert_eq!(raised.get(), 400);
    assert_eq!(book.fitting(&[]).condition_max(base), base);
}

#[test]
fn a_mod_leaves_the_pack_when_fitted_and_returns_when_taken_off() {
    let mut game = game();
    let (weapon, barrel) = (
        id("357_magnum_revolver"),
        id("357_magnum_revolver_long_barrel"),
    );
    game.give_item(&weapon, one()).expect("in the book");
    game.give_item(&barrel, one()).expect("in the book");
    let copy = game.inventory().copies_of(&weapon).next().expect("carried");

    game.fit_mod(copy, &barrel).expect("fits");
    assert_eq!(game.fitted_mods(copy), std::slice::from_ref(&barrel));
    assert_eq!(game.inventory().item_count(&barrel), 0, "left the pack");
    assert!(game.fitting(copy).damage() > 0.0);

    game.remove_mod(copy, &barrel).expect("comes off");
    assert!(game.fitted_mods(copy).is_empty());
    assert_eq!(game.inventory().item_count(&barrel), 1, "back in the pack");
    assert!(game.fitting(copy).is_plain());
}

#[test]
fn a_mod_for_another_weapon_is_refused_and_stays_in_the_pack() {
    let mut game = game();
    let (weapon, wrong) = (id("357_magnum_revolver"), id("10mm_pistol_laser_sight"));
    game.give_item(&weapon, one()).expect("in the book");
    game.give_item(&wrong, one()).expect("in the book");
    let copy = game.inventory().copies_of(&weapon).next().expect("carried");
    assert!(matches!(
        game.fit_mod(copy, &wrong),
        Err(FitError::DoesNotFit { .. })
    ));
    assert_eq!(game.inventory().item_count(&wrong), 1);
}

#[test]
fn the_same_mod_is_never_fitted_twice() {
    let mut game = game();
    let (weapon, barrel) = (
        id("357_magnum_revolver"),
        id("357_magnum_revolver_long_barrel"),
    );
    game.give_item(&weapon, one()).expect("in the book");
    game.give_item(&barrel, NonZeroU32::new(2).expect("not zero"))
        .expect("in the book");
    let copy = game.inventory().copies_of(&weapon).next().expect("carried");
    game.fit_mod(copy, &barrel).expect("fits");
    assert!(matches!(
        game.fit_mod(copy, &barrel),
        Err(FitError::Pack(InventoryError::AlreadyFitted { .. }))
    ));
    assert_eq!(game.inventory().item_count(&barrel), 1, "not spent twice");
}

#[test]
fn a_mod_that_is_not_carried_cannot_be_fitted() {
    let mut game = game();
    let weapon = id("357_magnum_revolver");
    game.give_item(&weapon, one()).expect("in the book");
    let copy = game.inventory().copies_of(&weapon).next().expect("carried");
    assert!(matches!(
        game.fit_mod(copy, &id("357_magnum_revolver_long_barrel")),
        Err(FitError::Pack(InventoryError::NotHeld(_)))
    ));
}
