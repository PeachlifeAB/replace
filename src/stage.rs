use camino::Utf8PathBuf;
use std::fs::{self, File};
use std::time::Duration;

use crate::config::REPLACE_DIR_NAME;
use crate::error::ReplaceError;

mod entry_dir;
mod types;

use entry_dir::EntryDir;
pub use types::{StageEntry, StageId, StageMode};

const STAGE_DIR_NAME: &str = "stage";
const HISTORY_DIR_NAME: &str = "history";
const LOCK_FILE_NAME: &str = "lock";
const GITIGNORE_FILE_NAME: &str = ".gitignore";

/// Keeps scratch state out of git while leaving `.replace/config.toml` trackable.
const STATE_GITIGNORE: &str = "# Written by replace: scratch state, not project files.\n\
/stage/\n\
/history/\n\
/lock\n";

/// Proof that this process holds the repository's stage lock; dropping it unlocks.
///
/// Operations that read and then change staged state take `&StageLock`, so none can run unlocked.
#[derive(Debug)]
pub struct StageLock {
    _file: File,
}

/// Manage the `.replace/stage` and `.replace/history` directories.
pub struct StageManager {
    root: Utf8PathBuf,
    lock_timeout: Option<Duration>,
}

impl StageManager {
    pub fn new(root: impl Into<Utf8PathBuf>) -> Self {
        Self {
            root: root.into(),
            lock_timeout: None,
        }
    }

    /// Give up acquiring the stage lock after `timeout` instead of waiting indefinitely.
    #[must_use]
    pub const fn with_lock_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.lock_timeout = timeout;
        self
    }

    fn state_dir(&self) -> Utf8PathBuf {
        self.root.join(REPLACE_DIR_NAME)
    }

    fn stage_dir(&self) -> Utf8PathBuf {
        self.state_dir().join(STAGE_DIR_NAME)
    }

    fn history_dir(&self) -> Utf8PathBuf {
        self.state_dir().join(HISTORY_DIR_NAME)
    }

    fn stages(&self) -> EntryDir {
        EntryDir::new(self.stage_dir())
    }

    fn history(&self) -> EntryDir {
        EntryDir::new(self.history_dir())
    }

    fn lock_path(&self) -> Utf8PathBuf {
        self.state_dir().join(LOCK_FILE_NAME)
    }

    /// Ensure `.replace/stage` and `.replace/history` exist, and that git ignores them.
    ///
    /// # Errors
    ///
    /// Returns an error if the directories cannot be created.
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.stage_dir())?;
        fs::create_dir_all(self.history_dir())?;
        let gitignore = self.state_dir().join(GITIGNORE_FILE_NAME);
        if !gitignore.exists() {
            fs::write(gitignore, STATE_GITIGNORE)?;
        }
        Ok(())
    }

    /// Acquire the exclusive repository-local lock for staged operations, honoring the
    /// timeout set with [`StageManager::with_lock_timeout`].
    ///
    /// # Errors
    ///
    /// Returns an error if the lock file cannot be created, or the lock cannot be acquired
    /// within the configured timeout.
    pub fn lock_exclusive(&self) -> Result<StageLock, ReplaceError> {
        self.ensure_dirs()?;
        let file = self.lock_timeout.map_or_else(
            || crate::lock::lock_file(self.lock_path()),
            |timeout| {
                crate::lock::lock_file_with_timeout(self.lock_path(), timeout, "replace-stage")
            },
        )?;
        Ok(StageLock { _file: file })
    }

    /// Generate a unique stage ID, appending `-2`, `-3`, etc. when the base ID is taken.
    #[must_use]
    pub fn unique_stage_id(&self, base_id: &str) -> String {
        let base_json_path = self.stage_dir().join(format!("{base_id}.json"));
        if !base_json_path.exists() {
            return base_id.to_string();
        }

        let mut suffix = 2;
        loop {
            let candidate = format!("{base_id}-{suffix}");
            let candidate_path = self.stages().entry_path(&candidate);
            if !candidate_path.exists() {
                return candidate;
            }
            suffix += 1;
        }
    }

    /// Write a stage entry and its patch file to `.replace/stage`.
    ///
    /// If `allow_collision` is true and a collision occurs, appends `-2`, `-3`, etc. to the stage ID.
    /// If `allow_collision` is false, the stage ID is used as-is (for updates).
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or the filesystem writes fail.
    pub fn write_stage(
        &self,
        entry: &mut StageEntry,
        patch: &str,
        allow_collision: bool,
    ) -> std::io::Result<()> {
        self.ensure_dirs()?;

        let id = if allow_collision {
            self.unique_stage_id(entry.id.as_str())
        } else {
            entry.id.as_str().to_owned()
        };
        entry.id = StageId(id.clone());

        let json = serde_json::to_string(entry)?;
        fs::write(self.stages().entry_path(&id), json)?;

        fs::write(self.stages().patch_path(&id), patch)?;

        Ok(())
    }

    /// Record a change that was written without being staged, so `undo` can reverse it.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or the filesystem writes fail.
    pub fn write_history(&self, entry: &StageEntry, patch: &str) -> std::io::Result<()> {
        self.ensure_dirs()?;
        let id = entry.id.as_str();
        fs::write(self.history().entry_path(id), serde_json::to_string(entry)?)?;
        fs::write(self.history().patch_path(id), patch)
    }

    /// Read a stage entry from `.replace/stage`.
    #[must_use]
    pub fn read_stage(&self, id: &str) -> Option<StageEntry> {
        self.stages().read_entry(id)
    }

    /// Read a patch from `.replace/stage`.
    #[must_use]
    pub fn read_stage_patch(&self, id: &str) -> Option<String> {
        self.stages().read_patch(id)
    }

    /// List all staged entries.
    #[must_use]
    pub fn list_stages(&self) -> Vec<StageEntry> {
        self.stages().list()
    }

    /// Discard a stage (remove from `.replace/stage`).
    ///
    /// # Errors
    ///
    /// Returns an error if the stage files cannot be removed.
    pub fn discard_stage(&self, id: &str) -> std::io::Result<()> {
        self.stages().remove(id)
    }

    /// Archive a stage to history (move from `.replace/stage` to `.replace/history`).
    ///
    /// # Errors
    ///
    /// Returns an error if the stage files cannot be moved.
    pub fn archive_to_history(&self, id: &str) -> std::io::Result<()> {
        self.ensure_dirs()?;
        self.stages().move_to(&self.history(), id)
    }

    /// Restore a history entry back to staged state.
    ///
    /// # Errors
    ///
    /// Returns an error if the history files cannot be moved.
    pub fn restore_history_to_stage(&self, id: &str) -> std::io::Result<()> {
        self.ensure_dirs()?;
        self.history().move_to(&self.stages(), id)
    }

    /// Read a history entry from `.replace/history`.
    #[must_use]
    pub fn read_history(&self, id: &str) -> Option<StageEntry> {
        self.history().read_entry(id)
    }

    /// Read a history patch from `.replace/history`.
    #[must_use]
    pub fn read_history_patch(&self, id: &str) -> Option<String> {
        self.history().read_patch(id)
    }

    /// List all history entries.
    #[must_use]
    pub fn list_history(&self) -> Vec<StageEntry> {
        self.history().list()
    }

    /// Delete a history entry (remove from `.replace/history`).
    ///
    /// # Errors
    ///
    /// Returns an error if the history files cannot be removed.
    pub fn delete_history(&self, id: &str) -> std::io::Result<()> {
        self.history().remove(id)
    }
}

#[cfg(test)]
mod tests;
