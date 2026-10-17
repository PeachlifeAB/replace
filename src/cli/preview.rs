//! The diff printed right after staging, shortened so a large change does not flood the screen.
use std::fmt::Write as _;

/// A preview with more lines than this is folded.
const FOLD_ABOVE_LINES: usize = 80;
/// Lines of a folded preview that stay visible.
const FOLDED_KEEP_LINES: usize = 40;
/// Characters of a line kept before it is cut.
const MAX_LINE_CHARS: usize = 160;
const CUT_MARKER: &str = "...";

/// `rendered` with long lines cut and, when it is long, folded after its first lines.
/// `replace show STAGE_ID` and `--expand` print it whole.
pub(super) fn condense(rendered: &str, stage_id: &str) -> String {
    let lines: Vec<&str> = rendered.lines().collect();
    let shown = if lines.len() > FOLD_ABOVE_LINES {
        &lines[..FOLDED_KEEP_LINES]
    } else {
        &lines[..]
    };
    let mut condensed = String::new();
    for line in shown {
        condensed.push_str(&cut(line));
        condensed.push('\n');
    }
    let hidden = lines.len() - shown.len();
    if hidden > 0 {
        writeln!(
            condensed,
            "... {hidden} more lines folded (replace show {stage_id}, or --expand)"
        )
        .expect("writing to String cannot fail");
    }
    condensed
}

fn cut(line: &str) -> String {
    match line.char_indices().nth(MAX_LINE_CHARS) {
        Some((end, _)) => format!("{}{CUT_MARKER}", &line[..end]),
        None => line.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cut_keeps_whole_characters() {
        let line = "å".repeat(MAX_LINE_CHARS + 1);

        assert_eq!(
            cut(&line),
            format!("{}{CUT_MARKER}", "å".repeat(MAX_LINE_CHARS))
        );
    }
}
