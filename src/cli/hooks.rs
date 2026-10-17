//! Pre/post-apply hook commands: their context, environment and execution.
use crate::error::ReplaceError;

/// Context passed to hooks with environment variables.
#[derive(Debug, Clone)]
pub struct HookContext {
    pub stage_id: Option<String>,
    pub mode: Option<String>,
    pub files: Vec<String>,
    pub repo_root: Option<String>,
    pub apply_phase: HookPhase,
}

/// The phase of the apply process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookPhase {
    Pre,
    Post,
}

impl HookContext {
    /// Create a new `HookContext` for a pre-apply hook.
    pub const fn pre(
        stage_id: Option<String>,
        mode: Option<String>,
        files: Vec<String>,
        repo_root: Option<String>,
    ) -> Self {
        Self {
            stage_id,
            mode,
            files,
            repo_root,
            apply_phase: HookPhase::Pre,
        }
    }

    /// Create a new `HookContext` for a post-apply hook.
    pub const fn post(
        stage_id: Option<String>,
        mode: Option<String>,
        files: Vec<String>,
        repo_root: Option<String>,
    ) -> Self {
        Self {
            stage_id,
            mode,
            files,
            repo_root,
            apply_phase: HookPhase::Post,
        }
    }

    /// Convert to `HookEnv` for the hook command.
    pub fn to_hook_env(&self) -> HookEnv {
        HookEnv::from_context(self)
    }
}

/// Environment variables for hooks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HookEnv {
    pub stage_id: String,
    pub mode: String,
    pub files: String,
    pub repo_root: String,
    pub apply_phase: String,
}

impl HookEnv {
    /// Create from a `HookContext`.
    pub fn from_context(context: &HookContext) -> Self {
        let phase_str = match context.apply_phase {
            HookPhase::Pre => "pre",
            HookPhase::Post => "post",
        };

        Self {
            stage_id: context.stage_id.clone().unwrap_or_default(),
            mode: context.mode.clone().unwrap_or_default(),
            files: context.files.join("\n"),
            repo_root: context.repo_root.clone().unwrap_or_default(),
            apply_phase: phase_str.to_owned(),
        }
    }

    /// Convert to a list of (key, value) pairs for setting env vars.
    pub fn to_pairs(&self) -> Vec<(&str, &str)> {
        vec![
            ("REPLACE_STAGE_ID", self.stage_id.as_str()),
            ("REPLACE_MODE", self.mode.as_str()),
            ("REPLACE_FILES", self.files.as_str()),
            ("REPLACE_REPO_ROOT", self.repo_root.as_str()),
            ("REPLACE_APPLY_PHASE", self.apply_phase.as_str()),
        ]
    }
}

#[cfg_attr(test, mockall::automock)]
trait HookRunner {
    fn run(&mut self, command: &str, env: &HookEnv) -> Result<(), ReplaceError>;
}

struct ShellHookRunner;

impl HookRunner for ShellHookRunner {
    fn run(&mut self, command: &str, env: &HookEnv) -> Result<(), ReplaceError> {
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg(command);

        for (key, value) in env.to_pairs() {
            cmd.env(key, value);
        }

        let status = cmd.status()?;
        if !status.success() {
            return Err(ReplaceError::Message(format!(
                "hook failed with status {status}: {command}"
            )));
        }
        Ok(())
    }
}

pub(super) fn run_hooks(commands: &[String], context: &HookContext) -> Result<(), ReplaceError> {
    let mut runner = ShellHookRunner;
    run_hooks_with(commands, &mut runner, context)
}

fn run_hooks_with(
    commands: &[String],
    runner: &mut impl HookRunner,
    context: &HookContext,
) -> Result<(), ReplaceError> {
    let env = context.to_hook_env();
    for command in commands {
        runner
            .run(command, &env)
            .map_err(|error| match context.apply_phase {
                HookPhase::Pre => ReplaceError::PreHookFailed(error.to_string()),
                HookPhase::Post => ReplaceError::PostHookFailed(error.to_string()),
            })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_env_sets_all_vars() {
        let context = HookContext::post(
            Some("stage-123".to_owned()),
            Some("literal".to_owned()),
            vec!["file1.txt".to_owned(), "file2.txt".to_owned()],
            Some("/repo".to_owned()),
        );

        let env = context.to_hook_env();

        assert_eq!(env.stage_id, "stage-123");
        assert_eq!(env.mode, "literal");
        assert_eq!(env.files, "file1.txt\nfile2.txt");
        assert_eq!(env.repo_root, "/repo");
        assert_eq!(env.apply_phase, "post");
    }

    #[test]
    fn hook_env_pre_phase() {
        let context = HookContext::pre(None, None, vec![], None);
        let env = context.to_hook_env();

        assert_eq!(env.apply_phase, "pre");
    }

    #[test]
    fn hook_env_to_pairs() {
        let env = HookEnv {
            stage_id: "test".to_owned(),
            mode: "regex".to_owned(),
            files: "a.txt".to_owned(),
            repo_root: "/".to_owned(),
            apply_phase: "pre".to_owned(),
        };

        let pairs = env.to_pairs();
        assert_eq!(pairs.len(), 5);
        assert_eq!(pairs[0], ("REPLACE_STAGE_ID", "test"));
        assert_eq!(pairs[1], ("REPLACE_MODE", "regex"));
        assert_eq!(pairs[2], ("REPLACE_FILES", "a.txt"));
        assert_eq!(pairs[3], ("REPLACE_REPO_ROOT", "/"));
        assert_eq!(pairs[4], ("REPLACE_APPLY_PHASE", "pre"));
    }

    #[test]
    fn hook_env_empty_values() {
        let context = HookContext::pre(None, None, vec![], None);
        let env = context.to_hook_env();

        assert_eq!(env.stage_id, "");
        assert_eq!(env.mode, "");
        assert_eq!(env.files, "");
        assert_eq!(env.repo_root, "");
    }
}
