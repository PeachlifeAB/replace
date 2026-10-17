//! Option types shared by every rewrite and stage command.
use super::commands::{ApplyArgs, Cli, DiffMode};

#[derive(Clone, Copy)]
pub struct OutputOptions {
    pub(crate) json: bool,
    pub(crate) stats: bool,
    pub(crate) diff: DiffMode,
    pub(crate) expand: bool,
}

#[derive(Clone, Copy)]
pub enum StagePolicy {
    Stage,
    Apply,
}

#[derive(Clone)]
pub struct RewriteOptions {
    pub(crate) stage_policy: StagePolicy,
    pub(crate) rename_files: bool,
    pub(crate) output: OutputOptions,
    pub(crate) walk_options: crate::walk::WalkOptions,
    pub(crate) backup: bool,
    pub(crate) ast_options: Option<AstStageOptions>,
}

#[derive(Clone)]
pub struct AstStageOptions {
    pub(crate) selector: Option<String>,
    pub(crate) strictness: Option<String>,
    pub(crate) rule_yaml: Option<String>,
}

impl ApplyArgs {
    pub(super) const fn stage_policy(&self, yes: bool) -> StagePolicy {
        if self.apply || (yes && !self.stage) {
            StagePolicy::Apply
        } else {
            StagePolicy::Stage
        }
    }
}

impl Cli {
    pub(super) const fn output_options(&self) -> OutputOptions {
        OutputOptions {
            json: self.output_flags.json,
            stats: self.output_flags.stats,
            diff: self.diff,
            expand: self.output_flags.expand,
        }
    }
}
