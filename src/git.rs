use std::path::Path;

use crate::error::ReplaceError;

/// Discover the git repository containing `path`.
///
/// # Errors
///
/// Returns an error if no enclosing git repository is found.
pub fn discover(path: impl AsRef<Path>) -> Result<gix::Repository, ReplaceError> {
    gix::discover(path.as_ref()).map_err(|error| ReplaceError::Git(error.to_string()))
}
