use std::process::ExitCode;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReplaceError {
    #[error("REPLACE_E_INVALID_ARGS: {0}")]
    InvalidArgs(String),

    #[error("REPLACE_E_CONFIG: {0}")]
    Config(#[from] toml::de::Error),

    #[error("REPLACE_E_IO: {0}")]
    Io(#[from] std::io::Error),

    #[error("REPLACE_E_GLOB: {0}")]
    Glob(#[from] globset::Error),

    #[error("REPLACE_E_REGEX: {0}")]
    Regex(#[from] regex::Error),

    #[error("REPLACE_E_JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("REPLACE_E_LITERAL: {0}")]
    LiteralMatcher(#[from] aho_corasick::BuildError),

    #[error("REPLACE_E_AST: {0}")]
    Ast(String),

    #[error("REPLACE_E_GIT: {0}")]
    Git(String),

    #[error("REPLACE_E_PATCH: {0}")]
    Patch(String),

    #[error("REPLACE_E_LOCK: {0}")]
    Lock(String),

    #[error("REPLACE_E_LOCK_TIMEOUT: {0}")]
    LockTimeout(String),

    #[error("REPLACE_E_UNKNOWN_STAGE: {0}")]
    UnknownStage(String),

    #[error("REPLACE_E_UNKNOWN_HISTORY: {0}")]
    UnknownHistory(String),

    #[error("REPLACE_E_ANCHOR_MISMATCH: {0}")]
    AnchorMismatch(String),

    #[error("REPLACE_E_FILE_DELETED: {0}")]
    FileDeleted(String),

    #[error("REPLACE_E_FILE_MOVED_AMBIGUOUS: {0}")]
    FileMovedAmbiguous(String),

    #[error("REPLACE_E_ROLLBACK: {0}")]
    Rollback(String),

    #[error("REPLACE_E_STAGE_CONFLICT: {0}")]
    StageConflict(String),

    #[error("REPLACE_E_PARSE_BLOCKED: {0}")]
    ParseBlocked(String),

    #[error("REPLACE_E_PRE_HOOK_FAILED: {0}")]
    PreHookFailed(String),

    #[error("REPLACE_E_POST_HOOK_FAILED: {0}")]
    PostHookFailed(String),

    #[error("REPLACE_E_STRICT: {0}")]
    Strict(String),

    #[error("{0}")]
    Message(String),
}

impl ReplaceError {
    #[must_use]
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::InvalidArgs(_)
            | Self::Config(_)
            | Self::Glob(_)
            | Self::Regex(_)
            | Self::Json(_)
            | Self::LiteralMatcher(_)
            | Self::UnknownStage(_)
            | Self::UnknownHistory(_) => ExitCode::from(2),
            Self::Ast(_) | Self::ParseBlocked(_) => ExitCode::from(4),
            Self::AnchorMismatch(_)
            | Self::FileDeleted(_)
            | Self::FileMovedAmbiguous(_)
            | Self::Patch(_)
            | Self::StageConflict(_) => ExitCode::from(5),
            Self::Rollback(_) => ExitCode::from(6),
            Self::PreHookFailed(_) => ExitCode::from(9),
            Self::PostHookFailed(_) => ExitCode::from(10),
            Self::LockTimeout(_) => ExitCode::from(8),
            Self::Io(_) | Self::Git(_) | Self::Lock(_) | Self::Strict(_) | Self::Message(_) => {
                ExitCode::from(1)
            }
        }
    }

    /// Get the stable error code prefix (e.g. "`REPLACE_E_CONFIG`").
    #[must_use]
    pub const fn code(&self) -> &str {
        match self {
            Self::InvalidArgs(_) => "REPLACE_E_INVALID_ARGS",
            Self::Config(_) => "REPLACE_E_CONFIG",
            Self::Io(_) => "REPLACE_E_IO",
            Self::Glob(_) => "REPLACE_E_GLOB",
            Self::Regex(_) => "REPLACE_E_REGEX",
            Self::Json(_) => "REPLACE_E_JSON",
            Self::LiteralMatcher(_) => "REPLACE_E_LITERAL",
            Self::Ast(_) => "REPLACE_E_AST",
            Self::ParseBlocked(_) => "REPLACE_E_PARSE_BLOCKED",
            Self::Git(_) => "REPLACE_E_GIT",
            Self::Patch(_) => "REPLACE_E_PATCH",
            Self::Lock(_) => "REPLACE_E_LOCK",
            Self::LockTimeout(_) => "REPLACE_E_LOCK_TIMEOUT",
            Self::UnknownStage(_) => "REPLACE_E_UNKNOWN_STAGE",
            Self::UnknownHistory(_) => "REPLACE_E_UNKNOWN_HISTORY",
            Self::AnchorMismatch(_) => "REPLACE_E_ANCHOR_MISMATCH",
            Self::FileDeleted(_) => "REPLACE_E_FILE_DELETED",
            Self::FileMovedAmbiguous(_) => "REPLACE_E_FILE_MOVED_AMBIGUOUS",
            Self::Rollback(_) => "REPLACE_E_ROLLBACK",
            Self::StageConflict(_) => "REPLACE_E_STAGE_CONFLICT",
            Self::PreHookFailed(_) => "REPLACE_E_PRE_HOOK_FAILED",
            Self::PostHookFailed(_) => "REPLACE_E_POST_HOOK_FAILED",
            Self::Strict(_) => "REPLACE_E_STRICT",
            Self::Message(_) => "REPLACE_E_UNKNOWN",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ReplaceError;
    use std::process::ExitCode;

    #[test]
    fn exit_code_maps_existing_variants_to_spec_codes() {
        let cases = [
            (ReplaceError::InvalidArgs("bad args".to_owned()), 2),
            (ReplaceError::UnknownStage("missing".to_owned()), 2),
            (ReplaceError::Ast("parse blocked".to_owned()), 4),
            (ReplaceError::AnchorMismatch("drift".to_owned()), 5),
            (ReplaceError::FileDeleted("gone".to_owned()), 5),
            (ReplaceError::FileMovedAmbiguous("ambiguous".to_owned()), 5),
            (ReplaceError::StageConflict("overlap".to_owned()), 5),
            (ReplaceError::Rollback("rolled back".to_owned()), 6),
            (ReplaceError::LockTimeout("held".to_owned()), 8),
            (ReplaceError::PreHookFailed("false".to_owned()), 9),
            (ReplaceError::PostHookFailed("false".to_owned()), 10),
            (ReplaceError::Io(std::io::Error::other("io")), 1),
        ];

        for (error, code) in cases {
            assert_eq!(error.exit_code(), ExitCode::from(code));
        }
    }
}
