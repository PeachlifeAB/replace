//! Plain-language notes for a person about what a rewrite left out. Programs read warnings instead.
use std::collections::HashSet;

use camino::{Utf8Path, Utf8PathBuf};

use super::super::targets::collect_targets_with_caps;
use super::{FileRewrite, RewriteRun};
use crate::error::ReplaceError;
use crate::walk::WalkOptions;
use crate::warning::Warnings;

/// How many skipped files a zero-match run reads to look for matches it missed.
const PROBE_FILE_BUDGET: usize = 2_000;

impl RewriteRun<'_> {
    pub(super) fn print_hints(&self) {
        if !self.outdated_stages.is_empty() {
            let count = self.outdated_stages.len();
            let noun = if count == 1 { "change" } else { "changes" };
            eprintln!("Note: discarded {count} outdated staged {noun} for files this run rewrote");
        }
        if self.options.output.json {
            return;
        }
        if !self.options.rename_files && self.renamable_names > 0 {
            let (count, noun) = match self.renamable_names {
                1 => (1, "file or folder name also matches"),
                many => (many, "file or folder names also match"),
            };
            eprintln!("Hint: {count} {noun}; drop --skip-paths to rename them");
        }
        if let Some(flags) = self.scope_hint {
            eprintln!(
                "Hint: skipped hidden or git-ignored files match; add {flags} to include them"
            );
        }
    }

    /// When nothing matched, look in the files the walk skipped (hidden, git-ignored) for a
    /// match, and name the flags that would include it. Reads at most [`PROBE_FILE_BUDGET`] files.
    pub(super) fn probe_skipped_files<F>(
        &self,
        visited: &[Utf8PathBuf],
        rewrite_file: &mut F,
    ) -> Option<&'static str>
    where
        F: FnMut(&Utf8PathBuf, &str) -> Result<FileRewrite, ReplaceError>,
    {
        let walk = &self.options.walk_options;
        if walk.include_hidden && walk.no_ignore {
            return None;
        }
        let roots = self.searched_folders();
        if roots.is_empty() {
            return None;
        }
        let visited: HashSet<&Utf8PathBuf> = visited.iter().collect();
        let widest = files_under(&roots, walk, true, true);
        let found = widest
            .iter()
            .filter(|path| !visited.contains(path))
            .take(PROBE_FILE_BUDGET)
            .find(|path| matches_in(path, rewrite_file))?;

        let reached_with =
            |hidden, no_ignore| files_under(&roots, walk, hidden, no_ignore).contains(found);
        Some(
            if !walk.include_hidden && reached_with(true, walk.no_ignore) {
                "--hidden"
            } else if !walk.no_ignore && reached_with(walk.include_hidden, true) {
                "--no-ignore"
            } else {
                "--hidden --no-ignore"
            },
        )
    }

    fn searched_folders(&self) -> Vec<Utf8PathBuf> {
        if self.job.paths.is_empty() {
            return vec![Utf8PathBuf::from(".")];
        }
        self.job
            .paths
            .iter()
            .filter(|path| path.is_dir())
            .cloned()
            .collect()
    }
}

/// The files a walk of `roots` visits with the given hidden and ignore switches; no file cap.
fn files_under(
    roots: &[Utf8PathBuf],
    walk: &WalkOptions,
    include_hidden: bool,
    no_ignore: bool,
) -> Vec<Utf8PathBuf> {
    let options = WalkOptions {
        include_hidden,
        no_ignore,
        max_files: None,
        ..walk.clone()
    };
    collect_targets_with_caps(roots, &options, &mut Warnings::default()).unwrap_or_default()
}

/// Whether the rewrite would change this text file; unreadable or binary files never match.
fn matches_in<F>(path: &Utf8Path, rewrite_file: &mut F) -> bool
where
    F: FnMut(&Utf8PathBuf, &str) -> Result<FileRewrite, ReplaceError>,
{
    std::fs::read_to_string(path)
        .ok()
        .filter(|text| !text.contains('\0'))
        .and_then(|text| rewrite_file(&path.to_path_buf(), &text).ok())
        .is_some_and(|rewritten| rewritten.replacements > 0)
}
