//! A unified diff as lines with their positions, grouped into the changes an anchor selects.
use super::compute_anchor_hash;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Context,
    Removed,
    Added,
}

pub(super) struct DiffLine<'a> {
    pub(super) raw: &'a str,
    pub(super) kind: Kind,
    pub(super) text: &'a str,
    /// Line number in the old file for context and removed lines, in the new file for added ones.
    pub(super) number: usize,
    /// The `\ No newline at end of file` marker that follows this line.
    pub(super) no_newline: Option<&'a str>,
}

impl DiffLine<'_> {
    pub(super) fn anchor(&self, file_path: &str) -> Option<String> {
        let sign = match self.kind {
            Kind::Context => return None,
            Kind::Removed => '-',
            Kind::Added => '+',
        };
        let content = format!("{sign}{}:{}", self.number, self.text);
        Some(format!(
            "{file_path}#{}",
            compute_anchor_hash(file_path, &content)
        ))
    }
}

pub(super) struct Hunk<'a> {
    pub(super) header: &'a str,
    pub(super) old_start: usize,
    pub(super) new_start: usize,
    pub(super) lines: Vec<DiffLine<'a>>,
}

/// Consecutive removed and added lines, `lines[start..end]`, and the changes inside them.
pub(super) struct Run {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) changes: Vec<Change>,
}

/// What one match did: the line indexes it removed and added. Anchors select whole changes.
pub(super) struct Change {
    pub(super) removed: Vec<usize>,
    pub(super) added: Vec<usize>,
}

impl Hunk<'_> {
    pub(super) fn runs(&self) -> Vec<Run> {
        let mut runs = Vec::new();
        let mut index = 0;
        while index < self.lines.len() {
            if self.lines[index].kind == Kind::Context {
                index += 1;
                continue;
            }
            let start = index;
            while index < self.lines.len() && self.lines[index].kind != Kind::Context {
                index += 1;
            }
            runs.push(Run {
                start,
                end: index,
                changes: self.changes_in(start..index),
            });
        }
        runs
    }

    /// A run with as many removed as added lines pairs them in order, one change per pair;
    /// any other run is a single change.
    fn changes_in(&self, span: std::ops::Range<usize>) -> Vec<Change> {
        let in_kind = |kind| {
            span.clone()
                .filter(|&index| self.lines[index].kind == kind)
                .collect::<Vec<_>>()
        };
        let removed = in_kind(Kind::Removed);
        let added = in_kind(Kind::Added);
        if removed.len() == added.len() && removed.len() > 1 {
            removed
                .into_iter()
                .zip(added)
                .map(|(removed, added)| Change {
                    removed: vec![removed],
                    added: vec![added],
                })
                .collect()
        } else {
            vec![Change { removed, added }]
        }
    }

    pub(super) fn anchors_of(&self, change: &Change, file_path: &str) -> Vec<String> {
        change
            .removed
            .iter()
            .chain(&change.added)
            .filter_map(|&index| self.lines[index].anchor(file_path))
            .collect()
    }
}

/// A unified diff of one file, split into its header lines and hunks.
pub(super) struct ParsedPatch<'a> {
    pub(super) header: Vec<&'a str>,
    pub(super) hunks: Vec<Hunk<'a>>,
}

impl<'a> ParsedPatch<'a> {
    pub(super) fn parse(patch: &'a str) -> Self {
        let mut header = Vec::new();
        let mut hunks: Vec<Hunk<'a>> = Vec::new();
        let mut cursor = LineCursor::default();
        for line in patch.lines() {
            if line.starts_with("@@") {
                let (old_start, new_start) = range_starts(line);
                cursor = LineCursor {
                    old: old_start,
                    new: new_start,
                };
                hunks.push(Hunk {
                    header: line,
                    old_start,
                    new_start,
                    lines: Vec::new(),
                });
            } else if let Some(hunk) = hunks.last_mut() {
                hunk.push_line(line, &mut cursor);
            } else {
                header.push(line);
            }
        }
        Self { header, hunks }
    }
}

impl<'a> Hunk<'a> {
    fn push_line(&mut self, line: &'a str, cursor: &mut LineCursor) {
        if line.starts_with('\\') {
            if let Some(previous) = self.lines.last_mut() {
                previous.no_newline = Some(line);
            }
            return;
        }
        self.lines.push(DiffLine::read(line, cursor));
    }
}

impl<'a> DiffLine<'a> {
    fn read(raw: &'a str, cursor: &mut LineCursor) -> Self {
        let (kind, text) = match raw.as_bytes().first() {
            Some(b'-') => (Kind::Removed, &raw[1..]),
            Some(b'+') => (Kind::Added, &raw[1..]),
            Some(_) => (Kind::Context, &raw[1..]),
            None => (Kind::Context, ""),
        };
        Self {
            raw,
            kind,
            text,
            number: cursor.advance(kind),
            no_newline: None,
        }
    }
}

/// The next line number in the old and in the new file while reading a hunk.
#[derive(Default)]
struct LineCursor {
    old: usize,
    new: usize,
}

impl LineCursor {
    /// The number of a line of `kind`, then step past it.
    const fn advance(&mut self, kind: Kind) -> usize {
        match kind {
            Kind::Context => {
                let number = self.old;
                self.old += 1;
                self.new += 1;
                number
            }
            Kind::Removed => {
                self.old += 1;
                self.old - 1
            }
            Kind::Added => {
                self.new += 1;
                self.new - 1
            }
        }
    }
}

/// The old and new start lines of an `@@ -a[,b] +c[,d] @@` header.
fn range_starts(header: &str) -> (usize, usize) {
    let start_of = |sign: char| {
        header
            .split_whitespace()
            .find_map(|token| token.strip_prefix(sign))
            .and_then(|range| range.split(',').next())
            .and_then(|start| start.parse().ok())
            .unwrap_or(1)
    };
    (start_of('-'), start_of('+'))
}
