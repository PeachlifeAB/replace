use std::fmt::Write as _;

use sha2::{Digest, Sha256};

mod diff;
mod selection;

use diff::{Kind, ParsedPatch};
pub use selection::{ChangeSelection, count_changes, retain_changes};

/// Compute a 4-hex-char anchor hash from file path and content.
///
/// The hash is derived from SHA-256 of `path + "\n" + content`, truncated to 4 hex chars.
/// Anchors pass the line's sign, number and text as `content`, so they are stable across
/// repeated runs on unchanged content and distinct for identical lines at different positions.
#[must_use]
pub fn compute_anchor_hash(path: &str, content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    hasher.update(b"\n");
    hasher.update(content.as_bytes());
    let digest = hasher.finalize();
    hex::encode(&digest[..2]) // 2 bytes = 4 hex chars
}

/// Parse an anchor string of the form `path#hash` into its components.
///
/// Returns `None` if the anchor is malformed (no `#` separator, or hash is not 4 hex chars).
#[must_use]
pub fn parse_anchor(anchor: &str) -> Option<(String, String)> {
    let hash_start = anchor.rfind('#')?;
    let path = anchor[..hash_start].to_owned();
    let hash = anchor[hash_start + 1..].to_owned();
    if hash.len() != 4 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some((path, hash))
}

/// Add hashline anchors to a unified diff string.
///
/// Each changed line (prefixed with `-` or `+`) gets a preceding anchor line of the form
/// `¶path#hash`, where `hash` covers the line's position and content.
#[must_use]
pub fn add_anchors_to_diff(diff: &str, file_path: &str) -> String {
    let patch = ParsedPatch::parse(diff);
    let mut result = String::with_capacity(diff.len() + diff.len() / 10);
    for line in &patch.header {
        push_line(&mut result, line);
    }
    for hunk in &patch.hunks {
        push_line(&mut result, hunk.header);
        for line in &hunk.lines {
            if let Some(anchor) = line.anchor(file_path) {
                writeln!(result, "¶{anchor}").expect("writing to String cannot fail");
            }
            push_line(&mut result, line.raw);
            if let Some(marker) = line.no_newline {
                push_line(&mut result, marker);
            }
        }
    }
    result
}

/// Extract hashline anchors from a unified diff patch.
///
/// Returns a vector of anchor strings in `path#hash` format, one per changed line.
#[must_use]
pub fn extract_anchors_from_patch(patch: &str, file_path: &str) -> Vec<String> {
    ParsedPatch::parse(patch)
        .hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .filter(|line| line.kind != Kind::Context)
        .filter_map(|line| line.anchor(file_path))
        .collect()
}

fn push_line(result: &mut String, line: &str) {
    result.push_str(line);
    result.push('\n');
}
