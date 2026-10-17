use ast_grep_language::SupportLang;

use super::InjectedRegion;
use super::tag_language::{extract_script_lang, extract_style_lang};
use super::tag_scan::{find_closing_tag, find_tag_end, find_tag_start};
use crate::error::ReplaceError;

/// Detect injected language regions in an HTML document.
///
/// Scans for `<script>`, `<script lang="...">`, `<style>`, and `<style lang="...">`
/// blocks and returns the detected regions with their byte offsets and inferred languages.
///
/// # Errors
///
/// Returns an error if the document cannot be parsed or an unsupported language is detected.
pub fn detect_injected_regions(input: &str) -> Result<Vec<InjectedRegion>, ReplaceError> {
    let mut regions = Vec::new();

    // Detect <script> blocks
    detect_script_regions(input, &mut regions);

    // Detect <style> blocks
    detect_style_regions(input, &mut regions);

    // Sort by start offset
    regions.sort_by_key(|r| r.start);

    Ok(regions)
}

/// Detect `<script>` blocks in HTML and infer their language.
fn detect_script_regions(input: &str, regions: &mut Vec<InjectedRegion>) {
    detect_tag_regions(input, b"script", extract_script_lang, regions);
}

/// Detect `<style>` blocks in HTML and infer their language.
fn detect_style_regions(input: &str, regions: &mut Vec<InjectedRegion>) {
    detect_tag_regions(input, b"style", extract_style_lang, regions);
}

/// Collect the body of every `<tag ...>...</tag>` in `input`, skipping self-closing tags.
///
/// `language_of` infers the region language from the opening tag's byte range.
fn detect_tag_regions(
    input: &str,
    tag: &[u8],
    language_of: impl Fn(&str, usize, usize) -> SupportLang,
    regions: &mut Vec<InjectedRegion>,
) {
    let bytes = input.as_bytes();
    let mut pos = 0;

    while pos < bytes.len() {
        let Some(tag_start) = find_tag_start(bytes, pos, tag) else {
            break;
        };
        let Some(tag_end) = find_tag_end(bytes, tag_start) else {
            break;
        };
        if tag_end > 0 && bytes[tag_end - 1] == b'/' {
            pos = tag_end + 1;
            continue;
        }
        let Some(close_tag) = find_closing_tag(bytes, tag_end, tag) else {
            break;
        };

        regions.push(InjectedRegion {
            start: tag_end,
            end: close_tag,
            language: language_of(input, tag_start, tag_end),
        });
        pos = close_tag + 1;
    }
}
