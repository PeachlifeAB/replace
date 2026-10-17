//! Compiles source patterns that only parse inside a language context, such as a Go call
//! that is not valid top-level Go. Each language's context is one row of `pattern_context.toml`.
use std::sync::LazyLock;

use ast_grep_core::matcher::{Pattern, PatternError};
use ast_grep_language::SupportLang;
use serde::Deserialize;

use crate::error::ReplaceError;

const PATTERN_CONTEXT_TABLE: &str = include_str!("pattern_context.toml");
const PATTERN_PLACEHOLDER: &str = "{pattern}";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PatternContextTable {
    context: Vec<PatternContextRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct PatternContextRow {
    language: SupportLang,
    wrap: String,
    selector: String,
    trigger: ContextTrigger,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ContextTrigger {
    Contains {
        text: String,
        unless_starts_with: String,
    },
    BareRejected,
}

impl ContextTrigger {
    fn applies(&self, source: &str, bare_pattern_compiles: bool) -> bool {
        match self {
            Self::Contains {
                text,
                unless_starts_with,
            } => {
                source.contains(text.as_str())
                    && !source.trim_start().starts_with(unless_starts_with.as_str())
            }
            Self::BareRejected => !bare_pattern_compiles,
        }
    }
}

static PATTERN_CONTEXT_ROWS: LazyLock<Vec<PatternContextRow>> = LazyLock::new(|| {
    toml::from_str::<PatternContextTable>(PATTERN_CONTEXT_TABLE)
        .expect("pattern_context.toml must match the PatternContextTable schema")
        .context
});

fn context_row_for(language: SupportLang) -> Option<&'static PatternContextRow> {
    PATTERN_CONTEXT_ROWS
        .iter()
        .find(|row| row.language == language)
}

fn pattern_error(error: &PatternError) -> ReplaceError {
    ReplaceError::InvalidArgs(format!("invalid AST pattern: {error}"))
}

/// Compile `source` into a pattern for `language`, wrapping it in the language's context when
/// its table row says the bare form cannot stand alone.
///
/// # Errors
///
/// Returns an error if the pattern cannot be parsed in either form.
pub fn compile_pattern(source: &str, language: SupportLang) -> Result<Pattern, ReplaceError> {
    let bare_pattern = Pattern::try_new(source, language);
    let Some(row) =
        context_row_for(language).filter(|row| row.trigger.applies(source, bare_pattern.is_ok()))
    else {
        return bare_pattern.map_err(|error| pattern_error(&error));
    };

    let context = row.wrap.replacen(PATTERN_PLACEHOLDER, source, 1);
    let contextual_pattern = Pattern::contextual(&context, &row.selector, language);
    match (contextual_pattern, &row.trigger) {
        (Err(_), ContextTrigger::BareRejected) => {
            bare_pattern.map_err(|error| pattern_error(&error))
        }
        (contextual_pattern, _) => contextual_pattern.map_err(|error| pattern_error(&error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn go_row_wraps_call_patterns_in_a_function_body() {
        let row = context_row_for(SupportLang::Go).expect("go row");

        assert_eq!(row.selector, "call_expression");
        assert_eq!(
            row.wrap.replacen(PATTERN_PLACEHOLDER, "fmt.Println($A)", 1),
            "package main\nfunc _() {\nfmt.Println($A)\n}\n"
        );
        assert!(row.trigger.applies("fmt.Println($A)", true));
        assert!(!row.trigger.applies("package main", true));
        assert!(!row.trigger.applies("var x int", true));
    }

    #[test]
    fn rows_are_unique_per_language() {
        let mut seen_languages = HashSet::new();
        assert!(
            PATTERN_CONTEXT_ROWS
                .iter()
                .all(|row| seen_languages.insert(row.language))
        );
    }
}
