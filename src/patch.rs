use crate::error::ReplaceError;

#[must_use]
pub fn unified_diff(original: &str, modified: &str) -> String {
    diffy::create_patch(original, modified).to_string()
}

#[must_use]
pub fn reverse_unified_diff(original: &str, modified: &str) -> String {
    diffy::create_patch(modified, original).to_string()
}

/// Apply a unified diff `patch` to `base`, returning the patched text.
///
/// # Errors
///
/// Returns an error if the patch is malformed or cannot be applied to `base`.
pub fn apply_unified_patch(base: &str, patch: &str) -> Result<String, ReplaceError> {
    let patch =
        diffy::Patch::from_str(patch).map_err(|error| ReplaceError::Message(error.to_string()))?;
    diffy::apply(base, &patch).map_err(|error| ReplaceError::Message(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unified_patch_should_round_trip() -> Result<(), ReplaceError> {
        let original = "one\ntwo\n";
        let modified = "one\nthree\n";
        let patch = unified_diff(original, modified);

        let result = apply_unified_patch(original, &patch)?;

        assert_eq!(result, modified);
        Ok(())
    }

    #[test]
    fn reverse_unified_diff_should_restore_original() -> Result<(), ReplaceError> {
        let original = "one\ntwo\n";
        let modified = "one\nthree\n";
        let patch = reverse_unified_diff(original, modified);

        let result = apply_unified_patch(modified, &patch)?;

        assert_eq!(result, original);
        Ok(())
    }
}
