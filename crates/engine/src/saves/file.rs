//! Writing a save to disk without ever leaving half a file behind.

use std::fs;
use std::io::Write;
use std::path::Path;

use super::error::SaveError;
use super::record::Save;

impl Save {
    /// Writes the save to `path`. The text goes to a neighbouring temporary file
    /// first and is renamed over `path` once it is complete on disk, so a crash
    /// leaves either the old save or the new one, never a torn file.
    ///
    /// # Errors
    ///
    /// [`SaveError::Io`] when a directory, the file or the rename fails.
    pub fn write(&self, path: &Path) -> Result<(), SaveError> {
        let io = |what: &str, error: std::io::Error| {
            SaveError::Io(format!("{what} {}: {error}", path.display()))
        };
        let text = self.to_json()?;
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)
                .map_err(|error| io("cannot create the folder for", error))?;
        }
        let temporary = path.with_extension("json.tmp");
        let mut file = fs::File::create(&temporary).map_err(|error| io("cannot write", error))?;
        file.write_all(text.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|error| io("cannot write", error))?;
        fs::rename(&temporary, path).map_err(|error| io("cannot replace", error))
    }

    /// Reads a save from `path`.
    ///
    /// # Errors
    ///
    /// [`SaveError::Io`] when the file cannot be read, and the errors of
    /// [`Save::from_json`] when its text is not a save.
    pub fn read(path: &Path) -> Result<Self, SaveError> {
        let text = fs::read_to_string(path)
            .map_err(|error| SaveError::Io(format!("cannot read {}: {error}", path.display())))?;
        Self::from_json(&text)
    }
}
