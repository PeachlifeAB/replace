use std::env;

/// Read an environment variable as a usize, returning None if not set or invalid.
fn read_usize_var(name: &str) -> Option<usize> {
    env::var(name).ok().and_then(|val| val.parse().ok())
}

/// Read an environment variable as a string, returning None if not set.
fn read_string_var(name: &str) -> Option<String> {
    env::var(name).ok()
}

/// `$XDG_CONFIG_HOME`, when set and non-empty.
#[must_use]
pub fn xdg_config_home() -> Option<std::path::PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
}

/// `$HOME`, when set and non-empty.
#[must_use]
pub fn home_dir() -> Option<std::path::PathBuf> {
    env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
}

/// Maximum number of files to scan for AST operations.
///
/// Environment variable: `REPLACE_AST_MAX_FILES`
/// Default: None (no limit)
///
/// This is the default value; the CLI flag `--max-files` overrides it.
#[must_use]
pub fn ast_max_files() -> Option<usize> {
    read_usize_var("REPLACE_AST_MAX_FILES")
}

/// Maximum total replacements for AST operations.
///
/// Environment variable: `REPLACE_AST_MAX_REPLACEMENTS`
/// Default: None (no limit)
#[must_use]
pub fn ast_max_replacements() -> Option<usize> {
    read_usize_var("REPLACE_AST_MAX_REPLACEMENTS")
}

/// Maximum number of parse error diagnostics to show before truncation.
///
/// Environment variable: `REPLACE_PARSE_ERROR_LIMIT`
/// Default: 20
///
/// When exceeded, a summary line like "N more errors" is appended.
#[must_use]
pub fn parse_error_limit() -> usize {
    read_usize_var("REPLACE_PARSE_ERROR_LIMIT").unwrap_or(20)
}

/// Format an error summary with truncation.
///
/// If the number of errors exceeds the limit, returns a summary line
/// indicating how many errors were truncated.
#[must_use]
pub fn format_error_summary(errors: &[String], limit: usize) -> String {
    if errors.len() <= limit {
        errors.join("\n")
    } else {
        let truncated: String = errors[..limit].join("\n");
        format!(
            "{truncated}\n... {count} more errors (limit: {limit})",
            count = errors.len() - limit,
            limit = limit
        )
    }
}

/// Test-only pause point for concurrency testing.
///
/// Environment variable: `REPLACE_TEST_PAUSE_AT`
/// Values: "stage", "apply", "check-after"
///
/// When set, replace pauses after acquiring the lock at the named lifecycle
/// point and waits for a signal or test harness release. Unset in production
/// has zero effect.
///
/// This is test-only and should not be used in production.
#[must_use]
pub fn test_pause_at() -> Option<TestPausePoint> {
    read_string_var("REPLACE_TEST_PAUSE_AT").and_then(|val| match val.as_str() {
        "stage" => Some(TestPausePoint::Stage),
        "apply" => Some(TestPausePoint::Apply),
        "check-after" => Some(TestPausePoint::CheckAfter),
        _ => None,
    })
}

/// Pause points for test cooperation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestPausePoint {
    /// Pause after stage lock acquisition
    Stage,
    /// Pause after apply lock acquisition
    Apply,
    /// Pause after check-after (post-apply validation)
    CheckAfter,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_error_summary_shows_all_when_under_limit() {
        let errors = vec!["error 1".to_string(), "error 2".to_string()];
        assert_eq!(format_error_summary(&errors, 5), "error 1\nerror 2");
    }

    #[test]
    fn format_error_summary_truncates_when_over_limit() {
        let errors: Vec<String> = (1..=10).map(|i| format!("error {i}")).collect();
        let result = format_error_summary(&errors, 3);
        assert!(result.contains("error 1"));
        assert!(result.contains("error 2"));
        assert!(result.contains("error 3"));
        assert!(result.contains("7 more errors"));
    }

    #[test]
    fn test_pause_point_parses_correctly() {
        assert_eq!(TestPausePoint::Stage, TestPausePoint::Stage);
        assert_eq!(TestPausePoint::Apply, TestPausePoint::Apply);
        assert_eq!(TestPausePoint::CheckAfter, TestPausePoint::CheckAfter);
    }
}
