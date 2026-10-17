use camino::{Utf8Path, Utf8PathBuf};
use globset::{Glob, GlobSetBuilder};
use serde::{Deserialize, Serialize};

use crate::limits::Limits;

mod layer;
mod locate;
pub use layer::ConfigLayer;
pub use locate::{
    ConfigLocations, HomeFallback, IgnoredLegacyFile, LoadedConfig, REPLACE_DIR_NAME,
};

/// File name of every config layer.
pub const CONFIG_FILE_NAME: &str = "config.toml";

/// A rule in the config file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Rule {
    /// Literal text replacement.
    Literal { find: String, replace: String },
    /// Regex pattern replacement.
    Regex {
        pattern: String,
        replacement: String,
    },
    /// AST pattern replacement.
    Ast {
        language: Option<String>,
        pattern: String,
        replacement: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HooksConfig {
    pub pre_apply: Vec<String>,
    pub post_apply: Vec<String>,
}

/// The effective configuration after every layer is merged and defaults are filled in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReplaceConfig {
    pub root: Option<Utf8PathBuf>,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub include_hidden: bool,
    pub limits: Limits,
    /// Rules `replace run` applies.
    pub rules: Vec<Rule>,
    /// Apply immediately without staging.
    pub apply_immediately: bool,
    /// Commands run before/after config-driven apply.
    pub hooks: HooksConfig,
    /// Map glob patterns to language names for AST parsing.
    /// CLI --lang always overrides this, which in turn overrides default extension inference.
    pub lang_by_glob: Vec<LangByGlobEntry>,
}

/// A single glob-to-language mapping in the `lang_by_glob` config section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LangByGlobEntry {
    pub glob: String,
    pub language: String,
}

impl ReplaceConfig {
    /// Look up the language for a given file path based on the `lang_by_glob` config section.
    ///
    /// Returns None if no mapping matches or if `lang_by_glob` is empty.
    /// The first matching glob wins (order matters).
    #[must_use]
    pub fn language_for_path(&self, path: &Utf8Path) -> Option<&str> {
        for entry in &self.lang_by_glob {
            if let Ok(glob) = Glob::new(&entry.glob)
                && let Ok(globset) = GlobSetBuilder::new().add(glob).build()
                && globset.is_match(path.as_std_path())
            {
                return Some(&entry.language);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved(toml: &str) -> ReplaceConfig {
        ConfigLayer::from_toml(toml)
            .expect("valid config")
            .resolve()
    }

    #[test]
    fn empty_config_resolves_to_defaults() {
        let config = resolved("");

        assert_eq!(config, ReplaceConfig::default());
        assert!(config.rules.is_empty());
        assert!(!config.apply_immediately);
    }

    #[test]
    fn rules_are_parsed() {
        let config = resolved(
            r#"
            [[rules]]
            type = "literal"
            find = "foo"
            replace = "bar"

            [[rules]]
            type = "regex"
            pattern = "\\d+"
            replacement = "42"

            [[rules]]
            type = "ast"
            language = "typescript"
            pattern = "var $A"
            replacement = "let $A"
        "#,
        );

        assert_eq!(config.rules.len(), 3);
    }

    #[test]
    fn lang_by_glob_entries_are_parsed() {
        let config = resolved(
            r#"
            [[lang_by_glob]]
            glob = "*.tsx"
            language = "tsx"

            [[lang_by_glob]]
            glob = "*.go"
            language = "go"
        "#,
        );

        assert_eq!(config.lang_by_glob.len(), 2);
        assert_eq!(config.lang_by_glob[0].glob, "*.tsx");
        assert_eq!(config.lang_by_glob[0].language, "tsx");
        assert_eq!(config.lang_by_glob[1].glob, "*.go");
        assert_eq!(config.lang_by_glob[1].language, "go");
    }

    #[test]
    fn language_for_path_should_match_globs() {
        let config = ReplaceConfig {
            lang_by_glob: vec![
                LangByGlobEntry {
                    glob: "*.tsx".to_owned(),
                    language: "tsx".to_owned(),
                },
                LangByGlobEntry {
                    glob: "*.go".to_owned(),
                    language: "go".to_owned(),
                },
            ],
            ..Default::default()
        };

        assert_eq!(
            config.language_for_path(Utf8Path::new("test.tsx")),
            Some("tsx")
        );
        assert_eq!(
            config.language_for_path(Utf8Path::new("src/main.go")),
            Some("go")
        );
        assert_eq!(config.language_for_path(Utf8Path::new("test.rs")), None);
    }

    #[test]
    fn language_for_path_returns_first_match() {
        let config = ReplaceConfig {
            lang_by_glob: vec![
                LangByGlobEntry {
                    glob: "*.test.*".to_owned(),
                    language: "typescript".to_owned(),
                },
                LangByGlobEntry {
                    glob: "*.ts".to_owned(),
                    language: "typescript".to_owned(),
                },
            ],
            ..Default::default()
        };

        assert_eq!(
            config.language_for_path(Utf8Path::new("test.test.ts")),
            Some("typescript")
        );
    }
}
