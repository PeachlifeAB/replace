use camino::Utf8PathBuf;
use sha2::{Digest, Sha256};

use super::ApplyResult;
use crate::engine::ast::{self, AstRule};
use crate::engine::literal::{self, LiteralRule};
use crate::engine::regex::{self, RegexRule};
use crate::error::ReplaceError;
use crate::stage::{StageEntry, StageManager, StageMode};
use ast_grep_language::SupportLang;

/// Compute SHA-256 of a file.
pub(super) fn file_sha256(path: impl AsRef<std::path::Path>) -> Result<String, ReplaceError> {
    let bytes = std::fs::read(path)?;
    let digest = Sha256::digest(&bytes);
    Ok(hex::encode(digest))
}

/// Check if the source file has drifted since staging.
pub(super) fn check_drift(entry: &StageEntry) -> Result<(), ReplaceError> {
    if let Some(ref source_sha) = entry.source_sha {
        let current_sha = file_sha256(&entry.file)?;
        if &current_sha != source_sha {
            return Err(ReplaceError::AnchorMismatch(
                "source file has drifted since staging".to_owned(),
            ));
        }
    }
    Ok(())
}

/// The content a staged rule produces from `content`: the rule re-run on the file text.
pub(super) fn rewrite_content(entry: &StageEntry, content: &str) -> Result<String, ReplaceError> {
    match entry.mode {
        StageMode::Ast => rewrite_ast(entry, content),
        StageMode::Literal => {
            let rule = LiteralRule {
                find: entry.pattern.clone(),
                replace: entry.rewrite.clone(),
                ignore_case: entry.ignore_case,
            };
            Ok(literal::replace_all(content, &[rule])?.output)
        }
        StageMode::Regex => {
            let rule = RegexRule {
                pattern: entry.pattern.clone(),
                replacement: entry.rewrite.clone(),
                ignore_case: entry.ignore_case,
            };
            Ok(regex::replace_all(content, &rule)?.output)
        }
        StageMode::FromError => Err(ReplaceError::InvalidArgs(
            "from-error stages cannot be applied directly".to_owned(),
        )),
        StageMode::RenameFile => Err(ReplaceError::InvalidArgs(
            "rename stages have no content rewrite".to_owned(),
        )),
    }
}

fn rewrite_ast(entry: &StageEntry, content: &str) -> Result<String, ReplaceError> {
    let lang = if let Some(yaml_rule) = &entry.ast_rule_yaml {
        ast::yaml_rule_language(yaml_rule)?
    } else {
        let lang_name = entry
            .language
            .clone()
            .or_else(|| ast::language_name_for_path(&entry.file))
            .ok_or_else(|| {
                ReplaceError::InvalidArgs(
                    "no language specified for AST stage; restage it".to_owned(),
                )
            })?;
        lang_name
            .as_str()
            .parse::<SupportLang>()
            .map_err(|_| ReplaceError::Ast(format!("unsupported language: {lang_name}")))?
    };
    let rule = AstRule {
        language: lang,
        pattern: entry.pattern.clone(),
        replacement: entry.rewrite.clone(),
        selector: entry.selector.clone().or_else(|| entry.kind.clone()),
        strictness: entry.strictness.clone(),
        yaml_rule: entry.ast_rule_yaml.clone(),
    };
    Ok(ast::replace_all(content, &rule)?.output)
}

pub(super) fn apply_rename_stage_locked(
    manager: &StageManager,
    id: &str,
    mut entry: StageEntry,
) -> Result<ApplyResult, ReplaceError> {
    let target = Utf8PathBuf::from(&entry.rewrite);
    validate_rename(&entry.file, &target)?;
    std::fs::rename(&entry.file, &target)?;
    entry.after_sha = None;
    let patch = manager.read_stage_patch(id).ok_or_else(|| {
        ReplaceError::UnknownStage(format!("patch not found for rename stage: {id}"))
    })?;
    manager.write_stage(&mut entry, &patch, false)?;
    manager.archive_to_history(id)?;
    Ok(ApplyResult {
        stage_id: id.to_owned(),
        file: target,
        replacements: entry.match_count,
    })
}

pub(super) fn validate_rename(
    source: &Utf8PathBuf,
    target: &Utf8PathBuf,
) -> Result<(), ReplaceError> {
    if !source.exists() {
        return Err(ReplaceError::FileDeleted(format!(
            "staged file was deleted: {source}"
        )));
    }
    if std::fs::symlink_metadata(source)?.file_type().is_symlink() {
        return Err(ReplaceError::InvalidArgs(format!(
            "refusing to rename symlink: {source}"
        )));
    }
    if source.parent() != target.parent() || !target.file_name().is_some_and(is_plain_file_name) {
        return Err(ReplaceError::InvalidArgs(format!(
            "rename must stay in its folder under a plain file name: {source} -> {target}"
        )));
    }
    // A case-only rename on a case-insensitive file system finds the source itself there.
    if target.exists() && !is_same_file(source, target) {
        return Err(ReplaceError::InvalidArgs(format!(
            "rename target already exists: {target}"
        )));
    }
    Ok(())
}

/// Whether `name` is one path component that stays in the folder it is created in.
pub fn is_plain_file_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\']) && name != "." && name != ".."
}

/// Whether two paths name the same file or folder on disk.
pub fn is_same_file(left: &Utf8PathBuf, right: &Utf8PathBuf) -> bool {
    same_file::is_same_file(left, right).unwrap_or(false)
}

/// Find candidates for a moved file by searching for content hash matches.
pub(super) fn find_moved_file_candidates(
    entry: &StageEntry,
) -> Result<Vec<Utf8PathBuf>, ReplaceError> {
    if !is_in_git_repo(&entry.file) {
        return Ok(Vec::new());
    }

    let source_sha = match &entry.source_sha {
        Some(sha) => sha.clone(),
        None => return Ok(Vec::new()),
    };

    let output = std::process::Command::new("git")
        .args(["ls-files", "-z"])
        .output()
        .map_err(|e| ReplaceError::Git(format!("failed to run git ls-files: {e}")))?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let file_list = std::str::from_utf8(&output.stdout)
        .map_err(|e| ReplaceError::Git(format!("invalid UTF-8 in git ls-files: {e}")))?;

    let mut candidates = Vec::new();
    for file_path in file_list.split('\0') {
        let file_path = file_path.trim();
        if file_path.is_empty() {
            continue;
        }

        let full_path = Utf8PathBuf::from(file_path);

        if !full_path.exists() {
            continue;
        }

        if full_path == entry.file {
            continue;
        }

        if file_sha256(&full_path).is_ok_and(|current_sha| current_sha == source_sha) {
            candidates.push(full_path);
        }
    }

    Ok(candidates)
}

/// Check if the given path is inside a git repository.
fn is_in_git_repo(path: &Utf8PathBuf) -> bool {
    let mut cursor = if path.is_absolute() {
        path.clone()
    } else {
        Utf8PathBuf::from(".")
    };

    loop {
        if cursor.join(".git").exists() {
            return true;
        }
        if !cursor.pop() {
            return false;
        }
    }
}
