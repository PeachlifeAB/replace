use sha2::{Digest, Sha256};

mod commit;
mod partial;
mod prepare;

use self::commit::{commit_files, commit_metadata, rollback_error, rollback_metadata};
use self::prepare::{
    PreparedStage, order_content_before_renames, prepare_stages, prevalidate_batch_paths,
};
use super::{ApplyResult, WriteJournal};
use crate::anchor::ChangeSelection;
use crate::error::ReplaceError;
use crate::stage::{StageLock, StageManager};

/// One stage to apply and which of its hunks to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageSelection {
    pub id: String,
    pub hunks: ChangeSelection,
}

impl StageSelection {
    /// Apply every hunk of stage `id`.
    #[must_use]
    pub fn whole(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            hunks: ChangeSelection::All,
        }
    }
}

/// Apply the selected hunks of several staged changes as one prevalidated, rollback-capable batch.
///
/// Hunks that are not selected stay staged under the stage's id, re-based on the written file.
///
/// # Errors
///
/// Returns an error if any stage is unknown, its patch is missing, its file was
/// deleted or is a symlink, drift is detected, a path conflicts within the
/// batch, a selection matches no hunk or cannot be applied in parts, metadata
/// cannot be archived, or a write fails. Prior writes and precommitted metadata
/// are rolled back; rollback failures are included in the returned error.
pub fn apply_stages_atomic(
    manager: &StageManager,
    _lock: &StageLock,
    selections: &[StageSelection],
) -> Result<Vec<ApplyResult>, ReplaceError> {
    let mut prepared = prepare_stages(manager, selections)?;
    order_content_before_renames(&mut prepared);
    prevalidate_batch_paths(&prepared)?;
    commit_metadata(manager, &mut prepared)?;

    let mut journal = WriteJournal::default();
    if let Err(commit_error) = commit_files(&prepared, &mut journal) {
        let file_rollback_failures = journal.roll_back();
        let metadata_errors = rollback_metadata(manager, &prepared);
        return Err(rollback_error(
            &commit_error,
            &file_rollback_failures,
            &metadata_errors,
        ));
    }

    Ok(prepared.iter().map(PreparedStage::result).collect())
}

pub(super) fn content_sha256(content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    hex::encode(digest)
}

#[cfg(test)]
mod tests;
