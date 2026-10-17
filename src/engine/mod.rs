pub mod ast;
pub mod inject;
pub mod literal;
pub mod regex;
pub mod rewriter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteResult {
    pub output: String,
    pub replacements: usize,
}

impl RewriteResult {
    pub fn unchanged(input: impl Into<String>) -> Self {
        Self {
            output: input.into(),
            replacements: 0,
        }
    }
}
