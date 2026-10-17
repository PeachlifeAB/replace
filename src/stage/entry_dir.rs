//! One directory of `<id>.json` + `<id>.patch` pairs; both `stage/` and `history/` are one.
use std::fs;
use std::io;

use camino::Utf8PathBuf;

use super::StageEntry;

const ENTRY_EXTENSION: &str = "json";
const PATCH_EXTENSION: &str = "patch";

pub(super) struct EntryDir {
    dir: Utf8PathBuf,
}

impl EntryDir {
    pub(super) const fn new(dir: Utf8PathBuf) -> Self {
        Self { dir }
    }

    pub(super) fn entry_path(&self, id: &str) -> Utf8PathBuf {
        self.dir.join(format!("{id}.{ENTRY_EXTENSION}"))
    }

    pub(super) fn patch_path(&self, id: &str) -> Utf8PathBuf {
        self.dir.join(format!("{id}.{PATCH_EXTENSION}"))
    }

    pub(super) fn read_entry(&self, id: &str) -> Option<StageEntry> {
        let json = fs::read_to_string(self.entry_path(id)).ok()?;
        serde_json::from_str(&json).ok()
    }

    pub(super) fn read_patch(&self, id: &str) -> Option<String> {
        fs::read_to_string(self.patch_path(id)).ok()
    }

    /// Every readable entry, oldest first.
    pub(super) fn list(&self) -> Vec<StageEntry> {
        let Ok(files) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut entries: Vec<StageEntry> = files
            .flatten()
            .map(|file| file.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some(ENTRY_EXTENSION))
            .filter_map(|path| fs::read_to_string(path).ok())
            .filter_map(|json| serde_json::from_str(&json).ok())
            .collect();
        entries.sort_by_key(|entry| entry.created_at);
        entries
    }

    pub(super) fn remove(&self, id: &str) -> io::Result<()> {
        fs::remove_file(self.entry_path(id))?;
        fs::remove_file(self.patch_path(id))
    }

    /// Move an entry and its patch into `destination`.
    pub(super) fn move_to(&self, destination: &Self, id: &str) -> io::Result<()> {
        fs::rename(self.entry_path(id), destination.entry_path(id))?;
        fs::rename(self.patch_path(id), destination.patch_path(id))
    }
}
