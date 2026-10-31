//! Source positions.
//!
//! A [`Span`] is a half-open range of byte offsets into one source text. Every token and
//! every AST node carries one. Lines and columns are computed on demand by a
//! [`LineIndex`], so a span stays two integers wide.
//!
//! Spans are metadata: two trees that differ only in where they were written are equal.
//! That is why `Span`'s `PartialEq` always answers `true`, and why the round-trip test can
//! compare a tree with the tree its printed form parses to. Compare `start` and `end`
//! directly when the position itself is the point.

use std::fmt;

/// A half-open byte range `[start, end)` in a source text.
#[derive(Clone, Copy, Default, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        Span { start: start as u32, end: end as u32 }
    }

    /// The span covering both `self` and `other`.
    pub fn to(self, other: Span) -> Span {
        Span { start: self.start.min(other.start), end: self.end.max(other.end) }
    }

    /// An empty span at this span's end.
    pub fn at_end(self) -> Span {
        Span { start: self.end, end: self.end }
    }

    /// True when the two spans are the same range (ordinary `==` ignores spans).
    pub fn same_range(self, other: Span) -> bool {
        self.start == other.start && self.end == other.end
    }
}

/// Spans never make two trees unequal; see the module comment.
impl PartialEq for Span {
    fn eq(&self, _other: &Span) -> bool {
        true
    }
}

impl std::hash::Hash for Span {
    fn hash<H: std::hash::Hasher>(&self, _state: &mut H) {}
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// A one-based line and column. Columns count characters, not bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LineCol {
    pub line: u32,
    pub col: u32,
}

impl fmt::Display for LineCol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// Converts byte offsets to lines and columns.
#[derive(Clone, Debug)]
pub struct LineIndex<'a> {
    src: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    pub fn new(src: &'a str) -> LineIndex<'a> {
        let mut line_starts = vec![0];
        for (i, b) in src.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        LineIndex { src, line_starts }
    }

    /// The one-based line and column of a byte offset.
    pub fn line_col(&self, offset: u32) -> LineCol {
        let offset = (offset as usize).min(self.src.len());
        let line = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let start = self.line_starts[line];
        let col = self.src[start..offset].chars().count();
        LineCol { line: line as u32 + 1, col: col as u32 + 1 }
    }

    /// The text of a one-based line, without its newline.
    pub fn line_text(&self, line: u32) -> &'a str {
        let i = (line as usize).saturating_sub(1);
        let start = self.line_starts.get(i).copied().unwrap_or(self.src.len());
        let end = self.line_starts.get(i + 1).map(|e| e - 1).unwrap_or(self.src.len());
        &self.src[start..end.max(start)]
    }
}
