use camino::Utf8PathBuf;

use crate::error::ReplaceError;
use crate::stage::{StageLock, StageManager, StageMode};

mod atomic;
mod helpers;
mod rollback;
mod undo;
mod write;

pub use atomic::{StageSelection, apply_stages_atomic};
pub(crate) use helpers::{is_plain_file_name, is_same_file};
pub(crate) use rollback::{RollbackFailure, WriteJournal};
pub use undo::undo_stage;
pub(crate) use write::write_atomic;

/// Result of an apply operation.
#[derive(Debug)]
pub struct ApplyResult {
    pub stage_id: String,
    pub file: Utf8PathBuf,
    pub replacements: usize,
}

use helpers::{
    apply_rename_stage_locked, check_drift, file_sha256, find_moved_file_candidates,
    rewrite_content,
};

/// Apply a staged change with drift check, atomic-ish write, and rollback.
///
/// # Errors
///
/// Returns an error if the stage is unknown, its file was deleted or is a
/// symlink, drift is detected, or the atomic write fails (rolled back).
///
/// # Panics
///
/// Panics only if moved-file candidate enumeration contradicts its length check.
pub fn apply_stage(
    manager: &StageManager,
    _lock: &StageLock,
    id: &str,
) -> Result<ApplyResult, ReplaceError> {
    let mut entry = manager
        .read_stage(id)
        .ok_or_else(|| ReplaceError::UnknownStage(format!("unknown stage: {id}")))?;

    if entry.mode == StageMode::RenameFile {
        return apply_rename_stage_locked(manager, id, entry);
    }

    if !entry.file.exists() {
        let candidates = find_moved_file_candidates(&entry)?;
        match candidates.len() {
            0 => {
                return Err(ReplaceError::FileDeleted(format!(
                    "staged file was deleted: {}",
                    entry.file
                )));
            }
            1 => {
                let new_path = candidates.into_iter().next().unwrap();
                eprintln!("note: staged file moved to {new_path}");
                entry.file = new_path;
            }
            _ => {
                let paths: Vec<_> = candidates.iter().map(|p| p.as_str()).collect();
                return Err(ReplaceError::FileMovedAmbiguous(format!(
                    "staged file moved to multiple locations: {}",
                    paths.join(", ")
                )));
            }
        }
    }
    if std::fs::symlink_metadata(&entry.file)?
        .file_type()
        .is_symlink()
    {
        return Err(ReplaceError::InvalidArgs(format!(
            "refusing to apply to symlink: {}",
            entry.file
        )));
    }

    check_drift(&entry)?;

    let backup = std::fs::read_to_string(&entry.file)?;

    let new_content = rewrite_content(&entry, &backup)?;

    write_atomic(&entry.file, &new_content).map_err(|e| {
        match write_atomic(&entry.file, &backup) {
            Ok(()) => ReplaceError::Rollback(format!("write failed; rolled back: {e}")),
            Err(rollback_err) => ReplaceError::Rollback(format!(
                "write failed: {e}; rollback also failed: {rollback_err}"
            )),
        }
    })?;

    let after_sha = file_sha256(&entry.file)?;
    entry.after_sha = Some(after_sha);

    // Update the sidecar with after_sha (may change ID due to collision)
    let patch = manager.read_stage_patch(id).unwrap_or_default();
    let new_id = entry.id.as_str().to_owned();
    manager.write_stage(&mut entry, &patch, false)?;

    // Archive to history using the (possibly new) ID
    manager.archive_to_history(&new_id)?;

    Ok(ApplyResult {
        stage_id: id.to_owned(),
        file: entry.file.clone(),
        replacements: entry.match_count,
    })
}
