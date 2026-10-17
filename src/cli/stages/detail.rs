use crate::error::ReplaceError;
use crate::stage::StageEntry;
use std::time::Duration;

use super::super::OutputOptions;
use super::super::util::{file_sha256, git_blame_author, stage_manager, write_stdout_line};

pub fn execute_log(output: OutputOptions) -> Result<(), ReplaceError> {
    let manager = stage_manager()?;
    let history = manager.list_history();
    if output.json {
        write_stdout_line(format_args!("{}", serde_json::to_string(&history)?))?;
        return Ok(());
    }
    if history.is_empty() {
        write_stdout_line(format_args!("No history entries"))?;
        return Ok(());
    }

    for entry in history {
        write_stdout_line(format_args!(
            "{}\t{:?}\t{}\t{} replacements\t{}",
            entry.id.as_str(),
            entry.mode,
            entry.file,
            entry.match_count,
            entry.timestamp
        ))?;
    }

    Ok(())
}

pub fn execute_gc(age_days: u64, lock_timeout: Option<Duration>) -> Result<(), ReplaceError> {
    let manager = stage_manager()?.with_lock_timeout(lock_timeout);
    let _lock = manager.lock_exclusive()?;

    let stages = manager.list_stages();
    let now = time::OffsetDateTime::now_utc();
    let threshold = time::Duration::days(i64::try_from(age_days).unwrap_or(i64::MAX));
    let mut removed = 0;
    let mut kept = 0;

    for entry in stages {
        if now - entry.created_at < threshold {
            kept += 1;
            continue;
        }
        let id = entry.id.as_str().to_owned();
        manager.discard_stage(&id)?;
        removed += 1;
    }

    write_stdout_line(format_args!(
        "Garbage collected: {removed} staged entries older than {age_days} days removed ({kept} kept); history preserved"
    ))?;
    Ok(())
}

pub fn execute_check(output: OutputOptions) -> Result<(), ReplaceError> {
    let manager = stage_manager()?;
    let stages = manager.list_stages();
    if stages.is_empty() && !output.json {
        write_stdout_line(format_args!("No stages to check"))?;
        return Ok(());
    }

    let mut drifted = Vec::new();
    let mut clean = 0;
    for entry in stages {
        match drift_reason(&entry)? {
            Some(reason) => drifted.push((entry, reason)),
            None => clean += 1,
        }
    }

    if output.json {
        let drifted: Vec<_> = drifted
            .iter()
            .map(|(entry, reason)| {
                serde_json::json!({
                    "id": entry.id.as_str(),
                    "file": entry.file,
                    "reason": reason.as_str(),
                })
            })
            .collect();
        write_stdout_line(format_args!(
            "{}",
            serde_json::json!({ "clean": clean, "drifted": drifted })
        ))?;
    } else {
        for (entry, reason) in &drifted {
            write_stdout_line(format_args!(
                "DRIFTED: {} {}",
                entry.file,
                reason.describe()
            ))?;
        }
        write_stdout_line(format_args!(
            "Check complete: {clean} clean, {} drifted",
            drifted.len()
        ))?;
    }

    if drifted.is_empty() {
        Ok(())
    } else {
        Err(ReplaceError::StageConflict(format!(
            "{} stages have drifted",
            drifted.len()
        )))
    }
}

enum Drift {
    Deleted,
    Modified,
}

impl Drift {
    const fn as_str(&self) -> &'static str {
        match self {
            Self::Deleted => "deleted",
            Self::Modified => "modified",
        }
    }

    const fn describe(&self) -> &'static str {
        match self {
            Self::Deleted => "file deleted",
            Self::Modified => "file modified since staging",
        }
    }
}

fn drift_reason(entry: &StageEntry) -> Result<Option<Drift>, ReplaceError> {
    if !entry.file.exists() {
        return Ok(Some(Drift::Deleted));
    }
    let Some(source_sha) = &entry.source_sha else {
        return Ok(None);
    };
    Ok((&file_sha256(&entry.file)? != source_sha).then_some(Drift::Modified))
}

pub fn execute_explain(stage_id: &str, output: OutputOptions) -> Result<(), ReplaceError> {
    let manager = stage_manager()?;
    let entry = manager
        .read_stage(stage_id)
        .or_else(|| manager.read_history(stage_id))
        .ok_or_else(|| ReplaceError::UnknownStage(format!("unknown stage: {stage_id}")))?;

    if output.json {
        let mut record = entry;
        if record.git_blame_author.is_none() {
            record.git_blame_author = git_blame_author(&record.file);
        }
        return write_stdout_line(format_args!("{}", serde_json::to_string(&record)?));
    }

    write_stdout_line(format_args!("Stage: {}", entry.id.as_str()))?;
    write_stdout_line(format_args!("Mode: {:?}", entry.mode))?;
    write_stdout_line(format_args!("Pattern: {}", entry.pattern))?;
    write_stdout_line(format_args!("Rewrite: {}", entry.rewrite))?;
    write_stdout_line(format_args!("File: {}", entry.file))?;
    write_stdout_line(format_args!("Matches: {}", entry.match_count))?;
    if let Some(ref sha) = entry.source_sha {
        write_stdout_line(format_args!("Source SHA: {}", short_sha(sha)))?;
    }
    if let Some(ref sha) = entry.after_sha {
        write_stdout_line(format_args!("After SHA: {}", short_sha(sha)))?;
    }
    if let Some(ref lang) = entry.language {
        write_stdout_line(format_args!("Language: {lang}"))?;
    }
    write_stdout_line(format_args!("Timestamp: {}", entry.timestamp))?;
    write_stdout_line(format_args!(
        "Author: {}",
        entry.author.as_deref().unwrap_or("unknown")
    ))?;
    // Staging skips blame for speed except on AST stages; look it up now when missing.
    if let Some(blame) = entry
        .git_blame_author
        .clone()
        .or_else(|| git_blame_author(&entry.file))
    {
        write_stdout_line(format_args!("Last committed by: {blame}"))?;
    }

    Ok(())
}

/// The first 12 hex digits of a hash, or all of it when shorter.
fn short_sha(sha: &str) -> &str {
    sha.get(..12).unwrap_or(sha)
}
