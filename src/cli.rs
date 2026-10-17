use crate::error::ReplaceError;

mod commands;
mod conclude;
mod diagnostics;
mod hooks;
mod maintenance;
mod options;
mod path_list;
mod preview;
mod prompt;
mod rewrite;
mod rewrite_commands;
mod run;
mod settings;
mod stages;
mod util;

pub use commands::{Cli, Commands, DiffMode};
pub(crate) use options::{AstStageOptions, OutputOptions, RewriteOptions, StagePolicy};

impl Cli {
    /// Run the parsed CLI command.
    ///
    /// # Errors
    ///
    /// Returns an error if the dispatched subcommand fails (staging, apply,
    /// undo, config run, diagnostics, etc.).
    pub fn execute(&self) -> Result<(), ReplaceError> {
        let output = self.output_options();
        let settings = self.resolve_settings()?;
        if let Some(legacy_file) = &settings.ignored_legacy_file {
            eprintln!(
                "hint: {} is no longer read; move it to {}",
                legacy_file.path.display(),
                legacy_file.move_to.display()
            );
        }
        let listed_paths = self
            .files0_from
            .as_deref()
            .map(path_list::read_nul_delimited_paths)
            .transpose()?;
        let context = rewrite_commands::RewriteContext {
            settings: &settings,
            output,
            listed_paths: listed_paths.as_deref(),
        };
        match &self.command {
            Commands::Ast(args) => self.execute_ast_command(args, &context),
            Commands::Literal {
                find,
                replace,
                args,
            } => self.execute_text_command(find, replace, args, rewrite::execute_literal, &context),
            Commands::Regex {
                pattern,
                replace,
                args,
            } => {
                self.execute_text_command(pattern, replace, args, rewrite::execute_regex, &context)
            }
            Commands::Show { stage_id } => stages::execute_show(stage_id.as_deref(), output),
            Commands::List => stages::execute_list(output),
            Commands::Apply {
                stage_id,
                expect_anchors,
                only_anchors,
                skip_anchors,
            } => {
                let selection = stages::AnchorSelection::from_flags(
                    expect_anchors,
                    only_anchors,
                    skip_anchors,
                )?;
                stages::execute_apply(
                    stage_id.as_deref(),
                    &selection,
                    settings.limits.lock_timeout,
                )
            }
            Commands::Discard { stage_id } => {
                Self::confirm_destructive("discard", self.yes)?;
                let lock_timeout = settings.limits.lock_timeout;
                stages::execute_discard(stage_id.as_deref(), lock_timeout)
            }
            Commands::Undo { stage_id } => {
                Self::confirm_destructive("undo", self.yes)?;
                let lock_timeout = settings.limits.lock_timeout;
                Self::execute_undo(stage_id, output, lock_timeout)
            }
            Commands::Status => Self::execute_status(output),
            Commands::Log => stages::execute_log(output),
            Commands::Gc { age } => {
                Self::confirm_destructive("gc", self.yes)?;
                let lock_timeout = settings.limits.lock_timeout;
                stages::execute_gc(*age, lock_timeout)
            }
            Commands::Doctor => Self::execute_doctor(output),
            Commands::Languages => Self::execute_languages(output),
            Commands::Check => stages::execute_check(output),
            Commands::FromError { message } => {
                diagnostics::execute_from_error(message, output, &settings.config)
            }
            Commands::Explain { stage_id } => stages::execute_explain(stage_id, output),
            Commands::Run => run::execute_run(
                &settings.config,
                output,
                &settings.walk_options(),
                self.exit_policy.strict,
                !self.match_flags.skip_paths,
            ),
            Commands::ShorthandLiteral(_) => Err(ReplaceError::InvalidArgs(
                "parse with Cli::parse_args so FIND REPLACE runs as `replace literal`".to_owned(),
            )),
        }
    }
}
