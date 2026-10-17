//! Direct (`--apply`) writes with all-or-nothing rollback across files.
use camino::Utf8PathBuf;

use crate::apply::WriteJournal;
use crate::error::ReplaceError;
use crate::stage::{StageEntry, StageMode};

/// Everything a direct apply changed so far, newest last.
#[derive(Default)]
pub(super) struct DirectApplyLog {
    journal: WriteJournal,
    /// Output lines held back until every write succeeded.
    pub(super) lines: Vec<String>,
}

impl DirectApplyLog {
    /// Undo every recorded write, newest first; failures to undo are appended to the error.
    pub(super) fn roll_back(&mut self, cause: ReplaceError) -> ReplaceError {
        if self.journal.is_empty() {
            return cause;
        }
        let failures = self.journal.roll_back();
        if failures.is_empty() {
            return ReplaceError::Rollback(format!("{cause}; earlier writes were rolled back"));
        }
        let failure_messages: Vec<String> = failures
            .iter()
            .map(|failure| failure.cause.to_string())
            .collect();
        ReplaceError::Rollback(format!(
            "{cause}; rollback also failed: {}",
            failure_messages.join("; ")
        ))
    }
}

/// One direct write: the pre-image of the file (absent for renames) and the shared log.
pub(super) struct DirectWrite<'a> {
    pub(super) original: Option<&'a str>,
    pub(super) log: &'a mut DirectApplyLog,
}

impl DirectWrite<'_> {
    /// Write `new_content` (or rename) for `entry`, recording how to undo it.
    pub(super) fn apply(
        &mut self,
        entry: &StageEntry,
        new_content: Option<&str>,
    ) -> Result<Utf8PathBuf, ReplaceError> {
        if entry.mode == StageMode::RenameFile {
            let target = Utf8PathBuf::from(&entry.rewrite);
            self.log.journal.rename(&entry.file, &target)?;
            return Ok(target);
        }

        let content = new_content.ok_or_else(|| {
            ReplaceError::InvalidArgs("missing rewrite output for direct apply".to_owned())
        })?;
        self.log
            .journal
            .write(&entry.file, content, self.original)?;
        Ok(entry.file.clone())
    }
}
