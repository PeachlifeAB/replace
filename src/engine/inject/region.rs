use ast_grep_language::SupportLang;

/// An injected language region within a host document.
#[derive(Debug, Clone)]
pub struct InjectedRegion {
    /// Byte offset of the region start (after the opening tag).
    pub start: usize,
    /// Byte offset of the region end (before the closing tag).
    pub end: usize,
    /// The language to use for parsing this region.
    pub language: SupportLang,
}
