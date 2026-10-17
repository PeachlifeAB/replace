use std::fs;

use camino::Utf8PathBuf;
use time::OffsetDateTime;

use super::OutputOptions;
use super::rewrite::{JsonRecords, RunTotals, print_summary};
use super::util::{
    current_dir_utf8, git_author, git_blame_author, infer_ast_language, invocation_argv,
    source_sha256, stage_manager, timestamp_rfc3339, write_stdout_line,
};
use crate::error::ReplaceError;
use crate::stage::{StageEntry, StageId, StageMode};
use crate::warning::Warnings;

pub fn execute_from_error(
    message: &str,
    output: OutputOptions,
    config: &crate::config::ReplaceConfig,
) -> Result<(), ReplaceError> {
    let manager = stage_manager()?;
    let looks_like_path = !message.starts_with('{') && !message.starts_with('"');
    let diagnostic = if looks_like_path && fs::metadata(message).is_ok() {
        fs::read_to_string(message)?
    } else {
        message.to_owned()
    };

    let file = extract_file_from_diagnostic(&diagnostic).ok_or_else(|| {
        ReplaceError::InvalidArgs("from-error could not find a file path".to_owned())
    })?;

    if !file.exists() {
        return Err(ReplaceError::InvalidArgs(format!("file not found: {file}")));
    }

    let original = fs::read_to_string(&file)?;
    let source_sha = source_sha256(original.as_bytes());
    let id_input = format!("from-error:{file}:{source_sha}");
    let id = StageId::from_stage_bytes(id_input.as_bytes());
    let now = OffsetDateTime::now_utc();

    let mut entry = StageEntry {
        ignore_case: false,
        id: id.clone(),
        mode: StageMode::FromError,
        pattern: diagnostic.clone(),
        rewrite: String::new(),
        file: file.clone(),
        match_count: 1,
        source_sha: Some(source_sha),
        after_sha: None,
        language: infer_ast_language(&file, config),
        anchors: Vec::new(),
        captures: None,
        parent_function: None,
        parent_class: None,
        injected_language: None,
        git_blame_author: git_blame_author(&file),
        selector: None,
        kind: None,
        strictness: None,
        sg_args: vec![],
        ast_rule_yaml: None,
        command_argv: invocation_argv(),
        cwd: current_dir_utf8()?.to_string(),
        author: git_author(),
        timestamp: timestamp_rfc3339(now),
        created_at: now,
    };

    let patch = format!(
        "from-error candidate for {}\nSource diagnostic:\n{}\nProposed: review and apply manually\nConfidence: low",
        file,
        truncate_str(&diagnostic, 500)
    );

    // The ID is content-derived: the same diagnostic restages in place.
    manager.write_stage(&mut entry, &patch, false)?;

    if output.json {
        let mut records = JsonRecords::default();
        records.record_staged(&entry);
        let totals = RunTotals {
            files: 1,
            replacements: entry.match_count,
            renames: 0,
        };
        return print_summary(output, &totals, &records, &Warnings::default());
    }

    write_stdout_line(format_args!("Staged: {}", id.as_str()))?;
    write_stdout_line(format_args!(
        "Next: replace show {}\n      replace apply {}\n      replace discard {}",
        id.as_str(),
        id.as_str(),
        id.as_str()
    ))?;
    write_stdout_line(format_args!(
        "Source diagnostic:\n{}",
        truncate_str(&diagnostic, 1000)
    ))?;
    write_stdout_line(format_args!("Confidence: low"))?;

    Ok(())
}

fn extract_file_from_diagnostic(diagnostic: &str) -> Option<Utf8PathBuf> {
    for line in diagnostic.lines() {
        let parts: Vec<&str> = line.splitn(4, ':').collect();
        for n in (1..=parts.len().min(3)).rev() {
            let candidate = parts[..n].join(":");
            let path = Utf8PathBuf::from(&candidate);
            if path.is_file() {
                return Some(path);
            }
        }

        if let Some(pos) = line.find("\"file\"") {
            let rest = &line[pos + 6..];
            if let Some((start, end)) = rest
                .find('"')
                .and_then(|start| rest[start + 1..].find('"').map(|end| (start, end)))
            {
                return Some(Utf8PathBuf::from(&rest[start + 1..start + 1 + end]));
            }
        }
    }

    for token in diagnostic.split_whitespace() {
        let cleaned =
            token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && !"/._-".contains(c));
        if let Some(path) = path_without_position(cleaned) {
            return Some(path);
        }
        let token_path =
            token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && !"/._-:".contains(c));
        if let Some(path) = path_without_position(token_path) {
            return Some(path);
        }
    }

    None
}

/// Existing file named by `text`, ignoring trailing `:line[:column]` position suffixes.
fn path_without_position(text: &str) -> Option<Utf8PathBuf> {
    let mut candidate = text;
    loop {
        if candidate.contains('/') {
            let path = Utf8PathBuf::from(candidate);
            if path.is_file() {
                return Some(path);
            }
        }
        let (head, tail) = candidate.rsplit_once(':')?;
        if tail.is_empty() || !tail.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        candidate = head;
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    let total = s.chars().count();
    if total <= max_len {
        s.to_owned()
    } else {
        let truncated: String = s.chars().take(max_len).collect();
        format!("{truncated}... [truncated, {total} chars total]")
    }
}

#[cfg(test)]
mod tests {
    use super::truncate_str;

    #[test]
    fn truncate_str_handles_multibyte_boundaries() {
        assert_eq!(truncate_str("åäö", 2), "åä... [truncated, 3 chars total]");
    }
}
