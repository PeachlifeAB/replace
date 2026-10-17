use aho_corasick::{AhoCorasickBuilder, MatchKind};

use crate::engine::RewriteResult;
use crate::error::ReplaceError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteralRule {
    pub find: String,
    pub replace: String,
    /// Match ASCII letters regardless of case.
    pub ignore_case: bool,
}

/// Apply all literal rules to `input`, returning the rewritten text and match count.
///
/// # Errors
///
/// Returns an error if the matcher cannot be built from the rules.
pub fn replace_all(input: &str, rules: &[LiteralRule]) -> Result<RewriteResult, ReplaceError> {
    if rules.is_empty() {
        return Ok(RewriteResult::unchanged(input));
    }

    let patterns: Vec<&str> = rules.iter().map(|rule| rule.find.as_str()).collect();
    let replacements: Vec<&str> = rules.iter().map(|rule| rule.replace.as_str()).collect();
    let matcher = AhoCorasickBuilder::new()
        .match_kind(MatchKind::LeftmostFirst)
        .ascii_case_insensitive(rules.iter().any(|rule| rule.ignore_case))
        .build(patterns)?;
    let count = matcher.find_iter(input).count();

    Ok(RewriteResult {
        output: matcher.replace_all(input, &replacements),
        replacements: count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_all_should_replace_literals() -> Result<(), ReplaceError> {
        let result = replace_all(
            "omega beta omega",
            &[LiteralRule {
                ignore_case: false,
                find: "omega".to_owned(),
                replace: "sigma".to_owned(),
            }],
        )?;

        assert_eq!(result.output, "sigma beta sigma");
        Ok(())
    }
}
