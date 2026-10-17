//! Committing one rewrite: staged for review, or written now with its undo recorded.
use std::collections::HashMap;
use std::fs;

use super::super::direct::DirectWrite;
use camino::{Utf8Path, Utf8PathBuf};

use super::super::emit::{print_staged, queue_applied};
use super::RewriteRun;
use crate::cli::StagePolicy;
use crate::cli::util::source_sha256;
use crate::error::ReplaceError;
use crate::stage::{StageEntry, StageManager, StageMode};

/// Content stages waiting in `.replace/stage`, by file: the ID and the text each was staged against.
#[derive(Default)]
pub(super) struct PendingStages(HashMap<Utf8PathBuf, Vec<(String, Option<String>)>>);

impl PendingStages {
    pub(super) fn load(manager: &StageManager) -> Self {
        let mut pending = Self::default();
        for entry in manager.list_stages() {
            if matches!(
                entry.mode,
                StageMode::Literal | StageMode::Regex | StageMode::Ast
            ) {
                pending
                    .0
                    .entry(normalized(&entry.file))
                    .or_default()
                    .push((entry.id.0, entry.source_sha));
            }
        }
        pending
    }

    /// Take the stages of `file` that were staged against other text than `current_sha`, and
    /// any copy of the change just written; `keep_id` is the change just staged.
    fn take_outdated(
        &mut self,
        file: &Utf8Path,
        keep_id: Option<&str>,
        current_sha: &str,
    ) -> Vec<String> {
        let Some(stages) = self.0.get_mut(&normalized(file)) else {
            return Vec::new();
        };
        let (outdated, kept): (Vec<_>, Vec<_>) = stages.drain(..).partition(|(id, sha)| {
            Some(id.as_str()) != keep_id && sha.as_deref() != Some(current_sha)
        });
        *stages = kept;
        outdated.into_iter().map(|(id, _)| id).collect()
    }
}

/// `./a.txt` and `a.txt` name the same file.
fn normalized(path: &Utf8Path) -> Utf8PathBuf {
    Utf8PathBuf::from(path.as_str().strip_prefix("./").unwrap_or(path.as_str()))
}

impl RewriteRun<'_> {
    /// Keep a `.orig` copy when a direct apply is about to overwrite the file.
    pub(super) fn write_backup(&self, path: &Utf8PathBuf, original: &str) {
        if !self.options.backup || self.stages() {
            return;
        }
        let backup_path = format!("{path}.orig");
        if let Err(error) = fs::write(&backup_path, original) {
            eprintln!("warning: failed to create backup at {backup_path}: {error}");
        }
    }

    pub(super) const fn stages(&self) -> bool {
        matches!(self.options.stage_policy, StagePolicy::Stage)
    }

    /// Stage the entry for review, or apply it now and queue its report line.
    pub(super) fn commit(
        &mut self,
        mut entry: StageEntry,
        patch: &str,
        change: DirectChange<'_>,
    ) -> Result<(), ReplaceError> {
        if self.stages() {
            // The ID is content-derived, so re-running the same rewrite restages in place.
            if self.manager.read_stage(entry.id.as_str()).is_none() {
                self.new_stages.push(entry.id.0.clone());
            }
            self.manager.write_stage(&mut entry, patch, false)?;
            if let Some(source_sha) = entry.source_sha.clone() {
                self.note_outdated_stages(&entry, Some(entry.id.as_str()), &source_sha);
            }
            self.outcome.record(&entry);
            if self.options.output.json {
                self.records.record_staged(&entry);
                return Ok(());
            }
            return print_staged(&entry, patch, self.options.output);
        }

        let applied_file = DirectWrite {
            original: change.original,
            log: &mut self.direct_log,
        }
        .apply(&entry, change.new_content)?;
        if let Some(new_content) = change.new_content {
            let after_sha = source_sha256(new_content.as_bytes());
            self.note_outdated_stages(&entry, None, &after_sha);
            entry.after_sha = Some(after_sha);
        }
        self.outcome.record(&entry);
        if self.options.output.json {
            self.records.record_applied(&entry, &applied_file);
        } else {
            queue_applied(&entry, &applied_file, &mut self.direct_log);
        }
        self.applied.push((entry, patch.to_owned()));
        Ok(())
    }

    /// Note earlier stages of this file that no longer match its text: they could only fail
    /// with drift, and would block `replace apply` for everything staged with them.
    fn note_outdated_stages(
        &mut self,
        entry: &StageEntry,
        keep_id: Option<&str>,
        current_sha: &str,
    ) {
        let outdated = self
            .pending
            .take_outdated(&entry.file, keep_id, current_sha);
        self.outdated_stages.extend(outdated);
    }

    /// Once the whole run succeeded: record direct writes in history, so each printed ID can be
    /// undone, and discard the stages they made outdated.
    pub(super) fn settle_stage_state(&self) -> Result<(), ReplaceError> {
        if self.applied.is_empty() && self.outdated_stages.is_empty() {
            return Ok(());
        }
        let _lock = self.manager.lock_exclusive()?;
        for (entry, patch) in &self.applied {
            self.manager.write_history(entry, patch)?;
        }
        for id in &self.outdated_stages {
            self.manager.discard_stage(id)?;
        }
        Ok(())
    }
}

/// Content and pre-image of a change, needed only when it is applied directly.
#[derive(Clone, Copy)]
pub(super) struct DirectChange<'a> {
    pub(super) new_content: Option<&'a str>,
    pub(super) original: Option<&'a str>,
}
