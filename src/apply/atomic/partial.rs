//! Applying only some hunks of a stage and keeping the remainder staged.
use crate::anchor::{self, ChangeSelection};
use crate::apply::helpers::rewrite_content;
use crate::error::ReplaceError;
use crate::patch::unified_diff;
use crate::stage::{StageEntry, StageId, StageManager};

use super::content_sha256;
use super::prepare::PreparedStage;

/// A stage applied only in part: the applied hunks are archived under a new id and the rest
/// stays staged under the original id, re-based on the file as it is after the apply.
pub(super) struct PartialApply {
    pub(super) applied_patch: String,
    pub(super) applied_changes: usize,
    pub(super) remaining_entry: StageEntry,
    pub(super) remaining_patch: String,
    /// Set once the applied part is archived.
    pub(super) history_id: Option<String>,
}

impl PreparedStage {
    /// Move what this apply wrote out of `.replace/stage` into history.
    pub(super) fn archive_applied(&mut self, manager: &StageManager) -> Result<(), ReplaceError> {
        let Some(partial) = self.partial.as_mut() else {
            manager.write_stage(&mut self.entry, &self.patch, false)?;
            manager.archive_to_history(&self.id)?;
            return Ok(());
        };
        let mut applied = self.entry.clone();
        applied.id =
            StageId::from_stage_bytes(format!("{}\0{}", self.id, partial.applied_patch).as_bytes());
        applied.anchors =
            anchor::extract_anchors_from_patch(&partial.applied_patch, applied.file.as_str());
        applied.match_count = partial.applied_changes;
        manager.write_stage(&mut applied, &partial.applied_patch, true)?;
        let history_id = applied.id.as_str().to_owned();
        manager.archive_to_history(&history_id)?;
        partial.history_id = Some(history_id);
        manager.write_stage(
            &mut partial.remaining_entry,
            &partial.remaining_patch,
            false,
        )?;
        Ok(())
    }

    /// Put the stage back as it was staged, undoing [`PreparedStage::archive_applied`].
    pub(super) fn restore_staged(&self, manager: &StageManager) -> std::io::Result<()> {
        let Some(partial) = &self.partial else {
            return manager.restore_history_to_stage(&self.id);
        };
        if let Some(history_id) = &partial.history_id {
            manager.delete_history(history_id)?;
        }
        let mut original = self.entry.clone();
        original.after_sha = None;
        manager.write_stage(&mut original, &self.patch, false)
    }
}

/// The file content after applying the selected hunks of `entry`, and the remainder to keep staged.
pub(super) fn rewrite_selected(
    entry: &StageEntry,
    patch: &str,
    original: &str,
    hunks: &ChangeSelection,
) -> Result<(String, Option<PartialApply>), ReplaceError> {
    let full = rewrite_content(entry, original)?;
    if *hunks == ChangeSelection::All {
        return Ok((full, None));
    }

    let applied_patch = anchor::retain_changes(patch, entry.file.as_str(), hunks);
    let applied_changes = anchor::count_changes(&applied_patch);
    if applied_changes == 0 {
        return Err(ReplaceError::InvalidArgs(format!(
            "no hunk of stage {} matches the selected anchors",
            entry.id.as_str()
        )));
    }
    if applied_changes == anchor::count_changes(patch) {
        return Ok((full, None));
    }

    let selected = diffy::Patch::from_str(&applied_patch)
        .map_err(|error| ReplaceError::Patch(format!("failed to parse selected hunks: {error}")))?;
    let partial_content = diffy::apply(original, &selected)
        .map_err(|_| ReplaceError::AnchorMismatch("selected hunks no longer apply".to_owned()))?;
    if rewrite_content(entry, &partial_content)? != full {
        return Err(ReplaceError::InvalidArgs(format!(
            "stage {} rewrites its own output; apply it whole, not by anchors",
            entry.id.as_str()
        )));
    }

    let remaining_patch = unified_diff(&partial_content, &full);
    let mut remaining_entry = entry.clone();
    remaining_entry.source_sha = Some(content_sha256(&partial_content));
    remaining_entry.after_sha = None;
    remaining_entry.anchors =
        anchor::extract_anchors_from_patch(&remaining_patch, entry.file.as_str());
    remaining_entry.match_count = anchor::count_changes(&remaining_patch);
    Ok((
        partial_content,
        Some(PartialApply {
            applied_patch,
            applied_changes,
            remaining_entry,
            remaining_patch,
            history_id: None,
        }),
    ))
}
