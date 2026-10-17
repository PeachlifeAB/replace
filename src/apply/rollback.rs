//! Undo log for a batch of file writes: every write or rename records how to reverse it, and a
//! failed batch reverses what it already did, newest first.
use std::fs;

use camino::Utf8PathBuf;

use super::write_atomic;
use crate::error::ReplaceError;

enum UndoStep {
    Restore {
        path: Utf8PathBuf,
        original_content: String,
    },
    Rename {
        from: Utf8PathBuf,
        to: Utf8PathBuf,
    },
}

impl UndoStep {
    fn undo(self) -> Result<(), RollbackFailure> {
        match self {
            Self::Restore {
                path,
                original_content,
            } => write_atomic(&path, &original_content)
                .map_err(|cause| RollbackFailure { path, cause }),
            Self::Rename { from, to } => fs::rename(&to, &from).map_err(|error| RollbackFailure {
                path: from,
                cause: error.into(),
            }),
        }
    }
}

/// A step that could not be reversed; `path` is the file the batch left changed.
pub struct RollbackFailure {
    pub(crate) path: Utf8PathBuf,
    pub(crate) cause: ReplaceError,
}

/// The writes and renames of one batch, each recorded so that [`WriteJournal::roll_back`] can
/// undo it.
#[derive(Default)]
pub struct WriteJournal {
    steps: Vec<UndoStep>,
}

impl WriteJournal {
    pub(crate) const fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Write `content` to `path`. The write is undone by restoring `original_content`, when the
    /// file had any.
    pub(crate) fn write(
        &mut self,
        path: &Utf8PathBuf,
        content: &str,
        original_content: Option<&str>,
    ) -> Result<(), ReplaceError> {
        write_atomic(path, content)?;
        if let Some(original) = original_content {
            self.steps.push(UndoStep::Restore {
                path: path.clone(),
                original_content: original.to_owned(),
            });
        }
        Ok(())
    }

    /// Rename `from` to `to`; the rename is undone by renaming back.
    pub(crate) fn rename(
        &mut self,
        from: &Utf8PathBuf,
        to: &Utf8PathBuf,
    ) -> Result<(), ReplaceError> {
        fs::rename(from, to)?;
        self.steps.push(UndoStep::Rename {
            from: from.clone(),
            to: to.clone(),
        });
        Ok(())
    }

    /// Undo every recorded step, newest first, and return the steps that could not be undone.
    pub(crate) fn roll_back(&mut self) -> Vec<RollbackFailure> {
        self.steps
            .drain(..)
            .rev()
            .filter_map(|step| step.undo().err())
            .collect()
    }
}
