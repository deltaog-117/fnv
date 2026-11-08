//! Versions, the JSON form and the files on disk.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use engine::saves::{Save, SaveError, VERSION};

use super::support::{played, template};

/// A folder of its own for one test, removed afterwards.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "fnv-saves-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

#[test]
fn a_save_carries_the_current_version() {
    assert_eq!(template().save().version, VERSION);
}

#[test]
fn text_that_is_not_json_is_refused() {
    assert!(matches!(
        Save::from_json("this is not a save"),
        Err(SaveError::Format(_))
    ));
}

#[test]
fn a_save_without_a_version_is_refused() {
    assert!(matches!(
        Save::from_json("{}"),
        Err(SaveError::Format(reason)) if reason.contains("version")
    ));
}

#[test]
fn a_save_from_a_newer_game_is_refused() {
    let mut value: serde_json::Value =
        serde_json::from_str(&template().save().to_json().expect("writes")).expect("json");
    value["version"] = serde_json::json!(VERSION + 1);
    assert_eq!(
        Save::from_json(&value.to_string()),
        Err(SaveError::TooNew {
            found: u64::from(VERSION) + 1,
            supported: VERSION,
        })
    );
}

#[test]
fn a_version_that_never_existed_is_refused() {
    let mut value: serde_json::Value =
        serde_json::from_str(&template().save().to_json().expect("writes")).expect("json");
    value["version"] = serde_json::json!(0);
    assert_eq!(
        Save::from_json(&value.to_string()),
        Err(SaveError::UnknownVersion(0))
    );
}

#[test]
fn a_field_this_game_does_not_know_is_refused() {
    let mut value: serde_json::Value =
        serde_json::from_str(&template().save().to_json().expect("writes")).expect("json");
    value["cheat"] = serde_json::json!(true);
    assert!(matches!(
        Save::from_json(&value.to_string()),
        Err(SaveError::Format(_))
    ));
}

#[test]
fn an_id_that_is_not_valid_is_refused_on_reading() {
    let mut value: serde_json::Value =
        serde_json::from_str(&played().save().to_json().expect("writes")).expect("json");
    value["quests"][0]["quest"] = serde_json::json!("Not A Valid Id!");
    assert!(matches!(
        Save::from_json(&value.to_string()),
        Err(SaveError::Format(_))
    ));
}

#[test]
fn a_save_written_to_disk_reads_back_the_same() {
    let scratch = Scratch::new();
    let path = scratch.0.join("nested").join("slot.json");
    let save = played().save();
    save.write(&path).expect("the save is written");
    assert_eq!(Save::read(&path).expect("the save is read"), save);
}

#[test]
fn writing_again_replaces_the_save_and_leaves_no_temporary_file() {
    let scratch = Scratch::new();
    let path = scratch.0.join("slot.json");
    template().save().write(&path).expect("the first write");
    let second = played().save();
    second.write(&path).expect("the second write");
    assert_eq!(Save::read(&path).expect("the save is read"), second);
    let names: Vec<_> = std::fs::read_dir(&scratch.0)
        .expect("the folder exists")
        .map(|entry| entry.expect("an entry").file_name())
        .collect();
    assert_eq!(names, ["slot.json"]);
}

#[test]
fn reading_a_file_that_is_not_there_says_so() {
    let scratch = Scratch::new();
    assert!(matches!(
        Save::read(&scratch.0.join("missing.json")),
        Err(SaveError::Io(_))
    ));
}

#[test]
fn a_version_one_save_is_upgraded_with_nothing_gathered() {
    let mut value: serde_json::Value =
        serde_json::from_str(&template().save().to_json().expect("writes")).expect("json");
    let object = value.as_object_mut().expect("a save is an object");
    object.remove("gathered");
    object.insert("version".to_owned(), serde_json::json!(1));
    let upgraded = Save::from_json(&value.to_string()).expect("a version 1 save reads");
    assert_eq!(upgraded.version, VERSION);
    assert!(upgraded.gathered.is_empty());
    assert_eq!(upgraded, template().save());
    assert!(template().restore(&upgraded).is_ok());
}

#[test]
fn a_version_three_save_is_upgraded_with_no_radiation_and_no_addictions() {
    let mut value: serde_json::Value =
        serde_json::from_str(&template().save().to_json().expect("writes")).expect("json");
    let object = value.as_object_mut().expect("a save is an object");
    object.remove("rads");
    object.remove("addictions");
    object.insert("version".to_owned(), serde_json::json!(3));
    let upgraded = Save::from_json(&value.to_string()).expect("a version 3 save reads");
    assert_eq!(upgraded.version, VERSION);
    assert_eq!(upgraded.rads, 0);
    assert!(upgraded.addictions.is_empty());
    assert_eq!(upgraded, template().save());
    assert!(template().restore(&upgraded).is_ok());
}

/// Rewrites a current save as a version 2 file: weapons are counted in the pack,
/// the wear is kept per kind, and the weapon in hand is named by kind.
fn as_version_two(save: &Save) -> serde_json::Value {
    let mut value: serde_json::Value =
        serde_json::from_str(&save.to_json().expect("writes")).expect("json");
    let object = value.as_object_mut().expect("a save is an object");
    let mut wear = Vec::new();
    let pack = object["pack"].as_object_mut().expect("a pack");
    let mut kinds: std::collections::BTreeMap<String, (u64, u64)> = Default::default();
    for weapon in save.pack.weapons.iter() {
        let entry = kinds.entry(weapon.item.as_str().to_owned()).or_default();
        entry.0 += 1;
        entry.1 = u64::from(weapon.lost);
    }
    for (kind, (count, lost)) in &kinds {
        pack["items"][kind] = serde_json::json!(count);
        wear.push(serde_json::json!({"item": kind, "lost": lost}));
    }
    pack.remove("weapons");
    pack.remove("wielded");
    pack["weapon"] = serde_json::json!(
        save.pack
            .wielded
            .and_then(|id| save.pack.weapons.iter().find(|w| w.id == id))
            .map(|w| w.item.as_str())
    );
    object.insert("wear".to_owned(), serde_json::Value::Array(wear));
    object.insert("version".to_owned(), serde_json::json!(2));
    value
}

#[test]
fn a_version_two_save_gives_every_copy_the_wear_its_kind_had() {
    let game = played();
    let current = game.save();
    assert!(
        current.pack.weapons.iter().any(|w| w.lost > 0),
        "the history wore a weapon"
    );
    let upgraded = Save::from_json(&as_version_two(&current).to_string()).expect("a v2 save reads");
    assert_eq!(upgraded.version, VERSION);
    let kinds = |save: &Save| {
        let mut all: Vec<_> = save
            .pack
            .weapons
            .iter()
            .map(|w| (w.item.clone(), w.lost))
            .collect();
        all.sort();
        all
    };
    assert_eq!(kinds(&upgraded), kinds(&current));
    let loaded = template()
        .restore(&upgraded)
        .expect("the upgraded save loads");
    assert_eq!(
        loaded.inventory().equipped(engine::inventory::Slot::Weapon),
        game.inventory().equipped(engine::inventory::Slot::Weapon)
    );
    assert_eq!(loaded.save().pack.weapons.len(), current.pack.weapons.len());
}

#[test]
fn a_version_two_weapon_without_wear_loads_as_new_copies() {
    let mut value: serde_json::Value =
        serde_json::from_str(&template().save().to_json().expect("writes")).expect("json");
    let object = value.as_object_mut().expect("a save is an object");
    object["pack"]["items"]["9mm_pistol"] = serde_json::json!(2);
    object.insert("wear".to_owned(), serde_json::json!([]));
    object["pack"]
        .as_object_mut()
        .expect("a pack")
        .remove("weapons");
    object["pack"]
        .as_object_mut()
        .expect("a pack")
        .remove("wielded");
    object.insert("version".to_owned(), serde_json::json!(2));
    let upgraded = Save::from_json(&value.to_string()).expect("a v2 save reads");
    let loaded = template().restore(&upgraded).expect("loads");
    assert_eq!(
        loaded
            .inventory()
            .copies_of(&engine::shared::ItemId::try_new("9mm_pistol").expect("id"))
            .count(),
        2
    );
}
