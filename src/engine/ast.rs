use ast_grep_core::{Language, MatchStrictness};
use ast_grep_language::{LanguageExt, SupportLang};
use std::cmp::Reverse;

use crate::engine::RewriteResult;
use crate::error::ReplaceError;

mod match_context;
mod pattern_context;
mod yaml_rule;

pub use match_context::MatchContext;
pub(crate) use pattern_context::compile_pattern;
use yaml_rule::replace_yaml_rule;
pub use yaml_rule::yaml_rule_language;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstRewriteSpec {
    pub language: SupportLang,
    pub pattern: String,
    pub replacement: String,
    /// Context selector for ast-grep pattern compilation.
    pub selector: Option<String>,
    /// Strictness level for matching.
    pub strictness: Option<String>,
    /// Complete native ast-grep YAML rewrite rule.
    pub yaml_rule: Option<String>,
}

/// Lowercase names of every language the ast-grep engine can parse.
#[must_use]
pub fn supported_language_names() -> Vec<String> {
    SupportLang::all_langs()
        .iter()
        .map(|language| language.to_string().to_lowercase())
        .collect()
}

/// Language name inferred from a file extension by the ast-grep engine.
#[must_use]
pub fn language_name_for_path(path: &camino::Utf8Path) -> Option<String> {
    SupportLang::from_path(path.as_std_path()).map(|language| language.to_string().to_lowercase())
}

/// Refuse to rewrite a source that has syntax errors, listing them up to the diagnostic limit.
fn ensure_parses_cleanly<D: ast_grep_core::Doc>(
    ast: &ast_grep_core::AstGrep<D>,
) -> Result<(), ReplaceError> {
    let diagnostics: Vec<String> = ast
        .root()
        .dfs()
        .filter(|node| node.is_error() || node.is_missing())
        .map(|node| {
            let position = node.start_pos();
            format!(
                "syntax error at line {}, column {}",
                position.line() + 1,
                position.column(&node) + 1
            )
        })
        .collect();
    if diagnostics.is_empty() {
        return Ok(());
    }
    Err(ReplaceError::ParseBlocked(
        crate::env_vars::format_error_summary(&diagnostics, crate::env_vars::parse_error_limit()),
    ))
}

/// Backward-compatible name for the public AST rewrite specification.
pub type AstRule = AstRewriteSpec;

/// Apply an AST rule to `input`, returning the rewritten text and match count.
///
/// # Errors
///
/// Returns an error if the pattern cannot be compiled or an edit cannot be applied.
pub fn replace_all(input: &str, rule: &AstRewriteSpec) -> Result<RewriteResult, ReplaceError> {
    Ok(rewrite_with_context(input, rule)?.result)
}

/// An AST rewrite plus what its first match looked like (none for native YAML rules).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstRewrite {
    pub result: RewriteResult,
    pub first_match: Option<MatchContext>,
}

/// Like [`replace_all`], also describing the first match for the stage record.
///
/// # Errors
///
/// Returns an error if the pattern cannot be compiled or an edit cannot be applied.
pub fn rewrite_with_context(
    input: &str,
    rule: &AstRewriteSpec,
) -> Result<AstRewrite, ReplaceError> {
    if let Some(yaml_rule) = &rule.yaml_rule {
        return Ok(AstRewrite {
            result: replace_yaml_rule(input, rule.language, yaml_rule)?,
            first_match: None,
        });
    }

    let mut ast = rule.language.ast_grep(input);
    ensure_parses_cleanly(&ast)?;
    let pattern = compile_pattern(&rule.pattern, rule.language)?;
    let pattern = pattern.with_strictness(parse_strictness(rule.strictness.as_deref())?);

    let (count, edits, first_match) = {
        let matches: Vec<_> = ast
            .root()
            .find_all(&pattern)
            .filter(|node| {
                rule.selector
                    .as_deref()
                    .is_none_or(|kind| node.kind() == kind)
            })
            .collect();
        if matches.is_empty() {
            return Ok(AstRewrite {
                result: RewriteResult::unchanged(input),
                first_match: None,
            });
        }
        let first_match = matches.first().map(match_context::describe);

        let mut edits: Vec<_> = matches
            .iter()
            .map(|m| m.replace_by(rule.replacement.as_str()))
            .collect();
        edits.sort_by_key(|edit| Reverse(edit.position));
        (matches.len(), edits, first_match)
    };

    for edit in edits {
        ast.edit(edit).map_err(ReplaceError::Ast)?;
    }

    Ok(AstRewrite {
        result: RewriteResult {
            output: ast.generate(),
            replacements: count,
        },
        first_match,
    })
}

fn parse_strictness(value: Option<&str>) -> Result<MatchStrictness, ReplaceError> {
    match value.unwrap_or("smart") {
        "cst" => Ok(MatchStrictness::Cst),
        "smart" => Ok(MatchStrictness::Smart),
        "ast" => Ok(MatchStrictness::Ast),
        "relaxed" => Ok(MatchStrictness::Relaxed),
        "signature" => Ok(MatchStrictness::Signature),
        "template" => Ok(MatchStrictness::Template),
        value => Err(ReplaceError::InvalidArgs(format!(
            "invalid strictness level: {value}; expected cst, smart, ast, relaxed, signature, or template"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_all_should_apply_ast_rule() -> Result<(), ReplaceError> {
        let result = replace_all(
            "var a = 1;",
            &AstRule {
                language: SupportLang::Tsx,
                pattern: "var $A = $B;".to_owned(),
                replacement: "let $A = $B;".to_owned(),
                selector: None,
                strictness: None,
                yaml_rule: None,
            },
        )?;

        assert_eq!(result.output, "let a = 1;");
        Ok(())
    }

    #[test]
    fn replace_all_should_apply_selector_and_strictness() -> Result<(), ReplaceError> {
        let result = replace_all(
            "const a = 1; let b = 2;",
            &AstRule {
                language: SupportLang::TypeScript,
                pattern: "const $A = $B;".to_owned(),
                replacement: "let $A = $B;".to_owned(),
                selector: Some("lexical_declaration".to_owned()),
                strictness: Some("smart".to_owned()),
                yaml_rule: None,
            },
        )?;

        assert_eq!(result.output, "let a = 1; let b = 2;");
        assert_eq!(result.replacements, 1);
        Ok(())
    }

    #[test]
    fn replace_all_should_filter_nonmatching_selector() -> Result<(), ReplaceError> {
        let result = replace_all(
            "const a = 1;",
            &AstRule {
                language: SupportLang::TypeScript,
                pattern: "const $A = $B;".to_owned(),
                replacement: "let $A = $B;".to_owned(),
                selector: Some("identifier".to_owned()),
                strictness: None,
                yaml_rule: None,
            },
        )?;

        assert_eq!(result.output, "const a = 1;");
        assert_eq!(result.replacements, 0);
        Ok(())
    }

    #[test]
    fn replace_all_should_apply_native_yaml_constraints() -> Result<(), ReplaceError> {
        let result = replace_all(
            "var chosen = 1; var ignored = 2;",
            &AstRule {
                language: SupportLang::TypeScript,
                pattern: "<native ast-grep YAML rule>".to_owned(),
                replacement: "<native ast-grep fix>".to_owned(),
                selector: None,
                strictness: None,
                yaml_rule: Some(
                    "id: replace-chosen\nlanguage: TypeScript\nrule:\n  pattern: var $A = $B;\nconstraints:\n  A:\n    regex: ^chosen$\nfix: let $A = $B;\n".to_owned(),
                ),
            },
        )?;

        assert_eq!(result.output, "let chosen = 1; var ignored = 2;");
        assert_eq!(result.replacements, 1);
        Ok(())
    }

    #[test]
    fn replace_all_should_count_multiple_matches() -> Result<(), ReplaceError> {
        let result = replace_all(
            "var a = 1; var b = 2;",
            &AstRule {
                language: SupportLang::Tsx,
                pattern: "var $A = $B;".to_owned(),
                replacement: "let $A = $B;".to_owned(),
                selector: None,
                strictness: None,
                yaml_rule: None,
            },
        )?;

        assert_eq!(result.output, "let a = 1; let b = 2;");
        assert_eq!(result.replacements, 2);
        Ok(())
    }
}
