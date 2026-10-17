//! The `ast`, `literal` and `regex` commands: resolve their targets, run the rewrite, conclude.
use camino::Utf8PathBuf;

use super::commands::{AstArgs, Cli, TextRewriteArgs};
use super::path_list::with_targets;
use super::rewrite::{self, RewriteOutcome, TextRewrite};
use super::settings::ResolvedSettings;
use super::{OutputOptions, RewriteOptions};
use crate::error::ReplaceError;

pub(super) struct RewriteContext<'a> {
    pub(super) settings: &'a ResolvedSettings,
    pub(super) output: OutputOptions,
    pub(super) listed_paths: Option<&'a [Utf8PathBuf]>,
}

type TextExecutor = fn(&TextRewrite<'_>, RewriteOptions) -> Result<RewriteOutcome, ReplaceError>;

impl Cli {
    pub(super) fn execute_ast_command(
        &self,
        args: &AstArgs,
        context: &RewriteContext<'_>,
    ) -> Result<(), ReplaceError> {
        let RewriteContext {
            settings,
            output,
            listed_paths,
        } = *context;
        let source = match (&args.flags.rule, &args.pattern_file, &args.rewrite_file) {
            (Some(_), _, _) => rewrite::PatternSource::RuleFile,
            (None, Some(pattern), Some(rewrite)) => {
                rewrite::PatternSource::Files { pattern, rewrite }
            }
            _ => rewrite::PatternSource::Positional,
        };
        let inputs = rewrite::resolve_ast_inputs(
            &source,
            args.pattern.as_deref(),
            args.rewrite.as_deref(),
            &args.paths,
        )?;
        with_targets(
            output,
            |outcome| self.conclude_rewrite(&outcome, settings, output),
            listed_paths,
            &inputs.paths,
            |targets| {
                let inputs = rewrite::AstInputs {
                    paths: targets.to_vec(),
                    ..inputs
                };
                let options = settings.rewrite_options(output, &self.apply_flags, self.yes, false);
                rewrite::execute_ast(&settings.config, &args.flags, &inputs, options)
            },
        )
    }

    pub(super) fn execute_text_command(
        &self,
        pattern: &str,
        replacement: &str,
        args: &TextRewriteArgs,
        execute: TextExecutor,
        context: &RewriteContext<'_>,
    ) -> Result<(), ReplaceError> {
        let RewriteContext {
            settings,
            output,
            listed_paths,
        } = *context;
        with_targets(
            output,
            |outcome| self.conclude_rewrite(&outcome, settings, output),
            listed_paths,
            &args.paths,
            |targets| {
                let request = TextRewrite {
                    ignore_case: self.match_flags.ignore_case,
                    pattern,
                    replacement,
                    paths: targets,
                };
                let options = settings.rewrite_options(
                    output,
                    &self.apply_flags,
                    self.yes,
                    !self.match_flags.skip_paths,
                );
                execute(&request, options)
            },
        )
    }
}
