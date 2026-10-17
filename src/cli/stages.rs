use crate::anchor;
use crate::error::ReplaceError;
use crate::stage::{StageEntry, StageLock, StageManager};
use std::time::Duration;

use super::OutputOptions;
use super::util::{stage_manager, write_stdout_line};

mod apply;
mod detail;

pub use apply::{AnchorSelection, execute_apply, execute_apply_staged};
pub use detail::{execute_check, execute_explain, execute_gc, execute_log};

pub fn execute_show(stage_id: Option<&str>, output: OutputOptions) -> Result<(), ReplaceError> {
    let manager = stage_manager()?;
    let Some(id) = stage_id else {
        return execute_list(output);
    };
    anchor::parse_anchor(id).map_or_else(
        || show_stage(&manager, id, output),
        |parsed| show_stage_holding_anchor(&manager, id, &parsed, output),
    )
}

fn print_stage(entry: &StageEntry, patch: &str, output: OutputOptions) -> Result<(), ReplaceError> {
    if output.json {
        let mut detail = serde_json::to_value(entry)?;
        detail["patch"] = patch.into();
        return write_stdout_line(format_args!("{detail}"));
    }
    let diff = output.diff;
    write_stdout_line(format_args!(
        "Stage: {} {:?} {} ({} replacements)",
        entry.id.as_str(),
        entry.mode,
        entry.file,
        entry.match_count
    ))?;
    diff.render(patch, entry.file.as_str())
        .map_or(Ok(()), |rendered| {
            write_stdout_line(format_args!("{rendered}"))
        })
}

fn show_stage(manager: &StageManager, id: &str, output: OutputOptions) -> Result<(), ReplaceError> {
    let entry = manager
        .read_stage(id)
        .ok_or_else(|| ReplaceError::UnknownStage(format!("unknown stage: {id}")))?;
    let patch = manager
        .read_stage_patch(id)
        .ok_or_else(|| ReplaceError::UnknownStage(format!("missing patch for stage: {id}")))?;
    print_stage(&entry, &patch, output)
}

/// Show the stage that owns `anchor_text` (a `path#hash` reference).
fn show_stage_holding_anchor(
    manager: &StageManager,
    anchor_text: &str,
    wanted: &(String, String),
    output: OutputOptions,
) -> Result<(), ReplaceError> {
    let owner = manager.list_stages().into_iter().find(|entry| {
        entry
            .anchors
            .iter()
            .any(|a| anchor::parse_anchor(a).as_ref() == Some(wanted))
    });
    let not_found =
        || ReplaceError::UnknownStage(format!("anchor not found in any stage: {anchor_text}"));
    let entry = owner.ok_or_else(not_found)?;
    let patch = manager
        .read_stage_patch(entry.id.as_str())
        .ok_or_else(not_found)?;
    print_stage(&entry, &patch, output)?;
    if output.json {
        return Ok(());
    }
    write_stdout_line(format_args!("Anchor: {anchor_text}"))
}

pub fn execute_list(output: OutputOptions) -> Result<(), ReplaceError> {
    let manager = stage_manager()?;
    let stages = manager.list_stages();
    if output.json {
        write_stdout_line(format_args!("{}", serde_json::to_string(&stages)?))?;
        return Ok(());
    }

    if stages.is_empty() {
        write_stdout_line(format_args!("No stages yet"))?;
        return Ok(());
    }

    for entry in stages {
        write_stdout_line(format_args!(
            "{}\t{:?}\t{}\t{} replacements",
            entry.id.as_str(),
            entry.mode,
            entry.file,
            entry.match_count
        ))?;
    }

    Ok(())
}

pub fn execute_discard(
    stage_id: Option<&str>,
    lock_timeout: Option<Duration>,
) -> Result<(), ReplaceError> {
    let Some(locked) = locked_stages(stage_id, lock_timeout)? else {
        return Ok(());
    };

    let mut failures = Vec::new();
    for entry in &locked.entries {
        let id = entry.id.as_str();
        match locked.manager.discard_stage(id) {
            Ok(()) => write_stdout_line(format_args!("Discarded: {id}"))?,
            Err(err) => failures.push(format!("{id}: {err}")),
        }
    }

    if !failures.is_empty() {
        return Err(ReplaceError::StageConflict(format!(
            "failed to discard {} stage(s): {}",
            failures.len(),
            failures.join(", ")
        )));
    }

    Ok(())
}

/// The stages a command works on, read while holding the stage lock.
pub struct LockedStages {
    pub(crate) manager: StageManager,
    pub(crate) lock: StageLock,
    pub(crate) entries: Vec<StageEntry>,
}

/// Lock the stage store, then read the stage named by `stage_id` (all stages when `None`).
///
/// Reports `No stages yet` and returns `None` when there is nothing to work on.
fn locked_stages(
    stage_id: Option<&str>,
    lock_timeout: Option<Duration>,
) -> Result<Option<LockedStages>, ReplaceError> {
    let manager = stage_manager()?.with_lock_timeout(lock_timeout);
    let lock = manager.lock_exclusive()?;
    let entries = match stage_id {
        Some(id) => vec![
            manager
                .read_stage(id)
                .ok_or_else(|| ReplaceError::UnknownStage(format!("unknown stage: {id}")))?,
        ],
        None => manager.list_stages(),
    };
    if entries.is_empty() {
        write_stdout_line(format_args!("No stages yet"))?;
        return Ok(None);
    }
    Ok(Some(LockedStages {
        manager,
        lock,
        entries,
    }))
}
