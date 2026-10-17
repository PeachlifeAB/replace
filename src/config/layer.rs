//! One config file as written: every key optional, so layers can inherit from each other.
use std::fs;
use std::path::Path;

use camino::Utf8PathBuf;
use serde::Deserialize;

use super::{HooksConfig, LangByGlobEntry, ReplaceConfig, Rule};
use crate::error::ReplaceError;
use crate::limits::Limits;

/// A single `config.toml`. Unset keys inherit from the next layer down.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ConfigLayer {
    pub root: Option<Utf8PathBuf>,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub include_hidden: Option<bool>,
    pub limits: Limits,
    pub rules: Option<Vec<Rule>>,
    #[serde(alias = "apply")]
    pub apply_immediately: Option<bool>,
    pub hooks: Option<HooksConfig>,
    pub lang_by_glob: Option<Vec<LangByGlobEntry>>,
}

impl ConfigLayer {
    /// Parse a layer from TOML text.
    ///
    /// # Errors
    ///
    /// Returns an error if the text is not a valid config.
    pub fn from_toml(content: &str) -> Result<Self, ReplaceError> {
        Ok(toml::from_str(content)?)
    }

    /// Read a layer from `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or is not a valid config.
    pub fn from_file(path: &Path) -> Result<Self, ReplaceError> {
        Self::from_toml(&fs::read_to_string(path)?)
    }

    /// Keys set on `self` win; unset keys fall back to `lower`.
    ///
    /// A list or table set here replaces the lower layer's value as a whole.
    #[must_use]
    pub fn or(self, lower: Self) -> Self {
        Self {
            root: self.root.or(lower.root),
            include: self.include.or(lower.include),
            exclude: self.exclude.or(lower.exclude),
            include_hidden: self.include_hidden.or(lower.include_hidden),
            limits: self.limits.or(lower.limits),
            rules: self.rules.or(lower.rules),
            apply_immediately: self.apply_immediately.or(lower.apply_immediately),
            hooks: self.hooks.or(lower.hooks),
            lang_by_glob: self.lang_by_glob.or(lower.lang_by_glob),
        }
    }

    /// Fill every unset key with its default.
    #[must_use]
    pub fn resolve(self) -> ReplaceConfig {
        ReplaceConfig {
            root: self.root,
            include: self.include.unwrap_or_default(),
            exclude: self.exclude.unwrap_or_default(),
            include_hidden: self.include_hidden.unwrap_or_default(),
            limits: self.limits,
            rules: self.rules.unwrap_or_default(),
            apply_immediately: self.apply_immediately.unwrap_or_default(),
            hooks: self.hooks.unwrap_or_default(),
            lang_by_glob: self.lang_by_glob.unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn higher_layer_overrides_only_the_keys_it_sets() {
        let local =
            ConfigLayer::from_toml("include_hidden = true\nexclude = [\"a\"]").expect("local");
        let global =
            ConfigLayer::from_toml("include_hidden = false\nexclude = [\"b\"]\ninclude = [\"c\"]")
                .expect("global");

        let merged = local.or(global).resolve();

        assert!(merged.include_hidden);
        assert_eq!(merged.exclude, ["a"]);
        assert_eq!(merged.include, ["c"]);
    }

    #[test]
    fn explicitly_empty_list_replaces_the_lower_layer() {
        let local = ConfigLayer::from_toml("exclude = []").expect("local");
        let global = ConfigLayer::from_toml("exclude = [\"b\"]").expect("global");

        assert!(local.or(global).resolve().exclude.is_empty());
    }

    #[test]
    fn unknown_keys_are_rejected() {
        assert!(ConfigLayer::from_toml("bogus = 1").is_err());
    }
}
