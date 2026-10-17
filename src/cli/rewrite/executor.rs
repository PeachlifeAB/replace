use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::io;

use camino::{Utf8Path, Utf8PathBuf};

use super::super::RewriteOptions;
use super::super::util::{StageContext, stage_manager, write_stdout_line};
use super::direct::DirectApplyLog;
use super::emit::{JsonRecords, RunTotals, print_summary};
use super::targets::collect_targets_with_caps;
use crate::engine::ast::MatchContext;
use crate::error::ReplaceError;
use crate::stage::{StageEntry, StageManager, StageMode};
use crate::warning::{WarningCode, Warnings};

mod file_commit;
mod hints;
mod stage_build;

use file_commit::PendingStages;
use stage_build::NameRewriter;

/// What to rewrite: the mode, its pattern and replacement, and the paths to visit.
pub(super) struct RewriteJob<'a> {
    pub(super) mode: StageMode,
    pub(super) pattern: &'a str,
    pub(super) rewrite: &'a str,
    pub(super) paths: &'a [Utf8PathBuf],
    pub(super) ignore_case: bool,
}

/// The stages a rewrite run produced: what hooks and callers need to know afterwards.
#[derive(Debug, Default)]
pub struct RewriteOutcome {
    pub(crate) stage_ids: Vec<String>,
    pub(crate) files: Vec<Utf8PathBuf>,
    pub(crate) warnings: Warnings,
    /// The changes were written to the files already, not just staged.
    pub(crate) applied_directly: bool,
    /// Stages that rename a file or folder.
    pub(crate) renames: usize,
}

impl RewriteOutcome {
    pub(super) fn record(&mut self, entry: &StageEntry) {
        self.stage_ids.push(entry.id.as_str().to_owned());
        self.files.push(entry.file.clone());
        if entry.mode == StageMode::RenameFile {
            self.renames += 1;
        }
    }

    /// Files touched, counting a file once however many stages changed it.
    pub(super) fn distinct_files(&self) -> usize {
        self.files.iter().collect::<HashSet<_>>().len()
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.stage_ids.is_empty()
    }

    /// Fail the run after the fact when `--strict` is set and anything was reported.
    pub(crate) fn fail_if_strict(&self, strict: bool) -> Result<(), ReplaceError> {
        if strict && !self.warnings.is_empty() {
            return Err(ReplaceError::Strict(format!(
                "--strict: {} warning(s): {}",
                self.warnings.as_slice().len(),
                self.warnings.codes()
            )));
        }
        Ok(())
    }

    pub(crate) fn merge(&mut self, other: Self) {
        self.stage_ids.extend(other.stage_ids);
        self.files.extend(other.files);
        self.renames += other.renames;
        self.warnings.merge(other.warnings);
    }
}

/// One file's text after the engine ran.
pub(super) struct FileRewrite {
    pub(super) output: String,
    pub(super) replacements: usize,
    pub(super) language: Option<String>,
    pub(super) first_match: Option<MatchContext>,
}

/// Rewrite every target file of `job` through `rewrite_file`, staging or applying the changes.
pub(super) fn execute_rewrite<F>(
    job: &RewriteJob<'_>,
    options: RewriteOptions,
    rewrite_file: F,
) -> Result<RewriteOutcome, ReplaceError>
where
    F: FnMut(&Utf8PathBuf, &str) -> Result<FileRewrite, ReplaceError>,
{
    let mut run = RewriteRun::new(job, options)?;
    if let Err(error) = run.process_all(rewrite_file) {
        run.discard_own_stages();
        return Err(run.direct_log.roll_back(error));
    }
    run.finish()
}

/// State of one rewrite run: counters, collected output and the undo log for direct writes.
struct RewriteRun<'a> {
    job: &'a RewriteJob<'a>,
    options: RewriteOptions,
    manager: StageManager,
    context: StageContext,
    outcome: RewriteOutcome,
    /// How names are rewritten; `None` for modes that never rename.
    names: Option<NameRewriter>,
    /// Rename targets planned so far, each with its source, so two renames never collide.
    rename_targets: HashMap<Utf8PathBuf, Utf8PathBuf>,
    /// Folders below the searched folders that hold visited files: candidates for renames.
    folders: BTreeSet<Utf8PathBuf>,
    renamable_names: usize,
    /// The walk flags that would reach skipped files holding matches, when nothing matched.
    scope_hint: Option<&'static str>,
    replacements: usize,
    records: JsonRecords,
    direct_log: DirectApplyLog,
    /// Changes written directly, with their patches, to record in history once all succeeded.
    applied: Vec<(StageEntry, String)>,
    pending: PendingStages,
    /// Stages this run created (not restaged), removed again if the run fails.
    new_stages: Vec<String>,
    /// Earlier stages of rewritten files that no longer match them, discarded on success.
    outdated_stages: Vec<String>,
}

impl<'a> RewriteRun<'a> {
    fn new(job: &'a RewriteJob<'a>, options: RewriteOptions) -> Result<Self, ReplaceError> {
        let manager = stage_manager()?;
        Ok(Self {
            job,
            options,
            pending: PendingStages::load(&manager),
            manager,
            context: StageContext::current()?,
            names: NameRewriter::for_job(job)?,
            rename_targets: HashMap::new(),
            folders: BTreeSet::new(),
            outcome: RewriteOutcome::default(),
            renamable_names: 0,
            scope_hint: None,
            replacements: 0,
            records: JsonRecords::default(),
            direct_log: DirectApplyLog::default(),
            applied: Vec::new(),
            outdated_stages: Vec::new(),
            new_stages: Vec::new(),
        })
    }

    fn process_all<F>(&mut self, mut rewrite_file: F) -> Result<(), ReplaceError>
    where
        F: FnMut(&Utf8PathBuf, &str) -> Result<FileRewrite, ReplaceError>,
    {
        let targets = collect_targets_with_caps(
            self.job.paths,
            &self.options.walk_options,
            &mut self.outcome.warnings,
        )?;
        for path in &targets {
            self.process_file(path, &mut rewrite_file)?;
            self.note_folders(path);
        }
        self.process_folders()?;
        if self.outcome.is_empty() && !self.options.output.json {
            self.scope_hint = self.probe_skipped_files(&targets, &mut rewrite_file);
        }
        Ok(())
    }

    /// Remember the folders between a searched folder and `path`; the searched folder itself
    /// is the scope of the run and keeps its name.
    fn note_folders(&mut self, path: &Utf8Path) {
        let current_dir = [Utf8PathBuf::from(".")];
        let roots = if self.job.paths.is_empty() {
            &current_dir[..]
        } else {
            self.job.paths
        };
        let Some(root) = roots
            .iter()
            .filter(|root| path.starts_with(root.as_path()) && root.as_path() != path)
            .max_by_key(|root| root.components().count())
        else {
            return;
        };
        for folder in path.ancestors().skip(1) {
            if folder.components().count() <= root.components().count() {
                break;
            }
            self.folders.insert(folder.to_path_buf());
        }
    }

    /// Rename matching folders after everything inside them, deepest first.
    fn process_folders(&mut self) -> Result<(), ReplaceError> {
        let mut folders: Vec<Utf8PathBuf> = std::mem::take(&mut self.folders).into_iter().collect();
        folders.sort_by_key(|folder| std::cmp::Reverse(folder.components().count()));
        for folder in &folders {
            if self.options.rename_files {
                self.stage_rename(folder)?;
            } else if self.name_would_be_renamed(folder) {
                self.renamable_names += 1;
            }
        }
        Ok(())
    }

    fn process_file<F>(
        &mut self,
        path: &Utf8PathBuf,
        rewrite_file: &mut F,
    ) -> Result<(), ReplaceError>
    where
        F: FnMut(&Utf8PathBuf, &str) -> Result<FileRewrite, ReplaceError>,
    {
        self.ensure_match_budget()?;

        if let Some(original) = read_text_file(path, &mut self.outcome.warnings)? {
            let rewritten = rewrite_file(path, &original)?;
            if self.exceeds_per_file_cap(path, rewritten.replacements) {
                return Ok(());
            }
            if rewritten.replacements > 0 && original != rewritten.output {
                self.stage_content_change(path, &original, rewritten)?;
            }
        }

        if self.options.rename_files {
            self.stage_rename(path)?;
        } else if self.name_would_be_renamed(path) {
            self.renamable_names += 1;
        }
        Ok(())
    }

    /// A failed run leaves nothing new staged: drop the stages it created (direct writes roll
    /// back on their own).
    fn discard_own_stages(&self) {
        for id in &self.new_stages {
            if let Err(error) = self.manager.discard_stage(id) {
                eprintln!("warning: could not discard stage {id} of the failed run: {error}");
            }
        }
    }

    fn ensure_match_budget(&self) -> Result<(), ReplaceError> {
        match self.options.walk_options.max_matches {
            Some(max_matches) if self.replacements >= max_matches => {
                Err(ReplaceError::Message(format!(
                    "Limit reached: {} matches found (max: {max_matches}); narrow search",
                    self.replacements
                )))
            }
            _ => Ok(()),
        }
    }

    fn exceeds_per_file_cap(&self, path: &Utf8PathBuf, replacements: usize) -> bool {
        let Some(max_per_file) = self.options.walk_options.max_matches_per_file else {
            return false;
        };
        if replacements <= max_per_file {
            return false;
        }
        eprintln!(
            "Skipping {path}: {replacements} matches exceed per-file limit of {max_per_file}"
        );
        true
    }

    /// AST-only ceiling on total replacements, configured through the environment.
    fn ensure_replacement_budget(&self, replacements: usize) -> Result<(), ReplaceError> {
        match crate::env_vars::ast_max_replacements() {
            Some(max_replacements) if self.replacements + replacements > max_replacements => {
                Err(ReplaceError::Message(format!(
                    "Limit reached: {} total replacements (max: {max_replacements}); narrow search",
                    self.replacements + replacements
                )))
            }
            _ => Ok(()),
        }
    }

    fn finish(mut self) -> Result<RewriteOutcome, ReplaceError> {
        self.settle_stage_state()?;
        for line in &self.direct_log.lines {
            write_stdout_line(format_args!("{line}"))?;
        }
        self.outcome.applied_directly = !self.stages();
        if self.outcome.is_empty() {
            self.outcome.warnings.record(
                WarningCode::ZeroMatches,
                None,
                "no matches found".to_owned(),
            );
        }
        self.print_hints();
        print_summary(
            self.options.output,
            &RunTotals {
                files: self.outcome.distinct_files(),
                replacements: self.replacements - self.outcome.renames,
                renames: self.outcome.renames,
            },
            &self.records,
            &self.outcome.warnings,
        )?;
        Ok(self.outcome)
    }
}

/// The file's text, or `None` (with a warning) for binary and non-UTF-8 files.
fn read_text_file(
    path: &Utf8PathBuf,
    warnings: &mut Warnings,
) -> Result<Option<String>, ReplaceError> {
    match fs::read_to_string(path) {
        Ok(content) if content.contains('\0') => {
            warnings.record(
                WarningCode::SkippedBinary,
                Some(path),
                format!("skipping binary file: {path}"),
            );
            Ok(None)
        }
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == io::ErrorKind::InvalidData => {
            warnings.record(
                WarningCode::SkippedEncoding,
                Some(path),
                format!("skipping non-UTF-8 file: {path}"),
            );
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}
