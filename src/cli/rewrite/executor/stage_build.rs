//! Stage entries for content rewrites and file and folder renames, built from the engine's output.
use camino::{Utf8Path, Utf8PathBuf};
use regex::{NoExpand, Regex, RegexBuilder};

use super::file_commit::DirectChange;
use super::{FileRewrite, RewriteJob, RewriteRun};
use crate::apply::{is_plain_file_name, is_same_file};
use crate::cli::util::{StageDraft, build_stage_entry, source_sha256};
use crate::error::ReplaceError;
use crate::patch::unified_diff;
use crate::stage::StageMode;

/// A file rename planned by a file-name rewrite.
struct RenamePlan {
    target: Utf8PathBuf,
    diff: String,
}

impl RewriteRun<'_> {
    pub(super) fn stage_content_change(
        &mut self,
        path: &Utf8PathBuf,
        original: &str,
        rewritten: FileRewrite,
    ) -> Result<(), ReplaceError> {
        self.ensure_replacement_budget(rewritten.replacements)?;

        let diff = unified_diff(original, &rewritten.output);
        let mut entry = build_stage_entry(
            &self.context,
            StageDraft {
                mode: self.job.mode,
                pattern: self.job.pattern,
                rewrite: self.job.rewrite,
                file: path.clone(),
                match_count: rewritten.replacements,
                source_sha: Some(source_sha256(original.as_bytes())),
                patch: &diff,
            },
        );
        entry.ignore_case = self.job.ignore_case;
        entry.language = rewritten.language;
        if let Some(first_match) = rewritten.first_match {
            entry.captures = first_match.captures;
            entry.parent_function = first_match.parent_function;
            entry.parent_class = first_match.parent_class;
        }
        if let Some(ast_options) = &self.options.ast_options {
            entry.selector.clone_from(&ast_options.selector);
            entry.strictness.clone_from(&ast_options.strictness);
            entry.ast_rule_yaml.clone_from(&ast_options.rule_yaml);
        }

        self.write_backup(path, original);
        self.commit(
            entry,
            &diff,
            DirectChange {
                new_content: Some(&rewritten.output),
                original: Some(original),
            },
        )?;
        self.replacements += rewritten.replacements;
        Ok(())
    }

    /// Whether a rename of this path's name would be staged if renames were on.
    pub(super) fn name_would_be_renamed(&self, path: &Utf8Path) -> bool {
        self.names.as_ref().is_some_and(|names| {
            path.file_name()
                .is_some_and(|name| names.rewrite(name).is_some())
        })
    }

    /// Stage (or apply) a rename of `path` when the pattern matches its name.
    pub(super) fn stage_rename(&mut self, path: &Utf8PathBuf) -> Result<(), ReplaceError> {
        let Some(plan) = self.plan_rename(path)? else {
            return Ok(());
        };
        let entry = build_stage_entry(
            &self.context,
            StageDraft {
                mode: StageMode::RenameFile,
                pattern: self.job.pattern,
                rewrite: plan.target.as_str(),
                file: path.clone(),
                match_count: 1,
                source_sha: None,
                patch: &plan.diff,
            },
        );
        self.commit(
            entry,
            &plan.diff,
            DirectChange {
                new_content: None,
                original: None,
            },
        )?;
        self.replacements += 1;
        Ok(())
    }

    fn plan_rename(&mut self, path: &Utf8PathBuf) -> Result<Option<RenamePlan>, ReplaceError> {
        let Some(names) = &self.names else {
            return Ok(None);
        };
        let file_name = path
            .file_name()
            .ok_or_else(|| ReplaceError::InvalidArgs(format!("path has no file name: {path}")))?;
        let Some(new_name) = names.rewrite(file_name) else {
            return Ok(None);
        };
        if !is_plain_file_name(&new_name) {
            return Err(ReplaceError::InvalidArgs(format!(
                "rename of {path} produced an invalid file name: {new_name:?}"
            )));
        }

        let parent = path
            .parent()
            .ok_or_else(|| ReplaceError::InvalidArgs(format!("path has no parent dir: {path}")))?;
        let target = parent.join(&new_name);
        if let Some(other) = self.rename_targets.insert(target.clone(), path.clone()) {
            return Err(ReplaceError::InvalidArgs(format!(
                "{other} and {path} would both be renamed to {target}"
            )));
        }
        // A case-only rename on a case-insensitive file system finds the file itself there.
        if target.exists() && !is_same_file(path, &target) {
            return Err(ReplaceError::InvalidArgs(format!(
                "rename target already exists: {target}"
            )));
        }

        let diff = format!("rename {path}\n    -> {target}\n");
        Ok(Some(RenamePlan { target, diff }))
    }
}

/// How the pattern rewrites file and folder names, compiled once per run.
pub(super) struct NameRewriter {
    regex: Regex,
    replacement: String,
    /// Literal replacements are inserted as-is; regex ones expand `$1` and `${name}`.
    literal: bool,
}

impl NameRewriter {
    /// The rewriter for a literal or regex job; other modes never rename.
    pub(super) fn for_job(job: &RewriteJob<'_>) -> Result<Option<Self>, ReplaceError> {
        let (pattern, literal) = match job.mode {
            StageMode::Literal => (regex::escape(job.pattern), true),
            StageMode::Regex => (job.pattern.to_owned(), false),
            _ => return Ok(None),
        };
        let regex = RegexBuilder::new(&pattern)
            .case_insensitive(job.ignore_case)
            .build()?;
        Ok(Some(Self {
            regex,
            replacement: job.rewrite.to_owned(),
            literal,
        }))
    }

    /// The rewritten name, or `None` when the name does not change.
    fn rewrite(&self, name: &str) -> Option<String> {
        let new_name = if self.literal {
            self.regex
                .replace_all(name, NoExpand(&self.replacement))
                .into_owned()
        } else {
            self.regex
                .replace_all(name, self.replacement.as_str())
                .into_owned()
        };
        (new_name != name).then_some(new_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(mode: StageMode, pattern: &str, rewrite: &str, ignore_case: bool) -> NameRewriter {
        let job = RewriteJob {
            mode,
            pattern,
            rewrite,
            paths: &[],
            ignore_case,
        };
        NameRewriter::for_job(&job)
            .expect("valid pattern")
            .expect("text modes rename")
    }

    #[test]
    fn literal_names_insert_the_replacement_verbatim() {
        let names = names(StageMode::Literal, "foo.", "$1", false);
        assert_eq!(names.rewrite("foo.txt").as_deref(), Some("$1txt"));
        assert_eq!(names.rewrite("fooXtxt"), None);
    }

    #[test]
    fn ignore_case_applies_to_names() {
        let names = names(StageMode::Literal, "foo", "bar", true);
        assert_eq!(names.rewrite("Foo.txt").as_deref(), Some("bar.txt"));
    }

    #[test]
    fn regex_names_expand_groups() {
        let names = names(StageMode::Regex, r"(\w+)_old", "${1}_new", false);
        assert_eq!(names.rewrite("a_old.rs").as_deref(), Some("a_new.rs"));
    }
}
