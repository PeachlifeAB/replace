/// Find the start of a tag (e.g., `<script` or `<style`).
/// The `tag` parameter should not include the `<` character.
pub(super) fn find_tag_start(bytes: &[u8], start: usize, tag: &[u8]) -> Option<usize> {
    let mut pos = start;
    while pos + tag.len() < bytes.len() {
        if bytes[pos] == b'<' {
            let slice = &bytes[pos + 1..pos + 1 + tag.len()];
            if slice.eq_ignore_ascii_case(tag) {
                // Make sure it's not part of a larger word (e.g., <noscript)
                let next_byte = bytes[pos + 1 + tag.len()];
                if next_byte.is_ascii_alphanumeric() || next_byte == b'_' {
                    pos += 1 + tag.len();
                    continue;
                }
                return Some(pos);
            }
        }
        pos += 1;
    }
    None
}

/// Find the end of an opening tag (the `>` character).
pub(super) fn find_tag_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut pos = start + 1; // Skip the '<'

    while pos < bytes.len() {
        match bytes[pos] {
            b'"' if !in_single_quote => in_double_quote = !in_double_quote,
            b'\'' if !in_double_quote => in_single_quote = !in_single_quote,
            b'>' if !in_single_quote && !in_double_quote => return Some(pos + 1),
            _ => {}
        }
        pos += 1;
    }
    None
}

/// Find the closing tag (e.g., `</script>` or `</style>`).
/// The `tag` parameter should not include the `<` or `/` characters.
pub(super) fn find_closing_tag(bytes: &[u8], start: usize, tag: &[u8]) -> Option<usize> {
    let mut pos = start;
    while pos + tag.len() + 2 < bytes.len() {
        if bytes[pos] == b'<'
            && bytes[pos + 1] == b'/'
            && bytes[pos + 2..pos + 2 + tag.len()].eq_ignore_ascii_case(tag)
        {
            // Find the closing '>'
            if let Some(end) = bytes[pos + 2 + tag.len()..].iter().position(|&b| b == b'>') {
                return Some(pos + 2 + tag.len() + end + 1);
            }
        }
        pos += 1;
    }
    None
}
