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
