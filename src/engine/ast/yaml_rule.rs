use ast_grep_config::{GlobalRules, SerializableRuleConfig, from_yaml_string};
use ast_grep_language::{LanguageExt, SupportLang};
use std::cmp::Reverse;

use crate::engine::RewriteResult;
use crate::error::ReplaceError;

pub(super) fn replace_yaml_rule(
    input: &str,
    language: SupportLang,
    yaml_rule: &str,
) -> Result<RewriteResult, ReplaceError> {
    let globals = GlobalRules::default();
    let configs = from_yaml_string::<SupportLang>(yaml_rule, &globals)
        .map_err(|error| ReplaceError::Ast(format!("invalid ast-grep YAML rule: {error}")))?;
    let [config] = configs.as_slice() else {
        return Err(ReplaceError::InvalidArgs(
            "--rule must define exactly one ast-grep rule".to_owned(),
        ));
    };
    let fixers = config
        .get_fixer()
        .map_err(|error| ReplaceError::Ast(format!("invalid ast-grep rule fix: {error}")))?;
    let [fixer] = fixers.as_slice() else {
        return Err(ReplaceError::InvalidArgs(
            "--rule must define exactly one fix".to_owned(),
        ));
    };

    let mut ast = language.ast_grep(input);
    let (count, edits) = {
        let matches: Vec<_> = ast.root().find_all(&config.matcher).collect();
        if matches.is_empty() {
            return Ok(RewriteResult::unchanged(input));
        }
        let mut edits: Vec<_> = matches.iter().map(|node| node.replace_by(fixer)).collect();
        edits.sort_by_key(|edit| Reverse(edit.position));
        (matches.len(), edits)
    };
    for edit in edits {
        ast.edit(edit).map_err(ReplaceError::Ast)?;
    }
    Ok(RewriteResult {
        output: ast.generate(),
        replacements: count,
    })
}

/// Return the language declared by a supported native ast-grep YAML rewrite rule.
///
/// # Errors
///
/// Returns an error when YAML is invalid or omits a rewrite fix.
pub fn yaml_rule_language(yaml_rule: &str) -> Result<SupportLang, ReplaceError> {
    let rule: SerializableRuleConfig<SupportLang> =
        ast_grep_config::from_str(yaml_rule).map_err(|error| {
            ReplaceError::InvalidArgs(format!("invalid ast-grep YAML rule: {error}"))
        })?;
    if rule.fix.is_none() {
        return Err(ReplaceError::InvalidArgs(
            "--rule must define a rewrite fix; lint-only rules are unsupported".to_owned(),
        ));
    }
    Ok(rule.language)
}
