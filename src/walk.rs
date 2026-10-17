use camino::{Utf8Path, Utf8PathBuf};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;

use crate::config::REPLACE_DIR_NAME;
use crate::error::ReplaceError;
use crate::warning::{WarningCode, Warnings};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WalkOptions {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub include_hidden: bool,
    /// Ignore `.gitignore`, `.ignore` and global git excludes.
    pub no_ignore: bool,
    /// Also visit directories skipped by default, such as `node_modules`.
    pub no_default_ignores: bool,
    /// Maximum file size in bytes (default 5MiB)
    pub max_file_size: Option<u64>,
    /// Maximum number of files to scan
    pub max_files: Option<usize>,
    /// Maximum number of matches across the whole run
    pub max_matches: Option<usize>,
    /// Maximum number of matches per file
    pub max_matches_per_file: Option<usize>,
}

/// Default cap for a single file when no `--max-file-size` is configured.
pub(crate) const DEFAULT_MAX_FILE_SIZE_BYTES: u64 = 5 * 1024 * 1024;
const GIT_DIR_NAME: &str = ".git";
const DEFAULT_IGNORED_DIR_NAMES: [&str; 1] = ["node_modules"];

/// Whether a file of `len` bytes fits `max_size`; reports the skip on stderr when it does not.
pub(crate) fn admit_file_size(
    path: &Utf8Path,
    len: u64,
    max_size: u64,
    warnings: &mut Warnings,
) -> bool {
    if len <= max_size {
        return true;
    }
    warnings.record(
        WarningCode::SkippedLargeFile,
        Some(path),
        format!(
            "Skipping large file ({}): {path}",
            bytesize::ByteSize::b(len).display().iec()
        ),
    );
    false
}

/// `.git` and `.replace` hold tool state; `--hidden` must never rewrite them.
fn is_tool_state_dir(name: &std::ffi::OsStr) -> bool {
    name == GIT_DIR_NAME || name == REPLACE_DIR_NAME
}

fn is_default_ignored_dir(name: &std::ffi::OsStr) -> bool {
    DEFAULT_IGNORED_DIR_NAMES
        .iter()
        .any(|ignored| name == *ignored)
}

fn ensure_below_file_cap(collected: usize, max_files: Option<usize>) -> Result<(), ReplaceError> {
    match max_files {
        Some(max_files) if collected >= max_files => Err(ReplaceError::Message(format!(
            "Limit reached: scanned {collected} files (max: {max_files}); narrow paths"
        ))),
        _ => Ok(()),
    }
}

fn build_walker(root: &Utf8Path, options: &WalkOptions) -> WalkBuilder {
    let no_default_ignores = options.no_default_ignores;
    let mut walker = WalkBuilder::new(root);
    walker
        .hidden(!options.include_hidden)
        .sort_by_file_name(Ord::cmp)
        .filter_entry(move |entry| {
            !is_tool_state_dir(entry.file_name())
                && (no_default_ignores || !is_default_ignored_dir(entry.file_name()))
        });
    if options.no_ignore {
        walker
            .ignore(false)
            .git_ignore(false)
            .git_global(false)
            .git_exclude(false)
            .parents(false);
    }
    walker
}

/// Collect all target file paths under `root`, honoring include/exclude globs.
///
/// Respects size, count, and match caps from options.
///
/// # Errors
///
/// Returns an error if traversal fails, a glob is invalid, a path is non-UTF-8,
/// or any cap is exceeded.
pub fn collect_paths(
    root: &Utf8Path,
    options: &WalkOptions,
    warnings: &mut Warnings,
) -> Result<Vec<Utf8PathBuf>, ReplaceError> {
    let include = build_globset(&options.include)?;
    let exclude = build_globset(&options.exclude)?;
    let max_size = options.max_file_size.unwrap_or(DEFAULT_MAX_FILE_SIZE_BYTES);
    let mut paths = Vec::new();

    for entry in build_walker(root, options).build() {
        let entry = entry.map_err(|error| ReplaceError::Message(error.to_string()))?;
        let path = entry.path();

        let match_path = path.strip_prefix(root).unwrap_or(path);
        let wanted = entry.file_type().is_some_and(|kind| kind.is_file())
            && !exclude.is_match(match_path)
            && (include.is_empty() || include.is_match(match_path));
        if !wanted {
            continue;
        }

        let path = Utf8PathBuf::from_path_buf(path.to_path_buf()).map_err(|path| {
            ReplaceError::Message(format!("path is not UTF-8: {}", path.display()))
        })?;
        let fits = std::fs::metadata(path.as_path())
            .is_ok_and(|metadata| admit_file_size(&path, metadata.len(), max_size, warnings));
        if fits {
            ensure_below_file_cap(paths.len(), options.max_files)?;
            paths.push(path);
        }
    }

    Ok(paths)
}

/// Whether a single path matches any of the given exclude globs.
///
/// A leading `./` is stripped before matching so `--exclude 'secrets/**'` matches
/// both `secrets/creds.txt` and `./secrets/creds.txt`.
///
/// # Errors
///
/// Returns an error if a glob pattern is invalid.
pub fn is_excluded(path: &Utf8Path, exclude: &[String]) -> Result<bool, ReplaceError> {
    let exclude = build_globset(exclude)?;
    let normalized = path.as_str().strip_prefix("./").unwrap_or(path.as_str());
    Ok(exclude.is_match(normalized))
}

fn build_globset(patterns: &[String]) -> Result<GlobSet, ReplaceError> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
    }
    Ok(builder.build()?)
}
