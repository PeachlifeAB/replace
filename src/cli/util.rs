use std::fs;
use std::io::{self, Write};

use ast_grep_language::SupportLang;
use camino::Utf8PathBuf;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

use crate::anchor;
use crate::error::ReplaceError;
use crate::stage::{StageEntry, StageId, StageManager, StageMode};

/// What a stage records before it is hashed and timestamped.
pub struct StageDraft<'a> {
    pub(crate) mode: StageMode,
    pub(crate) pattern: &'a str,
    pub(crate) rewrite: &'a str,
    pub(crate) file: Utf8PathBuf,
    pub(crate) match_count: usize,
    pub(crate) source_sha: Option<String>,
    pub(crate) patch: &'a str,
}

/// What every stage of one run shares: who ran which command, and where. Read once per run.
pub struct StageContext {
    cwd: String,
    command_argv: Vec<String>,
    author: Option<String>,
}

impl StageContext {
    pub(crate) fn current() -> Result<Self, ReplaceError> {
        Ok(Self {
            cwd: current_dir_utf8()?.to_string(),
            command_argv: invocation_argv(),
            author: git_author(),
        })
    }
}

pub fn build_stage_entry(context: &StageContext, draft: StageDraft<'_>) -> StageEntry {
    let StageDraft {
        mode,
        pattern,
        rewrite,
        file,
        match_count,
        source_sha,
        patch,
    } = draft;
    let timestamp = OffsetDateTime::now_utc();
    // Length-prefix each field so distinct entries never collide on a shared
    // separator (e.g. a field containing ':' must not blur the boundary).
    let mut id_buf = Vec::new();
    id_buf.extend_from_slice(format!("{mode:?}").as_bytes());
    for field in [
        file.as_str(),
        pattern,
        rewrite,
        source_sha.as_deref().unwrap_or_default(),
        patch,
    ] {
        id_buf.extend_from_slice(&(field.len() as u64).to_le_bytes());
        id_buf.extend_from_slice(field.as_bytes());
    }

    let anchors = anchor::extract_anchors_from_patch(patch, file.as_str());
    // Blame runs git once per file; only AST stages, which explain code context, pay for it.
    let blame_author = (mode == StageMode::Ast)
        .then(|| git_blame_author(&file))
        .flatten();

    StageEntry {
        id: StageId::from_stage_bytes(&id_buf),
        mode,
        pattern: pattern.to_owned(),
        rewrite: rewrite.to_owned(),
        file,
        match_count,
        source_sha,
        after_sha: None,
        ignore_case: false,
        language: None,
        anchors,
        captures: None,
        parent_function: None,
        parent_class: None,
        injected_language: None,
        git_blame_author: blame_author,
        selector: None,
        kind: None,
        strictness: None,
        sg_args: vec![],
        ast_rule_yaml: None,
        command_argv: context.command_argv.clone(),
        cwd: context.cwd.clone(),
        author: context.author.clone(),
        timestamp: timestamp_rfc3339(timestamp),
        created_at: timestamp,
    }
}

/// Infer the AST language for a file path.
///
/// Priority: CLI --lang (caller's responsibility) > config [`lang_by_glob`] > default extension inference.
pub fn infer_ast_language(
    path: &Utf8PathBuf,
    config: &crate::config::ReplaceConfig,
) -> Option<String> {
    config
        .language_for_path(path)
        .map(str::to_owned)
        .or_else(|| crate::engine::ast::language_name_for_path(path))
}

pub fn parse_ast_language(lang: &str) -> Result<SupportLang, ReplaceError> {
    lang.parse()
        .map_err(|_| ReplaceError::Ast(format!("unsupported language: {lang}")))
}

/// The full command line that produced a stage, program name first.
pub fn invocation_argv() -> Vec<String> {
    std::env::args().collect()
}

/// UTC instant as RFC 3339, the stage sidecar's timestamp format.
pub fn timestamp_rfc3339(at: time::OffsetDateTime) -> String {
    at.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| at.to_string())
}

pub fn stage_manager() -> Result<StageManager, ReplaceError> {
    Ok(StageManager::new(current_dir_utf8()?))
}

pub fn current_dir_utf8() -> Result<Utf8PathBuf, ReplaceError> {
    Utf8PathBuf::from_path_buf(std::env::current_dir()?).map_err(|path| {
        ReplaceError::InvalidArgs(format!(
            "current directory is not UTF-8: {}",
            path.display()
        ))
    })
}

pub fn source_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex::encode(digest)
}

pub fn file_sha256(path: impl AsRef<std::path::Path>) -> Result<String, ReplaceError> {
    let bytes = fs::read(path)?;
    Ok(source_sha256(&bytes))
}

pub fn write_stdout_line(args: std::fmt::Arguments<'_>) -> Result<(), ReplaceError> {
    let mut stdout = io::stdout().lock();
    stdout.write_fmt(args)?;
    stdout.write_all(b"\n")?;
    Ok(())
}

pub fn git_author() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()?;
    let name = std::str::from_utf8(&output.stdout).ok()?;
    let name = name.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_owned())
    }
}

/// Porcelain `git blame` names the author on a line starting with this.
const BLAME_AUTHOR_PREFIX: &str = "author ";

/// Get the git blame author for a file (author of the last commit that touched it).
pub fn git_blame_author(path: &Utf8PathBuf) -> Option<String> {
    if !path_is_versioned(path) {
        return None;
    }

    let output = std::process::Command::new("git")
        .args(["blame", "--porcelain", "-l", "HEAD", "--", path.as_str()])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = std::str::from_utf8(&output.stdout).ok()?;
    stdout
        .lines()
        .find_map(|line| line.strip_prefix(BLAME_AUTHOR_PREFIX))
        .filter(|author| !author.is_empty())
        .map(str::to_owned)
}

pub fn path_is_versioned(path: &Utf8PathBuf) -> bool {
    let start = if path.is_dir() {
        path.clone()
    } else {
        path.parent()
            .map_or_else(|| Utf8PathBuf::from("."), Utf8PathBuf::from)
    };
    let mut cursor = if start.is_absolute() {
        start.into_std_path_buf()
    } else if let Ok(cwd) = std::env::current_dir() {
        cwd.join(start.as_str())
    } else {
        return false;
    };

    loop {
        if cursor.join(".git").exists() || cursor.join(".jj").exists() {
            return true;
        }
        if !cursor.pop() {
            return false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_stage_entry_length_prefixes_stage_id_fields() {
        let file = Utf8PathBuf::from("same.txt");
        let draft = |pattern, rewrite, file| StageDraft {
            mode: StageMode::Literal,
            pattern,
            rewrite,
            file,
            match_count: 1,
            source_sha: Some("sha".to_owned()),
            patch: "patch",
        };
        let context = StageContext::current().expect("context");
        let left = build_stage_entry(&context, draft("a:b", "c", file.clone()));
        let right = build_stage_entry(&context, draft("a", "b:c", file));
        assert_ne!(left.id, right.id);
    }
}
