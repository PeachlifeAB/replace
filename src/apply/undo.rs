use super::helpers::file_sha256;
use super::write::write_atomic;
use crate::error::ReplaceError;
use crate::stage::{StageEntry, StageLock, StageManager, StageMode};
use camino::Utf8PathBuf;

/// Undo an applied change using reverse patch from history.
///
/// # Errors
///
/// Returns an error if the history entry or its patch is missing, the file was
/// deleted or modified since apply ([`ReplaceError::AnchorMismatch`]), or the
/// reverse write fails.
pub fn undo_stage(manager: &StageManager, _lock: &StageLock, id: &str) -> Result<(), ReplaceError> {
    let entry = manager
        .read_history(id)
        .ok_or_else(|| ReplaceError::UnknownHistory(format!("unknown history: {id}")))?;

    match entry.mode {
        StageMode::RenameFile => undo_rename(&entry),
        StageMode::FromError => Err(ReplaceError::InvalidArgs(
            "from-error stages cannot be undone directly".to_owned(),
        )),
        StageMode::Literal | StageMode::Regex | StageMode::Ast => undo_content(manager, id, &entry),
    }?;
    // An undone change is no longer applied: drop it from the log so it cannot be undone twice.
    manager.delete_history(id)?;
    Ok(())
}

fn require_existing(path: &Utf8PathBuf) -> Result<(), ReplaceError> {
    if path.exists() {
        Ok(())
    } else {
        Err(ReplaceError::FileDeleted(format!(
            "file was deleted: {path}"
        )))
    }
}

fn undo_rename(entry: &StageEntry) -> Result<(), ReplaceError> {
    let target = Utf8PathBuf::from(&entry.rewrite);
    require_existing(&target)?;
    if entry.file.exists() {
        return Err(ReplaceError::InvalidArgs(format!(
            "undo target already exists: {}",
            entry.file
        )));
    }
    std::fs::rename(target, &entry.file)?;
    Ok(())
}

/// Verify the file still matches its post-apply hash, if known.
fn require_unmodified_since_apply(entry: &StageEntry) -> Result<(), ReplaceError> {
    match &entry.after_sha {
        Some(after_sha) if &file_sha256(&entry.file)? != after_sha => Err(
            ReplaceError::AnchorMismatch("file has been modified since apply".to_owned()),
        ),
        _ => Ok(()),
    }
}

/// The stored patch is orig -> modified. To undo (modified -> orig) apply diffy's true
/// reverse of that patch to the current (modified) content.
fn undo_content(manager: &StageManager, id: &str, entry: &StageEntry) -> Result<(), ReplaceError> {
    require_existing(&entry.file)?;
    require_unmodified_since_apply(entry)?;

    let patch_content = manager
        .read_history_patch(id)
        .ok_or_else(|| ReplaceError::UnknownHistory(format!("patch not found for: {id}")))?;
    let current_content = std::fs::read_to_string(&entry.file)?;
    let patch = diffy::Patch::from_str(&patch_content)
        .map_err(|e| ReplaceError::Patch(format!("failed to parse patch for undo: {e}")))?;
    let reversed_content = diffy::apply(&current_content, &patch.reverse())
        .map_err(|_| ReplaceError::AnchorMismatch("undo patch failed".to_owned()))?;

    write_atomic(&entry.file, &reversed_content)
}

#[cfg(test)]
mod tests {
    #[test]
    fn reverse_patch_round_trips_to_original() {
        let orig = "one\ntwo\nthree\n";
        let modified = "one\nTWO\nthree\n";
        let patch = diffy::create_patch(orig, modified);
        let restored = diffy::apply(modified, &patch.reverse()).expect("reverse applies");
        assert_eq!(restored, orig);
    }
}
