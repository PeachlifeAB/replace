//! Resolve the `ast` command's positional arguments once, at the CLI boundary.
use std::fs;

use camino::{Utf8Path, Utf8PathBuf};

use crate::error::ReplaceError;

/// Pattern, rewrite and target paths after positional shifting.
#[derive(Debug, PartialEq, Eq)]
pub struct AstInputs {
    pub(crate) pattern: Option<String>,
    pub(crate) rewrite: Option<String>,
    pub(crate) paths: Vec<Utf8PathBuf>,
}

/// Where the pattern and rewrite come from.
pub enum PatternSource<'a> {
    /// `PATTERN REWRITE [PATHS...]` positionals.
    Positional,
    /// `--rule FILE`: every positional is a path.
    RuleFile,
    /// `--pattern-file` and `--rewrite-file`: every positional is a path.
    Files {
        pattern: &'a Utf8Path,
        rewrite: &'a Utf8Path,
    },
}

/// Interpret clap's `pattern`, `rewrite` and `paths` positionals for `source`.
///
/// # Errors
///
/// Returns an error if a pattern or rewrite file cannot be read.
pub fn resolve_ast_inputs(
    source: &PatternSource<'_>,
    pattern: Option<&str>,
    rewrite: Option<&str>,
    paths: &[Utf8PathBuf],
) -> Result<AstInputs, ReplaceError> {
    match source {
        PatternSource::Positional => Ok(AstInputs {
            pattern: pattern.map(str::to_owned),
            rewrite: rewrite.map(str::to_owned),
            paths: paths.to_vec(),
        }),
        PatternSource::RuleFile => Ok(AstInputs {
            pattern: None,
            rewrite: None,
            paths: shifted_paths(pattern, rewrite, paths),
        }),
        PatternSource::Files {
            pattern: pattern_file,
            rewrite: rewrite_file,
        } => Ok(AstInputs {
            pattern: Some(read_argument_file("pattern", pattern_file)?),
            rewrite: Some(read_argument_file("rewrite", rewrite_file)?),
            paths: shifted_paths(pattern, rewrite, paths),
        }),
    }
}

fn shifted_paths(
    pattern: Option<&str>,
    rewrite: Option<&str>,
    paths: &[Utf8PathBuf],
) -> Vec<Utf8PathBuf> {
    pattern
        .into_iter()
        .chain(rewrite)
        .map(Utf8PathBuf::from)
        .chain(paths.iter().cloned())
        .collect()
}

fn read_argument_file(role: &str, path: &Utf8Path) -> Result<String, ReplaceError> {
    let mut content = fs::read_to_string(path).map_err(|error| {
        ReplaceError::InvalidArgs(format!("cannot read {role} file {path}: {error}"))
    })?;
    if content.ends_with('\n') {
        content.pop();
        if content.ends_with('\r') {
            content.pop();
        }
    }
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_file_mode_treats_every_positional_as_a_path() {
        let inputs = resolve_ast_inputs(
            &PatternSource::RuleFile,
            Some("src"),
            Some("tests"),
            &[Utf8PathBuf::from("docs")],
        )
        .expect("inputs");

        assert_eq!(inputs.pattern, None);
        assert_eq!(
            inputs.paths,
            ["src", "tests", "docs"].map(Utf8PathBuf::from)
        );
    }
}
