//! Non-fatal conditions a run reports: stable labels for people, scripts and agents.
use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

/// The stable label of a warning; scripts match on `as_str`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum WarningCode {
    #[serde(rename = "REPLACE_W_SKIPPED_BINARY")]
    SkippedBinary,
    #[serde(rename = "REPLACE_W_SKIPPED_ENCODING")]
    SkippedEncoding,
    #[serde(rename = "REPLACE_W_SKIPPED_LARGE_FILE")]
    SkippedLargeFile,
    #[serde(rename = "REPLACE_W_SKIPPED_SYMLINK")]
    SkippedSymlink,
    #[serde(rename = "REPLACE_W_ZERO_MATCHES")]
    ZeroMatches,
}

impl WarningCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SkippedBinary => "REPLACE_W_SKIPPED_BINARY",
            Self::SkippedEncoding => "REPLACE_W_SKIPPED_ENCODING",
            Self::SkippedLargeFile => "REPLACE_W_SKIPPED_LARGE_FILE",
            Self::SkippedSymlink => "REPLACE_W_SKIPPED_SYMLINK",
            Self::ZeroMatches => "REPLACE_W_ZERO_MATCHES",
        }
    }
}

/// One reported condition: what happened and, when it concerns a file, which one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Warning {
    pub code: WarningCode,
    pub path: Option<Utf8PathBuf>,
    pub message: String,
}

/// The warnings of one run. Each is printed to stderr as it is recorded.
#[derive(Debug, Default)]
pub struct Warnings(Vec<Warning>);

impl Warnings {
    pub fn record(&mut self, code: WarningCode, path: Option<&Utf8Path>, message: String) {
        eprintln!("warning: {}: {message}", code.as_str());
        self.0.push(Warning {
            code,
            path: path.map(Utf8Path::to_path_buf),
            message,
        });
    }

    pub fn merge(&mut self, other: Self) {
        self.0.extend(other.0);
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn as_slice(&self) -> &[Warning] {
        &self.0
    }

    /// Comma-separated codes, for the `--strict` failure message.
    #[must_use]
    pub fn codes(&self) -> String {
        let mut codes: Vec<&str> = self.0.iter().map(|warning| warning.code.as_str()).collect();
        codes.sort_unstable();
        codes.dedup();
        codes.join(", ")
    }
}
