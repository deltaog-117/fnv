//! Named saves in a folder, and the automatic save beside them.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use engine::saves::{
    AUTOSAVE, MAX_NAME_LEN, NameError, SaveError, SaveFolder, SaveName, SaveSummary, Slot,
};
use proptest::prelude::*;

use super::support::{played, template};

/// A folder of its own for one test, removed afterwards.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        Self(std::env::temp_dir().join(format!(
            "fnv-folder-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }

    fn folder(&self) -> SaveFolder {
        SaveFolder::new(self.0.join("saves"))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn named(text: &str) -> Slot {
    Slot::Named(SaveName::try_new(text).expect("a valid name"))
}

#[test]
fn a_name_is_trimmed_and_may_hold_letters_digits_spaces_dashes_and_underscores() {
    assert_eq!(
        SaveName::try_new("  Before the fight_2-b ")
            .expect("valid")
            .as_str(),
        "Before the fight_2-b"
    );
}

#[test]
fn bad_names_are_refused_with_the_reason() {
    assert_eq!(SaveName::try_new(""), Err(NameError::Empty));
    assert_eq!(SaveName::try_new("   "), Err(NameError::Empty));
    assert_eq!(
        SaveName::try_new(&"a".repeat(MAX_NAME_LEN + 1)),
        Err(NameError::TooLong)
    );
    assert!(SaveName::try_new(&"a".repeat(MAX_NAME_LEN)).is_ok());
    for (text, bad) in [
        ("../escape", '.'),
        ("a/b", '/'),
        ("a\\b", '\\'),
        ("dot.json", '.'),
        ("tab\there", '\t'),
        ("café", 'é'),
        ("nul\0", '\0'),
    ] {
        assert_eq!(SaveName::try_new(text), Err(NameError::BadCharacter(bad)));
    }
    for text in [AUTOSAVE, "AutoSave", " AUTOSAVE "] {
        assert_eq!(SaveName::try_new(text), Err(NameError::Reserved));
    }
}

#[test]
fn a_slot_is_one_json_file_directly_inside_the_folder() {
    let folder = SaveFolder::new("/somewhere/saves");
    assert_eq!(
        folder.path_of(&named("Camp 1")),
        PathBuf::from("/somewhere/saves/Camp 1.json")
    );
    assert_eq!(
        folder.path_of(&Slot::Auto),
        PathBuf::from("/somewhere/saves/autosave.json")
    );
}

#[test]
fn a_missing_folder_holds_no_saves() {
    let scratch = Scratch::new();
    assert_eq!(scratch.folder().list(), Ok(Vec::new()));
    assert!(!scratch.folder().exists(&named("x")));
}

#[test]
fn saves_are_written_listed_and_read_back() {
    let scratch = Scratch::new();
    let folder = scratch.folder();
    let game = played();
    let save = game.save();
    folder.write(&named("first"), &save).expect("writes");
    folder.write(&Slot::Auto, &save).expect("writes");
    assert!(folder.exists(&named("first")));
    assert_eq!(folder.read(&named("first")), Ok(save.clone()));

    let listed = folder.list().expect("lists");
    assert_eq!(listed.len(), 2);
    let summary = SaveSummary::of(&save);
    for entry in &listed {
        assert_eq!(entry.summary.as_ref(), Ok(&summary));
    }
    let slots: Vec<&Slot> = listed.iter().map(|e| &e.slot).collect();
    assert!(slots.contains(&&Slot::Auto) && slots.contains(&&named("first")));
}

#[test]
fn the_summary_reads_the_clock_the_place_and_the_experience() {
    let game = played();
    let save = game.save();
    let summary = SaveSummary::of(&save);
    assert_eq!(summary.day, game.clock().day());
    assert_eq!(u64::from(summary.hour), u64::from(game.clock().hour()));
    assert_eq!(summary.place.as_ref(), game.at());
    assert_eq!(summary.xp, game.character().xp());
}

#[test]
fn the_most_recently_written_save_comes_first() {
    let scratch = Scratch::new();
    let folder = scratch.folder();
    let save = template().save();
    folder.write(&named("old"), &save).expect("writes");
    std::thread::sleep(std::time::Duration::from_millis(50));
    folder.write(&named("new"), &save).expect("writes");
    let order: Vec<String> = folder
        .list()
        .expect("lists")
        .iter()
        .map(|entry| entry.slot.label())
        .collect();
    assert_eq!(order, ["new", "old"]);
}

#[test]
fn a_damaged_file_is_listed_with_its_reason_and_strangers_are_left_out() {
    let scratch = Scratch::new();
    let folder = scratch.folder();
    folder
        .write(&named("good"), &template().save())
        .expect("writes");
    let dir = folder.dir();
    std::fs::write(dir.join("broken.json"), "not json at all").expect("writes");
    std::fs::write(dir.join("notes.txt"), "hello").expect("writes");
    std::fs::write(dir.join("bad.name.json"), "{}").expect("writes");
    std::fs::write(dir.join(" padded .json"), "{}").expect("writes");
    let listed = folder.list().expect("lists");
    let mut labels: Vec<String> = listed.iter().map(|e| e.slot.label()).collect();
    labels.sort();
    assert_eq!(labels, ["broken", "good"]);
    let broken = listed
        .iter()
        .find(|entry| entry.slot == named("broken"))
        .expect("listed");
    assert!(broken.summary.is_err());
}

#[test]
fn reading_a_slot_with_nothing_in_it_is_an_io_error() {
    let scratch = Scratch::new();
    assert!(matches!(
        scratch.folder().read(&named("nothing")),
        Err(SaveError::Io(_))
    ));
}

#[test]
fn writing_over_a_save_replaces_it_and_leaves_the_others() {
    let scratch = Scratch::new();
    let folder = scratch.folder();
    let (first, second) = (template().save(), played().save());
    folder.write(&named("a"), &first).expect("writes");
    folder.write(&named("b"), &first).expect("writes");
    folder.write(&named("a"), &second).expect("writes");
    assert_eq!(folder.read(&named("a")), Ok(second));
    assert_eq!(folder.read(&named("b")), Ok(first));
    assert_eq!(folder.list().expect("lists").len(), 2);
}

proptest! {
    /// Whatever is typed, a name that is accepted is safe as a file name: it stays
    /// inside the folder, has no separators and is never the automatic save's.
    #[test]
    fn an_accepted_name_always_stays_inside_the_folder(text in ".{0,60}") {
        if let Ok(name) = SaveName::try_new(&text) {
            let folder = SaveFolder::new("/base/saves");
            let path = folder.path_of(&Slot::Named(name.clone()));
            prop_assert_eq!(path.parent(), Some(std::path::Path::new("/base/saves")));
            prop_assert!(!name.as_str().contains(['/', '\\', '.', '\0']));
            prop_assert!(!name.as_str().eq_ignore_ascii_case(AUTOSAVE));
            prop_assert!(name.as_str().chars().count() <= MAX_NAME_LEN);
            prop_assert_eq!(name.as_str(), name.as_str().trim_matches(' '));
            // A name survives being read back from its own file name.
            prop_assert_eq!(SaveName::try_new(name.as_str()), Ok(name));
        }
    }
}
