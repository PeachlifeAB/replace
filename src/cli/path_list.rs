//! `--files0-from`: NUL-delimited target paths read from a file or stdin.
use std::fs;
use std::io::{self, Read};

use camino::Utf8PathBuf;

use crate::error::ReplaceError;

use super::OutputOptions;
use super::rewrite::RewriteOutcome;
use super::util;
use crate::warning::WarningCode;

const STDIN_SOURCE: &str = "-";

/// Read the NUL-delimited path list named by `source` (`-` is stdin).
///
/// Empty entries are ignored, so a trailing NUL or empty input yields fewer paths.
///
/// # Errors
///
/// Returns an error if the source cannot be read or contains non-UTF-8 paths.
pub fn read_nul_delimited_paths(source: &str) -> Result<Vec<Utf8PathBuf>, ReplaceError> {
    let mut bytes = Vec::new();
    if source == STDIN_SOURCE {
        io::stdin().lock().read_to_end(&mut bytes)?;
    } else {
        bytes = fs::read(source).map_err(|error| {
            ReplaceError::InvalidArgs(format!("cannot read --files0-from {source}: {error}"))
        })?;
    }
    parse_nul_delimited(&bytes)
}

fn parse_nul_delimited(bytes: &[u8]) -> Result<Vec<Utf8PathBuf>, ReplaceError> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            std::str::from_utf8(entry)
                .map(Utf8PathBuf::from)
                .map_err(|_| {
                    ReplaceError::InvalidArgs("--files0-from entry is not UTF-8".to_owned())
                })
        })
        .collect()
}

/// Paths a rewrite should visit: the `--files0-from` list, or the positional paths.
///
/// `None` means the list was given but empty, so there is nothing to visit.
fn target_paths(
    listed: Option<&[Utf8PathBuf]>,
    positional: &[Utf8PathBuf],
) -> Result<Option<Vec<Utf8PathBuf>>, ReplaceError> {
    match listed {
        None => Ok(Some(positional.to_vec())),
        Some(_) if !positional.is_empty() => Err(ReplaceError::InvalidArgs(
            "--files0-from cannot be combined with positional PATHS".to_owned(),
        )),
        Some([]) => Ok(None),
        Some(paths) => Ok(Some(paths.to_vec())),
    }
}

fn report_no_targets(output: OutputOptions) -> Result<RewriteOutcome, ReplaceError> {
    let mut outcome = RewriteOutcome::default();
    outcome.warnings.record(
        WarningCode::ZeroMatches,
        None,
        "no matches found".to_owned(),
    );
    if output.json {
        util::write_stdout_line(format_args!(
            "{}",
            serde_json::json!({
                "stages": [],
                "stats": { "files": 0, "replacements": 0 },
                "warnings": outcome.warnings.as_slice()
            })
        ))?;
    } else {
        util::write_stdout_line(format_args!("No matches"))?;
    }
    Ok(outcome)
}

/// Run `rewrite` on the paths it should visit, or report that there are none.
///
/// With `--files0-from` the listed paths replace the positional ones; an empty list visits nothing.
pub(super) fn with_targets(
    output: OutputOptions,
    conclude: impl FnOnce(RewriteOutcome) -> Result<(), ReplaceError>,
    listed: Option<&[Utf8PathBuf]>,
    positional: &[Utf8PathBuf],
    rewrite: impl FnOnce(&[Utf8PathBuf]) -> Result<RewriteOutcome, ReplaceError>,
) -> Result<(), ReplaceError> {
    match target_paths(listed, positional)? {
        Some(targets) => conclude(rewrite(&targets)?),
        None => conclude(report_no_targets(output)?),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_nul_and_ignores_empty_entries() {
        let paths = parse_nul_delimited(b"a b.txt\0\0c.txt\0").expect("paths");

        assert_eq!(paths, ["a b.txt", "c.txt"].map(Utf8PathBuf::from));
    }

    #[test]
    fn rejects_non_utf8_entries() {
        assert!(parse_nul_delimited(&[0xff, 0xfe]).is_err());
    }
}
