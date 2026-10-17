//! Applying part of a stage: keep the changes an anchor selection picks, the rest becomes context.
use super::diff::{DiffLine, Hunk, Kind, ParsedPatch, Run};

/// Which changes of a stage an apply may write. An anchor selects the change that holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeSelection {
    /// Every change.
    All,
    /// Changes holding at least one of these anchors.
    Only(Vec<String>),
    /// Changes holding none of these anchors.
    Except(Vec<String>),
}

impl ChangeSelection {
    fn selects(&self, anchors_in_change: &[String]) -> bool {
        match self {
            Self::All => true,
            Self::Only(wanted) => anchors_in_change
                .iter()
                .any(|anchor| wanted.contains(anchor)),
            Self::Except(unwanted) => !anchors_in_change
                .iter()
                .any(|anchor| unwanted.contains(anchor)),
        }
    }
}

/// Number of changes (one per match) in a unified diff.
#[must_use]
pub fn count_changes(patch: &str) -> usize {
    ParsedPatch::parse(patch)
        .hunks
        .iter()
        .flat_map(Hunk::runs)
        .map(|run| run.changes.len())
        .sum()
}

/// Keep the file header and only the changes `selection` picks. Changes it leaves out stay as
/// unchanged context, so the result applies to the original file.
///
/// `file_path` is the path anchors are computed with (the staged file), not the
/// placeholder names in the diff header.
///
/// # Examples
///
/// ```
/// use replace::anchor::{ChangeSelection, extract_anchors_from_patch, retain_changes};
///
/// let patch = "--- original\n+++ modified\n@@ -1,3 +1,3 @@\n line0\n-line1\n+line2\n line3\n@@ -5,3 +5,3 @@\n line4\n-line5\n+line6\n line7\n";
/// let anchors = extract_anchors_from_patch(patch, "test.go");
/// let filtered = retain_changes(patch, "test.go", &ChangeSelection::Only(vec![anchors[0].clone()]));
/// assert!(filtered.contains("line1") && filtered.contains("line0"));
/// assert!(!filtered.contains("line5"));
/// ```
#[must_use]
pub fn retain_changes(patch: &str, file_path: &str, selection: &ChangeSelection) -> String {
    let parsed = ParsedPatch::parse(patch);
    let mut result = String::new();
    for line in &parsed.header {
        result.push_str(line);
        result.push('\n');
    }
    for hunk in &parsed.hunks {
        if let Some(rendered) = render_hunk(hunk, file_path, selection) {
            result.push_str(&rendered);
        }
    }
    result
}

/// The hunk with only the picked changes written, or `None` when it has none.
fn render_hunk(hunk: &Hunk<'_>, file_path: &str, selection: &ChangeSelection) -> Option<String> {
    let runs = hunk.runs();
    let picks: Vec<Vec<bool>> = runs
        .iter()
        .map(|run| {
            run.changes
                .iter()
                .map(|change| selection.selects(&hunk.anchors_of(change, file_path)))
                .collect()
        })
        .collect();
    if !picks.iter().flatten().any(|&picked| picked) {
        return None;
    }

    let mut body = HunkBody::default();
    let mut cursor = 0;
    for (run, picked) in runs.iter().zip(&picks) {
        for line in &hunk.lines[cursor..run.start] {
            body.push(' ', line);
        }
        write_run(&mut body, hunk, run, picked);
        cursor = run.end;
    }
    for line in &hunk.lines[cursor..] {
        body.push(' ', line);
    }

    Some(format!(
        "@@ -{},{} +{},{} @@\n{}",
        hunk.old_start, body.old_len, hunk.new_start, body.new_len, body.text
    ))
}

fn write_run(body: &mut HunkBody, hunk: &Hunk<'_>, run: &Run, picked: &[bool]) {
    if picked.iter().all(|&picked| picked) {
        write_whole_run(body, hunk, run);
    } else {
        write_picked_changes(body, hunk, run, picked);
    }
}

fn write_whole_run(body: &mut HunkBody, hunk: &Hunk<'_>, run: &Run) {
    for line in &hunk.lines[run.start..run.end] {
        let sign = if line.kind == Kind::Removed { '-' } else { '+' };
        body.push(sign, line);
    }
}

/// Picked changes are written as removed and added lines; the others keep their old lines.
fn write_picked_changes(body: &mut HunkBody, hunk: &Hunk<'_>, run: &Run, picked: &[bool]) {
    for (change, &picked) in run.changes.iter().zip(picked) {
        for &index in &change.removed {
            body.push(if picked { '-' } else { ' ' }, &hunk.lines[index]);
        }
        if picked {
            for &index in &change.added {
                body.push('+', &hunk.lines[index]);
            }
        }
    }
}

#[derive(Default)]
struct HunkBody {
    text: String,
    old_len: usize,
    new_len: usize,
}

impl HunkBody {
    fn push(&mut self, sign: char, line: &DiffLine<'_>) {
        match sign {
            ' ' => {
                self.old_len += 1;
                self.new_len += 1;
            }
            '-' => self.old_len += 1,
            _ => self.new_len += 1,
        }
        self.text.push(sign);
        self.text.push_str(line.text);
        self.text.push('\n');
        if let Some(marker) = line.no_newline {
            self.text.push_str(marker);
            self.text.push('\n');
        }
    }
}
