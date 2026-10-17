use camino::Utf8PathBuf;
use clap::Args;

/// How a rewrite is applied: staged for review, or written now.
#[derive(Debug, Args)]
pub struct ApplyArgs {
    /// Apply changes immediately without replace staging
    #[arg(long, global = true, conflicts_with = "stage")]
    pub apply: bool,
    /// Force replace staging even inside Git/JJ repositories
    #[arg(long, global = true)]
    pub stage: bool,
    /// Create .orig backup files before modification
    #[arg(long, global = true)]
    pub backup: bool,
}

/// Target paths of `literal` and `regex`.
#[derive(Debug, Args)]
pub struct TextRewriteArgs {
    /// Target file or directory
    #[arg(value_name = "PATHS")]
    pub paths: Vec<Utf8PathBuf>,
}

/// How an AST rewrite matches: language, rule file and node selection.
#[derive(Debug, Default, Args)]
pub struct AstFlags {
    /// Language (auto-detected from file extension if omitted)
    #[arg(long)]
    pub lang: Option<String>,
    /// Load ast-grep YAML rule file; all positionals are then paths
    #[arg(long)]
    pub rule: Option<Utf8PathBuf>,
    /// Check again after applying (catches cascading rewrites)
    #[arg(long)]
    pub check_after: bool,
    /// AST node kind to rewrite (alias: --kind, e.g. `lexical_declaration`)
    #[arg(long)]
    pub selector: Option<String>,
    /// AST node kind to rewrite (alias for --selector)
    #[arg(long)]
    pub kind: Option<String>,
    /// Strictness level for ast-grep matching
    #[arg(long, value_parser = ["cst", "smart", "ast", "relaxed", "signature", "template"])]
    pub strictness: Option<String>,
    /// Legacy ast-grep pass-through option; rejected with a migration diagnostic.
    #[arg(long = "sg-arg", hide = true)]
    pub sg_args: Vec<String>,
}

/// Arguments of `replace ast`.
#[derive(Debug, Args)]
pub struct AstArgs {
    /// AST pattern to find
    pub pattern: Option<String>,
    /// Replacement template
    pub rewrite: Option<String>,
    /// Read the pattern from a file (needs --rewrite-file); all positionals are then paths
    #[arg(long, requires = "rewrite_file", conflicts_with = "rule")]
    pub pattern_file: Option<Utf8PathBuf>,
    /// Read the rewrite template from a file (needs --pattern-file)
    #[arg(long, requires = "pattern_file", conflicts_with = "rule")]
    pub rewrite_file: Option<Utf8PathBuf>,
    #[command(flatten)]
    pub flags: AstFlags,
    /// Target file or directory (defaults to current dir)
    #[arg(value_name = "PATHS")]
    pub paths: Vec<Utf8PathBuf>,
}
