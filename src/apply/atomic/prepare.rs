use std::collections::HashSet;

use camino::Utf8PathBuf;

use crate::apply::ApplyResult;
use crate::apply::helpers::{check_drift, validate_rename};
use crate::error::ReplaceError;
use crate::stage::{StageEntry, StageManager, StageMode};

use super::partial::{PartialApply, rewrite_selected};
use super::{StageSelection, content_sha256};

pub(super) struct PreparedStage {
    pub(super) id: String,
    /// The stage as it was staged, with `after_sha` set to the content the apply writes.
    pub(super) entry: StageEntry,
    pub(super) backup: Option<String>,
    pub(super) new_content: Option<String>,
    pub(super) patch: String,
    pub(super) result_file: Utf8PathBuf,
    pub(super) partial: Option<PartialApply>,
}

impl PreparedStage {
    pub(super) fn result(&self) -> ApplyResult {
        let (stage_id, replacements) = self.partial.as_ref().map_or_else(
            || (self.id.clone(), self.entry.match_count),
            |partial| {
                (
                    partial
                        .history_id
                        .clone()
                        .unwrap_or_else(|| self.id.clone()),
                    partial.applied_changes,
                )
            },
        );
        ApplyResult {
            stage_id,
            file: self.result_file.clone(),
            replacements,
        }
    }
}

pub(super) fn prepare_stages(
    manager: &StageManager,
    selections: &[StageSelection],
) -> Result<Vec<PreparedStage>, ReplaceError> {
    selections
        .iter()
        .map(|selection| prepare_stage(manager, selection))
        .collect()
}

pub(super) fn prepare_stage(
    manager: &StageManager,
    selection: &StageSelection,
) -> Result<PreparedStage, ReplaceError> {
    let id = selection.id.as_str();
    let mut entry = manager
        .read_stage(id)
        .ok_or_else(|| ReplaceError::UnknownStage(format!("unknown stage: {id}")))?;
    let patch = manager
        .read_stage_patch(id)
        .ok_or_else(|| ReplaceError::UnknownStage(format!("patch not found for stage: {id}")))?;

    if entry.mode == StageMode::RenameFile {
        let target = Utf8PathBuf::from(&entry.rewrite);
        validate_rename(&entry.file, &target)?;
        return Ok(PreparedStage {
            id: id.to_owned(),
            entry,
            backup: None,
            new_content: None,
            patch,
            result_file: target,
            partial: None,
        });
    }

    validate_content_stage(&entry)?;

    // If file doesn't exist, skip backup and drift check (moved-file detection happens later)
    let backup = if entry.file.exists() {
        check_drift(&entry)?;
        Some(std::fs::read_to_string(&entry.file)?)
    } else {
        None
    };

    let (new_content, partial) = match &backup {
        Some(original) => {
            let (content, partial) = rewrite_selected(&entry, &patch, original, &selection.hunks)?;
            (Some(content), partial)
        }
        None => (None, None),
    };
    entry.after_sha = new_content.as_ref().map(|c| content_sha256(c));
    let result_file = entry.file.clone();

    Ok(PreparedStage {
        id: id.to_owned(),
        entry,
        backup,
        new_content,
        patch,
        result_file,
        partial,
    })
}

pub(super) fn validate_content_stage(entry: &StageEntry) -> Result<(), ReplaceError> {
    if !entry.file.exists() {
        return Ok(());
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
    Ok(())
}

/// Reject a batch whose stages fight over a path.
///
/// A content rewrite and a rename may share their source path: content is written first
/// ([`order_content_before_renames`]), then the file is renamed.
pub(super) fn prevalidate_batch_paths(prepared: &[PreparedStage]) -> Result<(), ReplaceError> {
    let mut content_files = HashSet::new();
    let mut rename_sources = HashSet::new();
    let mut rename_targets = HashSet::new();
    for stage in prepared {
        if stage.entry.mode == StageMode::RenameFile {
            claim(&mut rename_sources, &stage.entry.file)?;
            claim(&mut rename_targets, &stage.result_file)?;
        } else {
            claim(&mut content_files, &stage.entry.file)?;
        }
    }
    for target in &rename_targets {
        if content_files.contains(target) || rename_sources.contains(target) {
            return Err(path_conflict(target));
        }
    }

    detect_moved_files(prepared)?;

    Ok(())
}

fn claim(seen: &mut HashSet<Utf8PathBuf>, path: &Utf8PathBuf) -> Result<(), ReplaceError> {
    if seen.insert(path.clone()) {
        Ok(())
    } else {
        Err(path_conflict(path))
    }
}

fn path_conflict(path: &Utf8PathBuf) -> ReplaceError {
    ReplaceError::StageConflict(format!(
        "multiple stages touch the same path in one atomic apply: {path}"
    ))
}

/// Content rewrites run before renames so a rename moves the already-rewritten file, and
/// renames run deepest path first so a folder is renamed after everything inside it.
pub(super) fn order_content_before_renames(prepared: &mut [PreparedStage]) {
    prepared.sort_by_key(|stage| {
        let rename = stage.entry.mode == StageMode::RenameFile;
        let depth = if rename {
            stage.entry.file.components().count()
        } else {
            0
        };
        (rename, std::cmp::Reverse(depth))
    });
}

/// Detect if any staged files have been moved/renamed since staging.
///
/// If a staged file's source doesn't exist, check if a file with the same
/// content hash (`source_sha`) exists at a different path. This indicates the
/// file was moved or renamed.
pub(super) fn detect_moved_files(prepared: &[PreparedStage]) -> Result<(), ReplaceError> {
    let mut hash_to_path: std::collections::HashMap<String, Utf8PathBuf> =
        std::collections::HashMap::new();

    for stage in prepared {
        if stage.entry.mode != StageMode::RenameFile
            && let Some(ref source_sha) = stage.entry.source_sha
            && stage.entry.file.exists()
        {
            hash_to_path.insert(source_sha.clone(), stage.entry.file.clone());
        }
    }

    for stage in prepared {
        if stage.entry.mode == StageMode::RenameFile {
            continue;
        }

        if !stage.entry.file.exists()
            && let Some(ref source_sha) = stage.entry.source_sha
            && let Some(moved_path) = hash_to_path.get(source_sha)
            && moved_path != &stage.entry.file
        {
            return Err(ReplaceError::FileDeleted(format!(
                "staged file was moved/renamed: {} -> {}",
                stage.entry.file, moved_path
            )));
        }
    }

    Ok(())
}
