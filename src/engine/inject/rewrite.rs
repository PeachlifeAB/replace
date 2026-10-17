use ast_grep_core::matcher::Pattern;
use ast_grep_language::LanguageExt;

use super::InjectedRegion;
use crate::error::ReplaceError;

/// Apply an AST rewrite to an injected region.
///
/// Returns the rewritten region content and the number of replacements made, or `None` if no matches were found.
///
/// # Errors
///
/// Returns an error if ast-grep cannot apply an edit.
pub fn rewrite_region(
    input: &str,
    region: &InjectedRegion,
    pattern: &Pattern,
    replacement: &str,
) -> Result<Option<(String, usize)>, ReplaceError> {
    let region_content = &input[region.start..region.end];

    let mut ast = region.language.ast_grep(region_content);
    let matches: Vec<_> = ast.root().find_all(pattern).collect();
    if matches.is_empty() {
        return Ok(None);
    }

    let count = matches.len();
    let edits: Vec<_> = matches.iter().map(|m| m.replace_by(replacement)).collect();

    for edit in edits {
        ast.edit(edit).map_err(ReplaceError::Ast)?;
    }

    Ok(Some((ast.generate(), count)))
}
