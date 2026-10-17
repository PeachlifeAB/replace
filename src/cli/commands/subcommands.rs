use std::ffi::OsString;

use clap::Subcommand;

use super::rewrite_args::{AstArgs, TextRewriteArgs};

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Search and replace using AST patterns
    #[command(about = "Search and replace using AST patterns")]
    Ast(AstArgs),

    /// Search and replace using literal text
    #[command(about = "Search and replace using literal text")]
    Literal {
        /// Text to find
        find: String,
        /// Replacement text
        replace: String,
        #[command(flatten)]
        args: TextRewriteArgs,
    },

    /// Search and replace using regex
    #[command(about = "Search and replace using regex patterns")]
    Regex {
        /// Regex pattern to find
        pattern: String,
        /// Replacement string
        replace: String,
        #[command(flatten)]
        args: TextRewriteArgs,
    },

    /// Show staged changes
    #[command(about = "Show staged changes")]
    Show {
        /// Stage ID to show
        stage_id: Option<String>,
    },

    /// List all staged changes
    #[command(about = "List all staged changes")]
    List,

    /// Apply staged changes
    #[command(about = "Apply staged changes")]
    Apply {
        /// Stage ID to apply (applies all if omitted)
        stage_id: Option<String>,
        /// Apply only if these exact anchors still exist (comma-separated)
        #[arg(long, value_delimiter = ',')]
        expect_anchors: Vec<String>,
        /// Apply only these anchors (comma-separated)
        #[arg(long, value_delimiter = ',')]
        only_anchors: Vec<String>,
        /// Skip these anchors during apply (comma-separated)
        #[arg(long, value_delimiter = ',')]
        skip_anchors: Vec<String>,
    },

    /// Discard staged changes
    #[command(about = "Discard staged changes")]
    Discard {
        /// Stage ID to discard (discards all if omitted)
        stage_id: Option<String>,
    },

    /// Undo applied changes
    #[command(about = "Undo applied changes")]
    Undo {
        /// Stage ID to undo
        stage_id: String,
    },

    /// Show status of replace tool
    #[command(about = "Show status of replace tool")]
    Status,

    /// Show replace log
    #[command(about = "Show replace log")]
    Log,

    /// Garbage collect old stages
    #[command(about = "Garbage collect old stages")]
    Gc {
        /// Minimum age in days for staged entries to be collected
        #[arg(long, default_value_t = 14)]
        age: u64,
    },

    /// Run diagnostics
    #[command(about = "Run diagnostics")]
    Doctor,

    /// List supported languages
    #[command(about = "List supported languages")]
    Languages,

    /// Check staged changes without applying
    #[command(about = "Check staged changes without applying")]
    Check,

    /// Stage a review candidate for the file named in a compiler diagnostic
    #[command(about = "Stage a review candidate for the file named in a diagnostic")]
    FromError {
        /// Diagnostic text, or a file containing it; must name a file (`path[:line[:col]]`)
        message: String,
    },

    /// Explain a stage
    #[command(about = "Explain a stage")]
    Explain {
        /// Stage ID to explain
        stage_id: String,
    },

    /// Load config and run
    #[command(about = "Load config and run")]
    Run,

    /// Shorthand literal replacement: replace FIND REPLACE [PATHS...]; `Cli::parse_args`
    /// re-reads it as `replace literal FIND REPLACE [PATHS...]`.
    #[command(external_subcommand)]
    ShorthandLiteral(Vec<OsString>),
}
