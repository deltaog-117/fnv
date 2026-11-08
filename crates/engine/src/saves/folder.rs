//! A folder of named saves, and the automatic saves beside them.
//!
//! Every save is one file, `<name>.json`, in the folder. A name is checked before it
//! is ever turned into a path, so no name can point outside the folder, and the
//! names `autosave`, `autosave_previous` and `autosave_previous_2` onwards belong to
//! the game: only the automatic saves may use them. The automatic save keeps the
//! ones before it, [`HISTORY_DEPTH`] of them, so a bad moment can be stepped back
//! from. The single `save.json` an older game wrote, one level above the folder, is
//! listed as the old save, can still be loaded, and can be moved into the folder as
//! a named save.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use thiserror::Error;

use super::error::SaveError;
use super::record::Save;
use crate::clock::GameClock;
use crate::shared::LocationId;

/// The name the automatic save is kept under.
pub const AUTOSAVE: &str = "autosave";
/// The name the automatic save before the latest is kept under.
pub const AUTOSAVE_PREVIOUS: &str = "autosave_previous";
/// How many automatic saves are kept before the latest one. The wiki has no such
/// thing, so five is a stand-in: enough to step back a few moments, small enough to
/// stay a handful of files.
pub const HISTORY_DEPTH: u8 = 5;
/// The file an older game wrote, one level above the saves folder.
const LEGACY_FILE: &str = "save.json";
/// The name the old save takes when it is moved into the folder.
pub const LEGACY_NAME: &str = "Old save";
/// The longest name, in characters.
pub const MAX_NAME_LEN: usize = 40;
const EXTENSION: &str = "json";

/// Why a name was refused.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NameError {
    /// Nothing was typed, or only spaces.
    #[error("a save needs a name")]
    Empty,
    /// More than [`MAX_NAME_LEN`] characters.
    #[error("a save name has at most {MAX_NAME_LEN} characters")]
    TooLong,
    /// A character other than a letter, a digit, a space, `-` or `_`.
    #[error("a save name may hold only letters, digits, spaces, '-' and '_', not {0:?}")]
    BadCharacter(char),
    /// The name is an automatic save's.
    #[error("\"{AUTOSAVE}\" and \"{AUTOSAVE_PREVIOUS}*\" are kept for the automatic saves")]
    Reserved,
}

/// The name of a manual save: plain text that is safe as a file name.
///
/// Letters (ASCII), digits, spaces, `-` and `_`, up to [`MAX_NAME_LEN`] characters,
/// trimmed of spaces at both ends, and not `autosave` or `autosave_previous` in any
/// case.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SaveName(String);

impl SaveName {
    /// Whether a typed character may be part of a name.
    #[must_use]
    pub fn allows(character: char) -> bool {
        character.is_ascii_alphanumeric() || matches!(character, ' ' | '-' | '_')
    }

    /// Checks a name.
    ///
    /// # Errors
    ///
    /// [`NameError`] naming what is wrong with it.
    pub fn try_new(text: &str) -> Result<Self, NameError> {
        let trimmed = text.trim_matches(' ');
        if trimmed.is_empty() {
            return Err(NameError::Empty);
        }
        if trimmed.chars().count() > MAX_NAME_LEN {
            return Err(NameError::TooLong);
        }
        if let Some(bad) = trimmed.chars().find(|c| !Self::allows(*c)) {
            return Err(NameError::BadCharacter(bad));
        }
        if trimmed.eq_ignore_ascii_case(AUTOSAVE)
            || steps_of_stem(&trimmed.to_ascii_lowercase()).is_some()
        {
            return Err(NameError::Reserved);
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// The name as typed.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SaveName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// How far back an earlier automatic save is: 2 up to [`HISTORY_DEPTH`]. The save
/// right before the latest is [`Slot::Previous`], so a slot has one spelling only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Depth(u8);

impl Depth {
    /// The depth `steps` back, if it is two or more and the folder keeps that many.
    #[must_use]
    pub fn try_new(steps: u8) -> Option<Self> {
        (2..=HISTORY_DEPTH).contains(&steps).then_some(Self(steps))
    }

    /// How many automatic saves back this is.
    #[must_use]
    pub fn steps(self) -> u8 {
        self.0
    }
}

/// The file stem of the automatic save `steps` back, from 1.
fn stem_back(steps: u8) -> String {
    if steps == 1 {
        AUTOSAVE_PREVIOUS.to_owned()
    } else {
        format!("{AUTOSAVE_PREVIOUS}_{steps}")
    }
}

/// How many steps back a file stem names, if it names an earlier automatic save.
/// `autosave_previous_1` and `autosave_previous_02` are not names.
fn steps_of_stem(stem: &str) -> Option<u8> {
    if stem == AUTOSAVE_PREVIOUS {
        return Some(1);
    }
    let digits = stem.strip_prefix(AUTOSAVE_PREVIOUS)?.strip_prefix('_')?;
    if digits.starts_with('0') || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits
        .parse()
        .ok()
        .filter(|steps| Depth::try_new(*steps).is_some())
}

/// Which save: one the player named, an automatic one, or the old single file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Slot {
    /// A save the player named.
    Named(SaveName),
    /// The automatic save.
    Auto,
    /// The automatic save before the latest one.
    Previous,
    /// An automatic save two or more steps before the latest one.
    Earlier(Depth),
    /// The single `save.json` an older game wrote. It is only ever read or deleted.
    Legacy,
}

impl Slot {
    /// Whether the game writes this slot by itself.
    #[must_use]
    pub fn is_automatic(&self) -> bool {
        matches!(self, Self::Auto | Self::Previous | Self::Earlier(_))
    }

    fn stem(&self) -> String {
        match self {
            Self::Named(name) => name.as_str().to_owned(),
            Self::Auto => AUTOSAVE.to_owned(),
            Self::Previous => AUTOSAVE_PREVIOUS.to_owned(),
            Self::Earlier(depth) => stem_back(depth.steps()),
            Self::Legacy => "save".to_owned(),
        }
    }

    /// The slot `steps` automatic saves back, if the folder keeps that many.
    #[must_use]
    pub fn back(steps: u8) -> Option<Self> {
        match steps {
            1 => Some(Self::Previous),
            _ => Depth::try_new(steps).map(Self::Earlier),
        }
    }

    /// What the player sees.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Named(name) => name.to_string(),
            Self::Auto => "Autosave".to_owned(),
            Self::Previous => "Previous autosave".to_owned(),
            Self::Earlier(depth) => format!("Autosave {} back", depth.steps()),
            Self::Legacy => "Old save".to_owned(),
        }
    }
}

/// What a save says about the game, read without loading it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveSummary {
    /// The day, from 1.
    pub day: u64,
    /// The hour, 0 to 23.
    pub hour: u8,
    /// The minute, 0 to 59.
    pub minute: u8,
    /// Where the player stood, if the game had a map.
    pub place: Option<LocationId>,
    /// Experience earned.
    pub xp: u32,
}

impl SaveSummary {
    /// The summary of a save.
    #[must_use]
    pub fn of(save: &Save) -> Self {
        let clock = GameClock::from_minutes(save.clock_minutes);
        Self {
            day: clock.day(),
            hour: clock.hour(),
            minute: clock.minute(),
            place: save.at.clone(),
            xp: save.character.xp,
        }
    }
}

/// One file in the folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveEntry {
    /// Which save it is.
    pub slot: Slot,
    /// When the file was last written, if the system says.
    pub modified: Option<SystemTime>,
    /// What it holds, or why it cannot be read.
    pub summary: Result<SaveSummary, String>,
}

/// A folder of saves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveFolder {
    dir: PathBuf,
}

impl SaveFolder {
    /// The folder at `dir`. It is created when the first save is written.
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The folder's path.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The file a slot is kept in. Names are checked, so this is always directly
    /// inside the folder, except the old save, which sits beside it.
    #[must_use]
    pub fn path_of(&self, slot: &Slot) -> PathBuf {
        match slot {
            Slot::Legacy => self.dir.parent().map_or_else(
                || PathBuf::from(LEGACY_FILE),
                |parent| parent.join(LEGACY_FILE),
            ),
            _ => self.dir.join(format!("{}.{EXTENSION}", slot.stem())),
        }
    }

    /// Whether a save exists in the slot.
    #[must_use]
    pub fn exists(&self, slot: &Slot) -> bool {
        self.path_of(slot).is_file()
    }

    /// Writes a save into a slot, replacing what was there whole or not at all.
    ///
    /// # Errors
    ///
    /// [`SaveError::Io`] when the folder or the file cannot be written, or when the
    /// slot is the old save, which is never written.
    pub fn write(&self, slot: &Slot, save: &Save) -> Result<(), SaveError> {
        if *slot == Slot::Legacy {
            return Err(SaveError::Io("the old save is never written".to_owned()));
        }
        save.write(&self.path_of(slot))
    }

    /// Writes the automatic save, first keeping the ones it replaces: each earlier
    /// automatic save moves one step back, and the oldest falls off the end. An
    /// automatic save that cannot be read is not kept over a good history. The new
    /// save is written last and whole, so a failure leaves the latest automatic
    /// save as it was, and the history only ever loses its oldest file.
    ///
    /// # Errors
    ///
    /// [`SaveError::Io`] when the folder or a file cannot be written.
    pub fn write_automatic(&self, save: &Save) -> Result<(), SaveError> {
        let latest = self.path_of(&Slot::Auto);
        if latest.is_file() && Save::read(&latest).is_ok() {
            // Oldest first, so no step overwrites a file that has not moved yet.
            for steps in (1..=HISTORY_DEPTH).rev() {
                let from = match steps {
                    1 => Some(Slot::Auto),
                    _ => Slot::back(steps - 1),
                };
                let (Some(from), Some(to)) = (from, Slot::back(steps)) else {
                    continue;
                };
                let from = self.path_of(&from);
                if from.is_file() {
                    copy_whole(&from, &self.path_of(&to))?;
                }
            }
        }
        self.write(&Slot::Auto, save)
    }

    /// Moves the old single `save.json` into the folder as the named save
    /// [`LEGACY_NAME`]. Nothing is replaced: a save already holding that name
    /// refuses the move and the old file stays where it is. `Ok(None)` means there
    /// was no old save.
    ///
    /// # Errors
    ///
    /// [`SaveError::Io`] when the name is taken or a file cannot be moved.
    pub fn adopt_legacy(&self) -> Result<Option<SaveName>, SaveError> {
        let old = self.path_of(&Slot::Legacy);
        if !old.is_file() {
            return Ok(None);
        }
        let name = SaveName::try_new(LEGACY_NAME)
            .map_err(|error| SaveError::Io(format!("the old save cannot be named: {error}")))?;
        let new = self.path_of(&Slot::Named(name.clone()));
        if new.exists() {
            return Err(SaveError::Io(format!(
                "a save named \"{name}\" already exists, so the old save stays where it is"
            )));
        }
        fs::create_dir_all(&self.dir).map_err(|error| {
            SaveError::Io(format!("cannot create {}: {error}", self.dir.display()))
        })?;
        fs::rename(&old, &new)
            .map_err(|error| SaveError::Io(format!("cannot move the old save: {error}")))?;
        Ok(Some(name))
    }

    /// Deletes the save in a slot.
    ///
    /// # Errors
    ///
    /// [`SaveError::Io`] when there is no such save or it cannot be removed.
    pub fn delete(&self, slot: &Slot) -> Result<(), SaveError> {
        let path = self.path_of(slot);
        fs::remove_file(&path)
            .map_err(|error| SaveError::Io(format!("cannot delete {}: {error}", path.display())))
    }

    /// Gives a named save a new name. Nothing is replaced: a save already holding
    /// the new name is refused.
    ///
    /// # Errors
    ///
    /// [`SaveError::Io`] when `from` does not exist, `to` already does, or the file
    /// cannot be renamed.
    pub fn rename(&self, from: &SaveName, to: &SaveName) -> Result<(), SaveError> {
        let (old, new) = (
            self.path_of(&Slot::Named(from.clone())),
            self.path_of(&Slot::Named(to.clone())),
        );
        if !old.is_file() {
            return Err(SaveError::Io(format!("there is no save named \"{from}\"")));
        }
        if new.exists() {
            return Err(SaveError::Io(format!(
                "a save named \"{to}\" already exists"
            )));
        }
        fs::rename(&old, &new)
            .map_err(|error| SaveError::Io(format!("cannot rename \"{from}\": {error}")))
    }

    /// Reads the save in a slot.
    ///
    /// # Errors
    ///
    /// [`SaveError`] when the file is missing or is not a save.
    pub fn read(&self, slot: &Slot) -> Result<Save, SaveError> {
        Save::read(&self.path_of(slot))
    }

    /// Every save in the folder, the most recently written first (and by name among
    /// saves written at the same moment). A file that is not a save is listed with
    /// the reason, so the player can see it is there; files whose names are not
    /// valid save names are left alone. A missing folder holds no saves of its own.
    ///
    /// # Errors
    ///
    /// [`SaveError::Io`] when the folder exists but cannot be read.
    pub fn list(&self) -> Result<Vec<SaveEntry>, SaveError> {
        let reader = match fs::read_dir(&self.dir) {
            Ok(reader) => Some(reader),
            // The folder is made by the first save, but the old save may be there
            // already.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(SaveError::Io(format!(
                    "cannot read {}: {error}",
                    self.dir.display()
                )));
            }
        };
        let legacy = self.path_of(&Slot::Legacy);
        let mut entries: Vec<SaveEntry> = reader
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|file| {
                let path = file.path();
                if path.extension().and_then(|e| e.to_str()) != Some(EXTENSION) {
                    return None;
                }
                let stem = path.file_stem()?.to_str()?;
                let slot = if stem == AUTOSAVE {
                    Slot::Auto
                } else if let Some(steps) = steps_of_stem(stem) {
                    Slot::back(steps)?
                } else {
                    Slot::Named(
                        SaveName::try_new(stem)
                            .ok()
                            .filter(|n| n.as_str() == stem)?,
                    )
                };
                let modified = file.metadata().and_then(|m| m.modified()).ok();
                let summary = Save::read(&path)
                    .map(|save| SaveSummary::of(&save))
                    .map_err(|error| error.to_string());
                Some(SaveEntry {
                    slot,
                    modified,
                    summary,
                })
            })
            .chain(legacy.is_file().then(|| entry_of(Slot::Legacy, &legacy)))
            .collect();
        entries.sort_by(|a, b| {
            b.modified
                .cmp(&a.modified)
                .then_with(|| a.slot.label().cmp(&b.slot.label()))
        });
        Ok(entries)
    }
}

/// Copies a file next to its destination and renames it into place, so the
/// destination is whole or untouched and no half-made copy is left lying around.
fn copy_whole(from: &Path, to: &Path) -> Result<(), SaveError> {
    let staging = to.with_extension("json.tmp");
    fs::copy(from, &staging)
        .and_then(|_| fs::rename(&staging, to))
        .map_err(|error| {
            let _ = fs::remove_file(&staging);
            SaveError::Io(format!("cannot keep {}: {error}", from.display()))
        })
}

/// The listing of one file, which may turn out not to be a save.
fn entry_of(slot: Slot, path: &Path) -> SaveEntry {
    SaveEntry {
        slot,
        modified: fs::metadata(path).and_then(|m| m.modified()).ok(),
        summary: Save::read(path)
            .map(|save| SaveSummary::of(&save))
            .map_err(|error| error.to_string()),
    }
}
