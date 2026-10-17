//! Which files a rewrite visits: explicit paths, directory walks, and the skips that apply.
use std::fs;
use std::io;

use camino::Utf8PathBuf;

use crate::error::ReplaceError;
use crate::walk::{self, WalkOptions};
use crate::warning::{WarningCode, Warnings};

/// Resolve the path arguments (default: the current directory) into sorted, unique files.
pub(super) fn collect_targets_with_caps(
    paths: &[Utf8PathBuf],
    walk_opts: &WalkOptions,
    warnings: &mut Warnings,
) -> Result<Vec<Utf8PathBuf>, ReplaceError> {
    let current_dir = [Utf8PathBuf::from(".")];
    let paths = if paths.is_empty() {
        &current_dir
    } else {
        paths
    };

    let mut targets = Vec::new();
    for path in paths {
        targets.extend(targets_for_path(path, walk_opts, warnings)?);
    }
    targets.sort();
    targets.dedup();
    Ok(targets)
}

fn path_metadata(path: &Utf8PathBuf) -> Result<fs::Metadata, ReplaceError> {
    fs::symlink_metadata(path).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => {
            ReplaceError::InvalidArgs(format!("path does not exist: {path}"))
        }
        _ => error.into(),
    })
}

/// The files one path argument stands for: itself, a directory's walk, or nothing if skipped.
fn targets_for_path(
    path: &Utf8PathBuf,
    walk_opts: &WalkOptions,
    warnings: &mut Warnings,
) -> Result<Vec<Utf8PathBuf>, ReplaceError> {
    let metadata = path_metadata(path)?;
    if metadata.file_type().is_symlink() {
        warnings.record(
            WarningCode::SkippedSymlink,
            Some(path),
            format!("skipping symlink target: {path}"),
        );
        return Ok(Vec::new());
    }
    if metadata.is_dir() {
        return walk::collect_paths(path, walk_opts, warnings);
    }
    if !metadata.is_file() {
        return Err(ReplaceError::InvalidArgs(format!(
            "unsupported path type: {path}"
        )));
    }

    let admitted = !walk::is_excluded(path, &walk_opts.exclude)?
        && walk_opts
            .max_file_size
            .is_none_or(|max_size| walk::admit_file_size(path, metadata.len(), max_size, warnings));
    Ok(if admitted {
        vec![path.clone()]
    } else {
        Vec::new()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_paths_are_reported_with_context() {
        let temp = tempfile::tempdir().expect("temp dir");
        let missing =
            Utf8PathBuf::from_path_buf(temp.path().join("missing.txt")).expect("utf8 temp path");

        let error = collect_targets_with_caps(
            &[missing],
            &WalkOptions::default(),
            &mut Warnings::default(),
        )
        .expect_err("missing path should fail");

        assert!(error.to_string().contains("path does not exist:"));
    }
}
