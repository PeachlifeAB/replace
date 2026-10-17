use std::ffi::OsString;

use camino::Utf8PathBuf;
use clap::{Args, Parser, ValueEnum};

use crate::error::ReplaceError;

use super::rewrite_args::ApplyArgs;
use super::subcommands::Commands;

#[derive(Debug, Parser)]
#[command(
    name = "replace",
    version,
    about = "A safe, preview-first structural rewrite tool",
    override_usage = "replace [OPTIONS] <COMMAND>\n       replace [OPTIONS] FIND REPLACE [PATHS...]",
    after_help = "Shorthand:\n  replace FIND REPLACE [PATHS...]\n\nExamples:\n  replace 'foo.' 'bar.' src/\n  replace 'hello.' 'hi.' --skip-paths\n  replace literal 'foo.' 'bar.' src/"
)]
pub struct Cli {
    #[command(flatten)]
    pub output_flags: OutputFlags,

    /// Preview format for staged changes
    #[arg(long, global = true, value_enum, default_value_t = DiffMode::Unified)]
    pub diff: DiffMode,

    /// Answer yes: rewrites apply right away (like --apply); discard, undo and gc skip the prompt
    #[arg(short = 'y', long, global = true)]
    pub yes: bool,

    /// Timeout for acquiring the stage lock (e.g., "30s", "5m")
    #[arg(long, global = true)]
    pub lock_timeout: Option<String>,

    /// Maximum file size to process (e.g., "5MiB", "100KiB", "1024B")
    #[arg(long, global = true)]
    pub max_file_size: Option<String>,

    /// Maximum number of files to scan
    #[arg(long, global = true)]
    pub max_files: Option<usize>,

    /// Maximum number of matches across the whole run
    #[arg(long, global = true)]
    pub max_matches: Option<usize>,

    /// Maximum number of matches per file
    #[arg(short = 'm', long, global = true)]
    pub max_matches_per_file: Option<usize>,

    /// Use only this `config.toml`, ignoring the local `.replace/` and global configs
    #[arg(long, global = true, value_name = "FILE")]
    pub config_path: Option<Utf8PathBuf>,

    /// Read NUL-delimited target paths from FILE (`-` for stdin) instead of PATHS
    #[arg(long, global = true, value_name = "FILE")]
    pub files0_from: Option<String>,

    #[command(flatten)]
    pub walk_flags: WalkFlags,

    #[command(flatten)]
    pub exit_policy: ExitPolicy,

    #[command(flatten)]
    pub match_flags: MatchFlags,

    #[command(flatten)]
    pub apply_flags: ApplyArgs,

    /// Only scan files matching this glob (repeatable)
    #[arg(long = "include", global = true)]
    pub include: Vec<String>,

    /// Skip files matching this glob (repeatable)
    #[arg(long = "exclude", short = 'g', global = true)]
    pub exclude: Vec<String>,

    #[command(subcommand)]
    pub command: Commands,
}

/// How staged changes are previewed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum DiffMode {
    /// Print a unified diff of every staged change.
    #[default]
    Unified,
    /// Print the diff with an anchor line before each changed line, for `--only-anchors`.
    Anchored,
    /// Print only the staged summary lines.
    None,
}

impl DiffMode {
    /// The diff text this mode prints for `patch`, or nothing for summary-only output.
    #[must_use]
    pub fn render(self, patch: &str, file: &str) -> Option<String> {
        match self {
            Self::Unified => Some(patch.to_owned()),
            Self::Anchored => Some(crate::anchor::add_anchors_to_diff(patch, file)),
            Self::None => None,
        }
    }
}

/// What is printed and how much of it.
#[derive(Debug, Clone, Copy, Args)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent command-line switches"
)]
pub struct OutputFlags {
    /// Output results as JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Output summary statistics
    #[arg(long, global = true)]
    pub stats: bool,

    /// Print the whole preview: no folding, no cut lines
    #[arg(long, global = true)]
    pub expand: bool,
}

/// How the pattern is matched.
#[derive(Debug, Clone, Copy, Args)]
pub struct MatchFlags {
    /// Match letters regardless of case (literal and regex)
    #[arg(short = 'i', long, global = true)]
    pub ignore_case: bool,
    /// Leave matching file and folder names alone (literal and regex rename them by default)
    #[arg(long, global = true)]
    pub skip_paths: bool,
}

/// How warnings affect the exit code.
#[derive(Debug, Clone, Copy, Args)]
pub struct ExitPolicy {
    /// Exit 1 when a run reported any warning (skipped files, zero matches)
    #[arg(long, global = true)]
    pub strict: bool,
}

/// Switches that widen which files the walker visits.
#[derive(Debug, Clone, Copy, Args)]
pub struct WalkFlags {
    /// Also scan hidden files (never descends into .git)
    #[arg(long, global = true)]
    pub hidden: bool,

    /// Do not honor .gitignore or .ignore files
    #[arg(long, global = true)]
    pub no_ignore: bool,

    /// Also scan dependency folders such as `node_modules`
    #[arg(long, global = true)]
    pub no_default_ignores: bool,
}

impl Cli {
    /// Parse the process arguments, reading `replace FIND REPLACE ...` as `replace literal ...`.
    ///
    /// `--help` and `--version` print and exit here.
    ///
    /// # Errors
    ///
    /// Returns [`ReplaceError::InvalidArgs`] when the arguments do not parse.
    pub fn parse_args() -> Result<Self, ReplaceError> {
        Self::parse_args_from(std::env::args_os())
    }

    /// Parse `args`, reading the shorthand form as the `literal` subcommand so it takes every
    /// flag `literal` does, in any position, and honors `--`.
    ///
    /// # Errors
    ///
    /// Returns [`ReplaceError::InvalidArgs`] when the arguments do not parse.
    pub fn parse_args_from<I, T>(args: I) -> Result<Self, ReplaceError>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let words: Vec<OsString> = args.into_iter().map(Into::into).collect();
        let parsed = match Self::try_parse_from(&words) {
            Ok(cli) => cli,
            // `replace --apply FIND REPLACE`: rewrite flags before FIND belong to `literal`.
            Err(error) if error.kind() == clap::error::ErrorKind::UnknownArgument => {
                return Self::try_parse_from(with_literal_at(&words, 1))
                    .map_err(|_| invalid_args(&error));
            }
            Err(error) => return Err(invalid_args(&error)),
        };
        let Commands::ShorthandLiteral(shorthand) = &parsed.command else {
            return Ok(parsed);
        };
        if let [word] = shorthand.as_slice() {
            return Err(ReplaceError::InvalidArgs(format!(
                "unknown command `{}`; to replace text: replace FIND REPLACE [PATHS...] \
                 (replace --help lists commands)",
                word.to_string_lossy()
            )));
        }
        // The external subcommand captures the tail of words verbatim, FIND first; a `--` just
        // before it stays in front of FIND, after `literal`.
        let mut at = words.len() - shorthand.len();
        if at > 1 && words[at - 1] == "--" {
            at -= 1;
        }
        Self::try_parse_from(with_literal_at(&words, at)).map_err(|error| invalid_args(&error))
    }
}

/// `words` with the `literal` subcommand inserted at index `at`.
fn with_literal_at(words: &[OsString], at: usize) -> Vec<OsString> {
    let mut literal = words[..at].to_vec();
    literal.push("literal".into());
    literal.extend_from_slice(&words[at..]);
    literal
}

/// A parse failure as an invalid-arguments error; help and version requests print and exit.
fn invalid_args(error: &clap::Error) -> ReplaceError {
    use clap::error::ErrorKind;
    if matches!(
        error.kind(),
        ErrorKind::DisplayHelp
            | ErrorKind::DisplayVersion
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    ) {
        error.exit();
    }
    let rendered = error.render().to_string();
    let message = rendered.strip_prefix("error: ").unwrap_or(&rendered);
    ReplaceError::InvalidArgs(message.trim_end().to_owned())
}
