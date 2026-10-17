//! What happens after a rewrite command: offer to apply what was staged (or say how to), then
//! enforce `--strict`.
use super::commands::Cli;
use super::prompt::{TerminalConsole, wants_apply_now};
use super::rewrite::{RewriteOutcome, print_next_steps};
use super::settings::ResolvedSettings;
use super::{OutputOptions, stages};
use crate::error::ReplaceError;

impl Cli {
    pub(super) fn conclude_rewrite(
        &self,
        outcome: &RewriteOutcome,
        settings: &ResolvedSettings,
        output: OutputOptions,
    ) -> Result<(), ReplaceError> {
        let staged = outcome.stage_ids.len();
        if !outcome.applied_directly && staged > 0 {
            if wants_apply_now(output.json, &mut TerminalConsole, staged)? {
                stages::execute_apply_staged(&outcome.stage_ids, settings.limits.lock_timeout)?;
            } else if !output.json {
                print_next_steps(&outcome.stage_ids)?;
            }
        }
        outcome.fail_if_strict(self.exit_policy.strict)
    }
}
