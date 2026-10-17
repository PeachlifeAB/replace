use crate::apply::{RollbackFailure, WriteJournal};
use crate::error::ReplaceError;
use crate::stage::{StageManager, StageMode};

use super::prepare::PreparedStage;

pub(super) fn commit_metadata(
    manager: &StageManager,
    prepared: &mut [PreparedStage],
) -> Result<(), ReplaceError> {
    for index in 0..prepared.len() {
        if let Err(error) = prepared[index].archive_applied(manager) {
            let _ = prepared[index].restore_staged(manager);
            for done in prepared[..index].iter().rev() {
                let _ = done.restore_staged(manager);
            }
            return Err(error);
        }
    }
    Ok(())
}

pub(super) fn commit_files(
    prepared: &[PreparedStage],
    journal: &mut WriteJournal,
) -> Result<(), ReplaceError> {
    for stage in prepared {
        if stage.entry.mode == StageMode::RenameFile {
            journal.rename(&stage.entry.file, &stage.result_file)?;
        } else if let Some(new_content) = &stage.new_content {
            journal.write(&stage.entry.file, new_content, stage.backup.as_deref())?;
        } else {
            return Err(ReplaceError::InvalidArgs(
                "missing rewrite output for atomic apply".to_owned(),
            ));
        }
    }
    Ok(())
}

pub(super) fn rollback_metadata(manager: &StageManager, prepared: &[PreparedStage]) -> Vec<String> {
    let mut errors = Vec::new();
    for stage in prepared.iter().rev() {
        if let Err(error) = stage.restore_staged(manager) {
            errors.push(format!("{}: {error}", stage.id));
        }
    }
    errors
}

pub(super) fn rollback_error(
    error: &ReplaceError,
    file_rollback_failures: &[RollbackFailure],
    metadata_errors: &[String],
) -> ReplaceError {
    let mut message = format!("atomic apply failed; rolled back prior writes: {error}");
    if !file_rollback_failures.is_empty() {
        let failures: Vec<String> = file_rollback_failures
            .iter()
            .map(|failure| format!("{}: {}", failure.path, failure.cause))
            .collect();
        message.push_str("; file rollback failures: ");
        message.push_str(&failures.join("; "));
    }
    if !metadata_errors.is_empty() {
        message.push_str("; metadata rollback failures: ");
        message.push_str(&metadata_errors.join("; "));
    }
    ReplaceError::Rollback(message)
}
