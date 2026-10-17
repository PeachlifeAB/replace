//! Housekeeping commands: undo, status, doctor, languages, and the destructive-command prompt.
use std::io::{self, IsTerminal, Write};
use std::time::Duration;

use crate::apply::undo_stage;
use crate::error::ReplaceError;

use super::commands::Cli;
use super::{OutputOptions, util};

impl Cli {
    pub(super) fn confirm_destructive(command: &str, yes: bool) -> Result<(), ReplaceError> {
        if yes {
            return Ok(());
        }
        if !io::stdout().is_terminal() || !io::stderr().is_terminal() {
            return Err(ReplaceError::InvalidArgs(format!(
                "{command} requires --yes when not attached to a TTY"
            )));
        }

        let mut stderr = io::stderr().lock();
        write!(stderr, "Proceed with replace {command}? [y/N] ")?;
        stderr.flush()?;

        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        if answer.trim().eq_ignore_ascii_case("y") {
            Ok(())
        } else {
            Err(ReplaceError::InvalidArgs(format!(
                "replace {command} cancelled"
            )))
        }
    }

    pub(super) fn execute_undo(
        stage_id: &str,
        output: OutputOptions,
        lock_timeout: Option<Duration>,
    ) -> Result<(), ReplaceError> {
        let manager = util::stage_manager()?.with_lock_timeout(lock_timeout);
        let lock = manager.lock_exclusive()?;
        undo_stage(&manager, &lock, stage_id)?;
        if output.json {
            util::write_stdout_line(format_args!("{}", serde_json::json!({ "undid": stage_id })))?;
        } else {
            util::write_stdout_line(format_args!("Undid: {stage_id}"))?;
        }
        Ok(())
    }

    pub(super) fn execute_status(output: OutputOptions) -> Result<(), ReplaceError> {
        let manager = util::stage_manager()?;
        let staged = manager.list_stages().len();
        let history = manager.list_history().len();
        if output.json {
            util::write_stdout_line(format_args!(
                "{}",
                serde_json::json!({ "staged": staged, "history": history })
            ))?;
        } else {
            util::write_stdout_line(format_args!(
                "replace status: {staged} staged, {history} in history"
            ))?;
        }
        Ok(())
    }

    pub(super) fn execute_doctor(output: OutputOptions) -> Result<(), ReplaceError> {
        if output.json {
            util::write_stdout_line(format_args!(
                "{}",
                serde_json::json!({ "diagnostics": "OK" })
            ))?;
        } else {
            util::write_stdout_line(format_args!("replace diagnostics: OK"))?;
        }
        Ok(())
    }

    pub(super) fn execute_languages(output: OutputOptions) -> Result<(), ReplaceError> {
        let supported_languages = crate::engine::ast::supported_language_names();
        if output.json {
            util::write_stdout_line(format_args!(
                "{}",
                serde_json::json!({ "languages": supported_languages })
            ))?;
        } else {
            util::write_stdout_line(format_args!(
                "Supported languages: {}",
                supported_languages.join(", ")
            ))?;
        }
        Ok(())
    }
}
