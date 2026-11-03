//! A folder of named saves, and the one automatic save beside them.
//!
//! Every save is one file, `<name>.json`, in the folder. A name is checked before it
//! is ever turned into a path, so no name can point outside the folder, and the
//! name `autosave` belongs to the game: only the automatic save may use it.

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
    /// The name is the automatic save's.
    #[error("\"{AUTOSAVE}\" is kept for the automatic save")]
    Reserved,
}

/// The name of a manual save: plain text that is safe as a file name.
///
/// Letters (ASCII), digits, spaces, `-` and `_`, up to [`MAX_NAME_LEN`] characters,
/// trimmed of spaces at both ends, and not `autosave` in any case.
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
        if trimmed.eq_ignore_ascii_case(AUTOSAVE) {
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

/// Which save: one the player named, or the automatic one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Slot {
    /// A save the player named.
    Named(SaveName),
    /// The automatic save.
    Auto,
}

impl Slot {
    fn stem(&self) -> &str {
        match self {
            Self::Named(name) => name.as_str(),
            Self::Auto => AUTOSAVE,
        }
    }

    /// What the player sees.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Named(name) => name.to_string(),
            Self::Auto => "Autosave".to_owned(),
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
    /// inside the folder.
    #[must_use]
    pub fn path_of(&self, slot: &Slot) -> PathBuf {
        self.dir.join(format!("{}.{EXTENSION}", slot.stem()))
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
    /// [`SaveError::Io`] when the folder or the file cannot be written.
    pub fn write(&self, slot: &Slot, save: &Save) -> Result<(), SaveError> {
        save.write(&self.path_of(slot))
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
    /// valid save names are left alone. A missing folder holds no saves.
    ///
    /// # Errors
    ///
    /// [`SaveError::Io`] when the folder exists but cannot be read.
    pub fn list(&self) -> Result<Vec<SaveEntry>, SaveError> {
        let reader = match fs::read_dir(&self.dir) {
            Ok(reader) => reader,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(SaveError::Io(format!(
                    "cannot read {}: {error}",
                    self.dir.display()
                )));
            }
        };
        let mut entries: Vec<SaveEntry> = reader
            .filter_map(Result::ok)
            .filter_map(|file| {
                let path = file.path();
                if path.extension().and_then(|e| e.to_str()) != Some(EXTENSION) {
                    return None;
                }
                let stem = path.file_stem()?.to_str()?;
                let slot = if stem == AUTOSAVE {
                    Slot::Auto
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
            .collect();
        entries.sort_by(|a, b| {
            b.modified
                .cmp(&a.modified)
                .then_with(|| a.slot.label().cmp(&b.slot.label()))
        });
        Ok(entries)
    }
}
