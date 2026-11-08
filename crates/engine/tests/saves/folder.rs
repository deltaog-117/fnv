//! Named saves in a folder, and the automatic save beside them.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use engine::saves::{
    AUTOSAVE, AUTOSAVE_PREVIOUS, HISTORY_DEPTH, LEGACY_NAME, MAX_NAME_LEN, NameError, SaveError,
    SaveFolder, SaveName, SaveSummary, Slot,
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
    for text in [
        AUTOSAVE,
        "AutoSave",
        " AUTOSAVE ",
        AUTOSAVE_PREVIOUS,
        "Autosave_Previous",
    ] {
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

#[test]
fn the_previous_autosave_sits_in_the_folder_and_the_old_save_beside_it() {
    let folder = SaveFolder::new("/somewhere/saves");
    assert_eq!(
        folder.path_of(&Slot::Previous),
        PathBuf::from("/somewhere/saves/autosave_previous.json")
    );
    assert_eq!(
        folder.path_of(&Slot::Legacy),
        PathBuf::from("/somewhere/save.json")
    );
}

#[test]
fn renaming_moves_the_save_and_refuses_to_replace_another() {
    let scratch = Scratch::new();
    let folder = scratch.folder();
    let (first, second) = (template().save(), played().save());
    folder.write(&named("a"), &first).expect("writes");
    folder.write(&named("b"), &second).expect("writes");
    let (a, b, c) = (
        SaveName::try_new("a").expect("valid"),
        SaveName::try_new("b").expect("valid"),
        SaveName::try_new("c").expect("valid"),
    );
    folder.rename(&a, &c).expect("renames");
    assert!(!folder.exists(&named("a")));
    assert_eq!(folder.read(&named("c")), Ok(first.clone()));

    assert!(matches!(folder.rename(&c, &b), Err(SaveError::Io(_))));
    assert_eq!(folder.read(&named("b")), Ok(second), "b was not replaced");
    assert_eq!(folder.read(&named("c")), Ok(first), "c is where it was");
    assert!(matches!(folder.rename(&a, &c), Err(SaveError::Io(_))));
}

#[test]
fn deleting_removes_one_save_and_nothing_else() {
    let scratch = Scratch::new();
    let folder = scratch.folder();
    let save = template().save();
    folder.write(&named("a"), &save).expect("writes");
    folder.write(&named("b"), &save).expect("writes");
    folder.delete(&named("a")).expect("deletes");
    assert!(!folder.exists(&named("a")));
    assert!(folder.exists(&named("b")));
    assert!(matches!(folder.delete(&named("a")), Err(SaveError::Io(_))));
}

#[test]
fn the_automatic_save_keeps_the_one_before_it() {
    let scratch = Scratch::new();
    let folder = scratch.folder();
    let (first, second, third) = (template().save(), played().save(), template().save());
    folder.write_automatic(&first).expect("writes");
    assert!(
        !folder.exists(&Slot::Previous),
        "there was nothing to keep yet"
    );
    folder.write_automatic(&second).expect("writes");
    assert_eq!(folder.read(&Slot::Auto), Ok(second.clone()));
    assert_eq!(folder.read(&Slot::Previous), Ok(first));
    folder.write_automatic(&third).expect("writes");
    assert_eq!(folder.read(&Slot::Previous), Ok(second));
    assert!(
        !folder.dir().join("autosave_previous.json.tmp").exists(),
        "no staging file is left behind"
    );
}

#[test]
fn a_damaged_autosave_is_not_kept_over_a_good_previous_one() {
    let scratch = Scratch::new();
    let folder = scratch.folder();
    let (good, newest) = (played().save(), template().save());
    folder.write(&Slot::Previous, &good).expect("writes");
    std::fs::write(folder.path_of(&Slot::Auto), "not json at all").expect("writes");
    folder.write_automatic(&newest).expect("writes");
    assert_eq!(folder.read(&Slot::Auto), Ok(newest));
    assert_eq!(folder.read(&Slot::Previous), Ok(good));
}

#[test]
fn the_old_single_save_is_listed_and_loaded_but_never_written() {
    let scratch = Scratch::new();
    let folder = scratch.folder();
    let save = played().save();
    // The saves folder does not exist yet: this is a first run after the upgrade.
    save.write(&scratch.0.join("save.json")).expect("writes");
    let listed = folder.list().expect("lists");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].slot, Slot::Legacy);
    assert_eq!(
        listed[0].summary.as_ref(),
        Ok(&SaveSummary::of(&save)),
        "it is read like any save"
    );
    assert_eq!(folder.read(&Slot::Legacy), Ok(save.clone()));
    assert!(matches!(
        folder.write(&Slot::Legacy, &save),
        Err(SaveError::Io(_))
    ));
    folder.delete(&Slot::Legacy).expect("it can be discarded");
    assert_eq!(folder.list(), Ok(Vec::new()));
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

/// A save told apart from the others by its clock.
fn stamped(minutes: u64) -> engine::saves::Save {
    let mut save = template().save();
    save.clock_minutes = minutes;
    save
}

#[test]
fn the_history_steps_back_and_drops_the_oldest() {
    let scratch = Scratch::new();
    let folder = scratch.folder();
    let total = u64::from(HISTORY_DEPTH) + 3;
    for minute in 0..total {
        folder.write_automatic(&stamped(minute)).expect("writes");
    }
    assert_eq!(folder.read(&Slot::Auto), Ok(stamped(total - 1)));
    for steps in 1..=HISTORY_DEPTH {
        let slot = Slot::back(steps).expect("kept");
        assert_eq!(
            folder.read(&slot),
            Ok(stamped(total - 1 - u64::from(steps))),
            "{steps} back"
        );
    }
    assert_eq!(
        Slot::back(HISTORY_DEPTH + 1),
        None,
        "nothing is kept deeper"
    );
    assert!(!folder.dir().join("autosave_previous_6.json").exists());
    let listed = folder.list().expect("lists");
    assert_eq!(listed.len(), usize::from(HISTORY_DEPTH) + 1);
    assert!(listed.iter().all(|entry| entry.slot.is_automatic()));
}

#[test]
fn the_earlier_autosave_names_are_kept_for_the_game() {
    for text in [
        "autosave_previous_2",
        "Autosave_Previous_5",
        " autosave_previous ",
    ] {
        assert_eq!(SaveName::try_new(text), Err(NameError::Reserved), "{text}");
    }
    // Only the exact spellings are the game's.
    for text in [
        "autosave_previous_1",
        "autosave_previous_6",
        "autosave_previous_02",
        "autosave_previous_x",
    ] {
        assert!(SaveName::try_new(text).is_ok(), "{text}");
    }
}

#[test]
fn the_old_save_moves_into_the_folder_and_nothing_is_replaced() {
    let scratch = Scratch::new();
    let folder = scratch.folder();
    assert_eq!(folder.adopt_legacy(), Ok(None), "nothing to move");
    let save = played().save();
    save.write(&scratch.0.join("save.json")).expect("writes");
    let moved = folder
        .adopt_legacy()
        .expect("moves")
        .expect("there was one");
    assert_eq!(moved.as_str(), LEGACY_NAME);
    assert!(!folder.exists(&Slot::Legacy));
    assert_eq!(folder.read(&Slot::Named(moved)), Ok(save));
    // A second old save will not take the first one's place.
    let other = template().save();
    other.write(&scratch.0.join("save.json")).expect("writes");
    assert!(matches!(folder.adopt_legacy(), Err(SaveError::Io(_))));
    assert!(folder.exists(&Slot::Legacy), "it stays where it was");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    /// However many automatic saves are written, the latest is the autosave and the
    /// one before it is the previous autosave.
    #[test]
    fn the_autosave_is_the_latest_and_the_previous_is_the_one_before(
        which in prop::collection::vec(any::<bool>(), 2..7)
    ) {
        let scratch = Scratch::new();
        let folder = scratch.folder();
        let saves: Vec<_> = which
            .iter()
            .map(|played_one| if *played_one { played().save() } else { template().save() })
            .collect();
        for save in &saves {
            folder.write_automatic(save).expect("writes");
        }
        prop_assert_eq!(folder.read(&Slot::Auto), Ok(saves[saves.len() - 1].clone()));
        prop_assert_eq!(folder.read(&Slot::Previous), Ok(saves[saves.len() - 2].clone()));
        let kept = usize::from(HISTORY_DEPTH).min(saves.len() - 1);
        for steps in 1..=kept {
            let slot = Slot::back(u8::try_from(steps).expect("small")).expect("kept");
            prop_assert_eq!(folder.read(&slot), Ok(saves[saves.len() - 1 - steps].clone()));
        }
    }
}
