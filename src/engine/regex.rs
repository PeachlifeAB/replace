use crate::engine::RewriteResult;
use crate::error::ReplaceError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegexRule {
    pub pattern: String,
    pub replacement: String,
    pub ignore_case: bool,
}

/// Apply a regex rule to `input`, returning the rewritten text and match count.
///
/// `^` and `$` match at line boundaries, as in rg, grep and sed.
///
/// # Errors
///
/// Returns an error if the regex pattern is invalid.
pub fn replace_all(input: &str, rule: &RegexRule) -> Result<RewriteResult, ReplaceError> {
    let regex = regex::RegexBuilder::new(&rule.pattern)
        .case_insensitive(rule.ignore_case)
        .multi_line(true)
        .build()?;
    let replacements = regex.find_iter(input).count();
    let output = regex
        .replace_all(input, rule.replacement.as_str())
        .into_owned();

    Ok(RewriteResult {
        output,
        replacements,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_all_should_apply_regex_replacement() -> Result<(), ReplaceError> {
        let result = replace_all(
            "omega-123",
            &RegexRule {
                ignore_case: false,
                pattern: r"\d+".to_owned(),
                replacement: "456".to_owned(),
            },
        )?;

        assert_eq!(result.output, "omega-456");
        Ok(())
    }

    #[test]
    fn anchors_match_at_every_line() -> Result<(), ReplaceError> {
        let result = replace_all(
            "x\nfoo\nfoo bar\n",
            &RegexRule {
                ignore_case: false,
                pattern: "^foo$".to_owned(),
                replacement: "X".to_owned(),
            },
        )?;

        assert_eq!(result.output, "x\nX\nfoo bar\n");
        assert_eq!(result.replacements, 1);
        Ok(())
    }
}
