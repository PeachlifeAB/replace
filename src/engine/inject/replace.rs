use super::detect_injected_regions;
use super::rewrite_region;
use crate::engine::RewriteResult;
use crate::error::ReplaceError;

/// Apply an AST rewrite to all injected regions in a host document.
///
/// Returns the rewritten host document and the total number of replacements made.
///
/// # Errors
///
/// Returns an error if any region cannot be parsed or rewritten.
pub fn replace_injected_regions(
    input: &str,
    pattern: &str,
    replacement: &str,
) -> Result<RewriteResult, ReplaceError> {
    let regions = detect_injected_regions(input)?;
    if regions.is_empty() {
        return Ok(RewriteResult::unchanged(input));
    }

    // For each region, apply the pattern with the region's language
    let mut total_replacements = 0;
    let mut output = String::with_capacity(input.len());
    let mut last_end = 0;

    for region in &regions {
        // Copy the host content before this region
        output.push_str(&input[last_end..region.start]);

        // Try to rewrite this region with the region's language
        let pattern = crate::engine::ast::compile_pattern(pattern, region.language)?;

        match rewrite_region(input, region, &pattern, replacement)? {
            Some((rewritten, count)) => {
                output.push_str(&rewritten);
                total_replacements += count;
            }
            None => {
                // No matches in this region, keep original
                output.push_str(&input[region.start..region.end]);
            }
        }

        last_end = region.end;
    }

    // Copy the remaining host content
    output.push_str(&input[last_end..]);

    if total_replacements == 0 {
        return Ok(RewriteResult::unchanged(input));
    }

    Ok(RewriteResult {
        output,
        replacements: total_replacements,
    })
}
