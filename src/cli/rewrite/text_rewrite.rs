//! Literal and regex rewrites from explicit requests.
use camino::Utf8PathBuf;

use super::super::RewriteOptions;
use super::executor::{FileRewrite, RewriteJob, RewriteOutcome, execute_rewrite};
use crate::engine::literal::{self, LiteralRule};
use crate::engine::regex::{self, RegexRule};
use crate::error::ReplaceError;
use crate::stage::StageMode;

/// A literal or regex rewrite request.
pub struct TextRewrite<'a> {
    pub(crate) pattern: &'a str,
    pub(crate) replacement: &'a str,
    pub(crate) paths: &'a [Utf8PathBuf],
    pub(crate) ignore_case: bool,
}

pub fn execute_regex(
    request: &TextRewrite<'_>,
    options: RewriteOptions,
) -> Result<RewriteOutcome, ReplaceError> {
    if request.pattern.is_empty() {
        return Err(ReplaceError::InvalidArgs(
            "regex pattern cannot be empty".to_owned(),
        ));
    }
    let rule = RegexRule {
        pattern: request.pattern.to_owned(),
        replacement: request.replacement.to_owned(),
        ignore_case: request.ignore_case,
    };
    execute_rewrite(
        &text_job(StageMode::Regex, request),
        options,
        |_path, original| {
            let result = regex::replace_all(original, &rule)?;
            Ok(FileRewrite {
                output: result.output,
                replacements: result.replacements,
                language: None,
                first_match: None,
            })
        },
    )
}

pub fn execute_literal(
    request: &TextRewrite<'_>,
    options: RewriteOptions,
) -> Result<RewriteOutcome, ReplaceError> {
    if request.pattern.is_empty() {
        return Err(ReplaceError::InvalidArgs(
            "literal find text cannot be empty".to_owned(),
        ));
    }
    let rules = [LiteralRule {
        find: request.pattern.to_owned(),
        replace: request.replacement.to_owned(),
        ignore_case: request.ignore_case,
    }];
    execute_rewrite(
        &text_job(StageMode::Literal, request),
        options,
        |_path, original| {
            let result = literal::replace_all(original, &rules)?;
            Ok(FileRewrite {
                output: result.output,
                replacements: result.replacements,
                language: None,
                first_match: None,
            })
        },
    )
}

const fn text_job<'a>(mode: StageMode, request: &TextRewrite<'a>) -> RewriteJob<'a> {
    RewriteJob {
        mode,
        pattern: request.pattern,
        rewrite: request.replacement,
        paths: request.paths,
        ignore_case: request.ignore_case,
    }
}
