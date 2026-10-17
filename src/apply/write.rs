//! Atomic file writes: content lands as given, except that a file written entirely with CRLF
//! line breaks stays that way.
use camino::Utf8PathBuf;

use crate::error::ReplaceError;

const CRLF: &str = "\r\n";

/// Whether `text` has line breaks and every one of them is CRLF.
pub(super) fn is_uniformly_crlf(text: &str) -> bool {
    text.contains(CRLF) && text.matches('\n').count() == text.matches(CRLF).count()
}

/// `content` with every line break as CRLF.
pub(super) fn with_crlf_line_breaks(content: &str) -> String {
    content.replace(CRLF, "\n").replace('\n', CRLF)
}

pub fn write_atomic(path: &Utf8PathBuf, content: &str) -> Result<(), ReplaceError> {
    let parent = path.parent().ok_or_else(|| {
        ReplaceError::InvalidArgs(format!("path has no parent directory: {path}"))
    })?;

    let keeps_crlf =
        std::fs::read_to_string(path).is_ok_and(|original| is_uniformly_crlf(&original));
    let final_content = if keeps_crlf {
        with_crlf_line_breaks(content)
    } else {
        content.to_owned()
    };

    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    // Preserve original file permissions before replacing the inode.
    if let Ok(metadata) = std::fs::metadata(path) {
        let _ = temp.as_file().set_permissions(metadata.permissions());
    }
    std::io::Write::write_all(&mut temp, final_content.as_bytes())?;
    temp.persist(path).map_err(|error| {
        ReplaceError::Rollback(format!("atomic write failed for {path}: {}", error.error))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests;
