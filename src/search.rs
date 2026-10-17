use grep_matcher::Matcher;
use grep_regex::RegexMatcher;
use grep_searcher::{Searcher, sinks::UTF8};

use crate::error::ReplaceError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    pub line_number: u64,
    pub line: String,
    pub byte_start: usize,
    pub byte_end: usize,
}

/// Search `haystack` for `pattern` (regex), returning all matching lines.
///
/// # Errors
///
/// Returns an error if the pattern is not a valid regex or the search fails.
pub fn search_text(pattern: &str, haystack: &str) -> Result<Vec<SearchMatch>, ReplaceError> {
    let matcher =
        RegexMatcher::new(pattern).map_err(|error| ReplaceError::Message(error.to_string()))?;
    let mut results = Vec::new();

    Searcher::new()
        .search_slice(
            &matcher,
            haystack.as_bytes(),
            UTF8(|line_number, line| {
                if let Some(range) = matcher.find(line.as_bytes())? {
                    results.push(SearchMatch {
                        line_number,
                        line: line.to_owned(),
                        byte_start: range.start(),
                        byte_end: range.end(),
                    });
                }
                Ok(true)
            }),
        )
        .map_err(|error| ReplaceError::Message(error.to_string()))?;

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_text_should_return_matching_line_numbers() -> Result<(), ReplaceError> {
        let matches = search_text("needle", "hay\nneedle here\n")?;

        assert_eq!(matches[0].line_number, 2);
        Ok(())
    }
}
