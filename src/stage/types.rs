use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use time::OffsetDateTime;

/// Content-derived 12-hex stage ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageId(pub String);

impl StageId {
    /// Generate a 12-hex content ID from stage bytes (patch + metadata).
    #[must_use]
    pub fn from_stage_bytes(bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        Self(hex::encode(digest)[..12].to_owned())
    }

    /// Generate a 12-hex content ID from a file on disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let bytes = fs::read(path)?;
        Ok(Self::from_stage_bytes(&bytes))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for StageId {
    fn default() -> Self {
        Self::from_stage_bytes(&[])
    }
}

/// Mode of the staged rewrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StageMode {
    Ast,
    Literal,
    Regex,
    FromError,
    RenameFile,
}

/// Full metadata for a staged rewrite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageEntry {
    pub id: StageId,
    pub mode: StageMode,
    pub pattern: String,
    pub rewrite: String,
    /// The primary file matched (first file in the stage).
    pub file: Utf8PathBuf,
    /// Number of matches/replacements.
    pub match_count: usize,
    /// SHA-256 of the source file before any changes.
    pub source_sha: Option<String>,
    /// SHA-256 of the source file after apply (set during apply).
    pub after_sha: Option<String>,
    /// Whether the pattern matched regardless of letter case.
    #[serde(default)]
    pub ignore_case: bool,
    /// Language used for AST rewrites.
    pub language: Option<String>,
    /// Hashline anchors for each match (path#hash format).
    pub anchors: Vec<String>,
    /// AST metavariable captures (e.g. {"$A": "value"}).
    pub captures: Option<serde_json::Value>,
    /// Parent function name when resolvable from AST.
    pub parent_function: Option<String>,
    /// Parent class name when resolvable from AST.
    pub parent_class: Option<String>,
    /// Injected language when match is inside an injected region.
    pub injected_language: Option<String>,
    /// Git blame author when in a git repo.
    pub git_blame_author: Option<String>,
    /// Selector kind for AST rewrites.
    pub selector: Option<String>,
    /// Kind for AST rewrites (alias for selector).
    pub kind: Option<String>,
    /// Strictness level for AST rewrites.
    pub strictness: Option<String>,
    /// Deprecated AST CLI pass-through arguments retained for legacy stages.
    #[serde(default)]
    pub sg_args: Vec<String>,
    /// Native ast-grep YAML rule source for AST rewrites.
    #[serde(default)]
    pub ast_rule_yaml: Option<String>,
    /// Command-line arguments that created this stage.
    pub command_argv: Vec<String>,
    /// Working directory when staged.
    pub cwd: String,
    /// Author (from git config).
    pub author: Option<String>,
    /// UTC timestamp when staged.
    pub timestamp: String,
    /// When the stage was created.
    pub created_at: OffsetDateTime,
}
