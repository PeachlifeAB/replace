use crate::engine::RewriteResult;
use crate::error::ReplaceError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RhaiRewriter {
    pub script: String,
}

impl RhaiRewriter {
    /// Run the embedded Rhai script against `input`, returning the rewritten text.
    ///
    /// # Errors
    ///
    /// Returns an error if the script fails to compile or evaluate, or returns a non-string.
    pub fn rewrite(&self, input: &str) -> Result<RewriteResult, ReplaceError> {
        let engine = rhai::Engine::new();
        let mut scope = rhai::Scope::new();
        scope.push("input", input.to_owned());
        let output = engine
            .eval_with_scope::<String>(&mut scope, &self.script)
            .map_err(|error| ReplaceError::Message(error.to_string()))?;
        let replacements = usize::from(output != input);

        Ok(RewriteResult {
            output,
            replacements,
        })
    }
}
