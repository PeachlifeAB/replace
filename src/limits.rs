use std::time::Duration;

use bytesize::ByteSize;
use serde::{Deserialize, Deserializer, de::Error as _};

use crate::error::ReplaceError;

/// Run limits shared by CLI flags, `replace.toml` and the user config.
///
/// Layers merge with [`Limits::or`]: the first layer that sets a value wins.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Limits {
    /// Maximum file size in bytes; written as e.g. "5MiB" in TOML.
    #[serde(deserialize_with = "deserialize_file_size")]
    pub max_file_size: Option<u64>,
    pub max_files: Option<usize>,
    pub max_matches: Option<usize>,
    pub max_matches_per_file: Option<usize>,
    /// Stage lock timeout; written as e.g. "30s" in TOML.
    #[serde(deserialize_with = "deserialize_lock_timeout")]
    pub lock_timeout: Option<Duration>,
}

impl Limits {
    /// Values set on `self` win; unset values fall back to `fallback`.
    #[must_use]
    pub fn or(self, fallback: Self) -> Self {
        Self {
            max_file_size: self.max_file_size.or(fallback.max_file_size),
            max_files: self.max_files.or(fallback.max_files),
            max_matches: self.max_matches.or(fallback.max_matches),
            max_matches_per_file: self.max_matches_per_file.or(fallback.max_matches_per_file),
            lock_timeout: self.lock_timeout.or(fallback.lock_timeout),
        }
    }
}

/// Parse a size such as "5MiB", "100KiB", "1KB", "1024B" or "1024".
///
/// # Errors
///
/// Returns an error when `text` is not a size.
pub fn parse_file_size(text: &str) -> Result<u64, ReplaceError> {
    text.parse::<ByteSize>()
        .map(|size| size.as_u64())
        .map_err(|error| ReplaceError::InvalidArgs(format!("invalid file size: {error}")))
}

/// Parse a duration such as "30s", "5m", "1h"; a bare number means seconds.
///
/// # Errors
///
/// Returns an error when `text` is not a duration.
pub fn parse_lock_timeout(text: &str) -> Result<Duration, ReplaceError> {
    if let Ok(seconds) = text.parse::<u64>() {
        return Ok(Duration::from_secs(seconds));
    }
    humantime::parse_duration(text)
        .map_err(|error| ReplaceError::InvalidArgs(format!("invalid lock-timeout: {error}")))
}

fn deserialize_file_size<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<u64>, D::Error> {
    Option::<String>::deserialize(deserializer)?
        .map(|text| parse_file_size(&text).map_err(D::Error::custom))
        .transpose()
}

fn deserialize_lock_timeout<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Duration>, D::Error> {
    Option::<String>::deserialize(deserializer)?
        .map(|text| parse_lock_timeout(&text).map_err(D::Error::custom))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_layer_that_sets_a_value_wins() {
        let cli = Limits {
            max_files: Some(1),
            ..Limits::default()
        };
        let project = Limits {
            max_files: Some(2),
            max_matches: Some(3),
            ..Limits::default()
        };
        let user = Limits {
            max_matches: Some(9),
            max_matches_per_file: Some(4),
            ..Limits::default()
        };

        let merged = cli.or(project).or(user);

        assert_eq!(merged.max_files, Some(1));
        assert_eq!(merged.max_matches, Some(3));
        assert_eq!(merged.max_matches_per_file, Some(4));
    }

    #[test]
    fn toml_sizes_and_durations_are_typed() {
        let limits: Limits =
            toml::from_str("max_file_size = \"1KiB\"\nlock_timeout = \"2m\"").expect("limits");

        assert_eq!(limits.max_file_size, Some(1024));
        assert_eq!(limits.lock_timeout, Some(Duration::from_mins(2)));
    }
}
