//! What a rewrite prints: staged previews, applied lines, and the closing summary.
use super::super::OutputOptions;
use super::super::preview::condense;
use super::super::util::write_stdout_line;
use super::direct::DirectApplyLog;
use crate::error::ReplaceError;
use crate::stage::{StageEntry, StageMode};
use crate::warning::Warnings;
use camino::Utf8Path;

/// Machine-readable records collected during a run, printed once at the end.
#[derive(Default)]
pub struct JsonRecords(Vec<serde_json::Value>);

impl JsonRecords {
    pub fn record_staged(&mut self, entry: &StageEntry) {
        self.0.push(serde_json::json!({
            "id": entry.id.as_str(),
            "file": entry.file,
            "mode": entry.mode,
            "pattern": entry.pattern,
            "rewrite": entry.rewrite,
            "replacements": entry.match_count,
            "anchors": entry.anchors,
            "applied": false,
            "captures": entry.captures,
            "parent_function": entry.parent_function.as_deref(),
            "parent_class": entry.parent_class.as_deref(),
            "injected_language": entry.injected_language.as_deref(),
            "git_blame_author": entry.git_blame_author.as_deref(),
        }));
    }

    pub(super) fn record_applied(&mut self, entry: &StageEntry, applied_file: &Utf8Path) {
        self.0.push(serde_json::json!({
            "id": entry.id.as_str(),
            "file": applied_file,
            "mode": entry.mode,
            "pattern": entry.pattern,
            "rewrite": entry.rewrite,
            "replacements": entry.match_count,
            "applied": true,
        }));
    }
}

/// Print the preview of a stage that was just written.
pub(super) fn print_staged(
    entry: &StageEntry,
    patch: &str,
    output: OutputOptions,
) -> Result<(), ReplaceError> {
    if let Some(rendered) = output.diff.render(patch, entry.file.as_str()) {
        if output.expand {
            write_stdout_line(format_args!("{rendered}"))?;
        } else {
            let condensed = condense(&rendered, entry.id.as_str());
            write_stdout_line(format_args!("{}", condensed.trim_end_matches('\n')))?;
        }
    }
    if entry.mode == StageMode::RenameFile {
        return write_stdout_line(format_args!(
            "Staged: {} {} -> {} (rename)",
            entry.id.as_str(),
            entry.file,
            entry.rewrite
        ));
    }
    write_stdout_line(format_args!(
        "Staged: {} {} ({} replacements)",
        entry.id.as_str(),
        entry.file,
        entry.match_count
    ))
}

/// Tell a person how to review, apply or drop what a run left staged, once per run.
pub fn print_next_steps(stage_ids: &[String]) -> Result<(), ReplaceError> {
    match stage_ids {
        [] => Ok(()),
        [id] => write_stdout_line(format_args!(
            "Next: replace show {id}\n      replace apply {id}\n      replace discard {id}"
        )),
        many => write_stdout_line(format_args!(
            "Next: replace show       (review all {} staged changes)\n      replace apply      (apply them)\n      replace discard    (drop them)",
            many.len()
        )),
    }
}

/// Hold back an "Applied" line until every direct write has succeeded.
pub(super) fn queue_applied(entry: &StageEntry, applied_file: &Utf8Path, log: &mut DirectApplyLog) {
    log.lines.push(format!(
        "Applied: {} {} ({} replacements)",
        entry.id.as_str(),
        applied_file,
        entry.match_count
    ));
}

/// Totals for the closing summary.
pub struct RunTotals {
    pub files: usize,
    pub replacements: usize,
    pub renames: usize,
}

pub fn print_summary(
    output: OutputOptions,
    totals: &RunTotals,
    records: &JsonRecords,
    warnings: &Warnings,
) -> Result<(), ReplaceError> {
    let RunTotals {
        files,
        replacements,
        renames,
    } = totals;
    if output.json {
        write_stdout_line(format_args!(
            "{}",
            serde_json::json!({
                "stages": records.0,
                "stats": { "files": files, "replacements": replacements, "renames": renames },
                "warnings": warnings.as_slice()
            })
        ))
    } else if *files == 0 {
        write_stdout_line(format_args!("No matches"))
    } else {
        if *renames > 0 {
            write_stdout_line(format_args!(
                "Renames: {renames} (--skip-paths keeps names)"
            ))?;
        }
        if output.stats {
            let renamed = if *renames > 0 {
                format!(", {renames} renames")
            } else {
                String::new()
            };
            write_stdout_line(format_args!(
                "Stats: {files} files, {replacements} replacements{renamed}"
            ))?;
        }
        Ok(())
    }
}
