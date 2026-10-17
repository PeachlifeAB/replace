//! `replace apply`: which stages and anchors a request is allowed to write.
use std::time::Duration;

use super::super::util::{stage_manager, write_stdout_line};
use super::locked_stages;
use crate::anchor::{self, ChangeSelection};
use crate::apply::{ApplyResult, StageSelection, apply_stage, apply_stages_atomic};
use crate::error::ReplaceError;
use crate::stage::{StageEntry, StageLock, StageManager};

/// Which anchors of the selected stages an apply may write.
pub enum AnchorSelection {
    /// Every anchor of every selected stage.
    All,
    /// Every anchor, after checking that these anchors still exist.
    Expect(Vec<String>),
    /// Only these anchors.
    Only(Vec<String>),
    /// Everything except these anchors.
    Skip(Vec<String>),
}

/// What to do with one selected stage.
enum StagePlan {
    /// Apply these hunks of the stage; the rest stays staged.
    Apply(StageSelection),
    /// Leave the stage staged.
    Skip { notice: String },
}

impl AnchorSelection {
    /// Combine the three anchor flags; at most one may be given.
    ///
    /// # Errors
    ///
    /// Returns an error when two of the flags are combined.
    pub(crate) fn from_flags(
        expect: &[String],
        only: &[String],
        skip: &[String],
    ) -> Result<Self, ReplaceError> {
        match (expect.is_empty(), only.is_empty(), skip.is_empty()) {
            (true, true, true) => Ok(Self::All),
            (false, true, true) => Ok(Self::Expect(expect.to_vec())),
            (true, false, true) => Ok(Self::Only(only.to_vec())),
            (true, true, false) => Ok(Self::Skip(skip.to_vec())),
            (false, false, _) => Err(conflicting("--expect-anchors", "--only-anchors")),
            (false, _, false) => Err(conflicting("--expect-anchors", "--skip-anchors")),
            (_, false, false) => Err(conflicting("--only-anchors", "--skip-anchors")),
        }
    }

    /// Check the requested anchors against the selected stages before anything is written.
    fn validate(&self, entries: &[StageEntry]) -> Result<(), ReplaceError> {
        match self {
            Self::All => Ok(()),
            Self::Expect(expected) => expected
                .iter()
                .try_for_each(|anchor| ensure_expected_anchor(entries, anchor)),
            Self::Only(anchors) | Self::Skip(anchors) => anchors
                .iter()
                .try_for_each(|anchor| ensure_known_anchor(entries, anchor)),
        }
    }

    fn hunks(&self) -> ChangeSelection {
        match self {
            Self::All | Self::Expect(_) => ChangeSelection::All,
            Self::Only(anchors) => ChangeSelection::Only(anchors.clone()),
            Self::Skip(anchors) => ChangeSelection::Except(anchors.clone()),
        }
    }

    fn plan_for(&self, entry: &StageEntry, patch: &str) -> StagePlan {
        let hunks = self.hunks();
        let file = entry.file.as_str();
        let total = anchor::count_changes(patch);
        let selected = anchor::count_changes(&anchor::retain_changes(patch, file, &hunks));
        if selected == 0 && total > 0 {
            let reason = match self {
                Self::Skip(_) => "all anchors skipped",
                _ => "no matching anchors",
            };
            return StagePlan::Skip {
                notice: format!("Skipping: {} {} ({reason})", entry.id.as_str(), entry.file),
            };
        }
        StagePlan::Apply(StageSelection {
            id: entry.id.as_str().to_owned(),
            hunks,
        })
    }
}

fn conflicting(left: &str, right: &str) -> ReplaceError {
    ReplaceError::InvalidArgs(format!("{left} and {right} cannot be combined"))
}

fn holds_anchor(entries: &[StageEntry], anchor: &str) -> bool {
    entries
        .iter()
        .any(|entry| entry.anchors.iter().any(|a| a == anchor))
}

fn ensure_known_anchor(entries: &[StageEntry], anchor: &str) -> Result<(), ReplaceError> {
    if holds_anchor(entries, anchor) {
        Ok(())
    } else {
        Err(ReplaceError::AnchorMismatch(format!(
            "unknown anchor: {anchor}"
        )))
    }
}

fn ensure_expected_anchor(entries: &[StageEntry], anchor: &str) -> Result<(), ReplaceError> {
    if anchor::parse_anchor(anchor).is_none() {
        return Err(ReplaceError::InvalidArgs(format!(
            "invalid anchor format: {anchor} (expected path#hash)"
        )));
    }
    if holds_anchor(entries, anchor) {
        Ok(())
    } else {
        Err(ReplaceError::AnchorMismatch(format!(
            "anchor not found or content changed: {anchor}"
        )))
    }
}

fn print_applied(results: Vec<ApplyResult>) -> Result<(), ReplaceError> {
    for applied in results {
        write_stdout_line(format_args!(
            "Applied: {} {} ({} replacements)",
            applied.stage_id, applied.file, applied.replacements
        ))?;
    }
    Ok(())
}

/// Apply one whole stage directly, or any other selection as an all-or-nothing batch.
fn apply_selections(
    manager: &StageManager,
    lock: &StageLock,
    selections: &[StageSelection],
) -> Result<Vec<ApplyResult>, ReplaceError> {
    match selections {
        [] => Ok(Vec::new()),
        [only] if only.hunks == ChangeSelection::All => {
            Ok(vec![apply_stage(manager, lock, &only.id)?])
        }
        many => apply_stages_atomic(manager, lock, many),
    }
}

/// Apply the staged changes selected by `stage_id` (all when `None`), limited by `selection`.
pub fn execute_apply(
    stage_id: Option<&str>,
    selection: &AnchorSelection,
    lock_timeout: Option<Duration>,
) -> Result<(), ReplaceError> {
    let Some(locked) = locked_stages(stage_id, lock_timeout)? else {
        return Ok(());
    };
    selection.validate(&locked.entries)?;

    let mut selections = Vec::new();
    for entry in &locked.entries {
        let id = entry.id.as_str();
        let patch = locked.manager.read_stage_patch(id).ok_or_else(|| {
            ReplaceError::UnknownStage(format!("patch not found for stage: {id}"))
        })?;
        match selection.plan_for(entry, &patch) {
            StagePlan::Apply(stage) => selections.push(stage),
            StagePlan::Skip { notice } => write_stdout_line(format_args!("{notice}"))?,
        }
    }
    print_applied(apply_selections(
        &locked.manager,
        &locked.lock,
        &selections,
    )?)
}

/// Apply exactly these just-staged changes, as a person answered yes to the prompt.
pub fn execute_apply_staged(
    stage_ids: &[String],
    lock_timeout: Option<Duration>,
) -> Result<(), ReplaceError> {
    let manager = stage_manager()?.with_lock_timeout(lock_timeout);
    let lock = manager.lock_exclusive()?;
    let selections: Vec<StageSelection> = stage_ids.iter().map(StageSelection::whole).collect();
    print_applied(apply_selections(&manager, &lock, &selections)?)
}
