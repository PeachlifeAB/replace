//! Resolve limits, globs and walk flags once: CLI flags > replace.toml > user config.
use crate::config::{ConfigLocations, ReplaceConfig};
use crate::error::ReplaceError;
use crate::limits::{Limits, parse_file_size, parse_lock_timeout};

use super::commands::{ApplyArgs, Cli};
use super::{OutputOptions, RewriteOptions};

pub(super) struct ResolvedSettings {
    pub(super) limits: Limits,
    pub(super) include: Vec<String>,
    pub(super) exclude: Vec<String>,
    pub(super) include_hidden: bool,
    pub(super) no_ignore: bool,
    pub(super) no_default_ignores: bool,
    /// The effective merged config, for rules, hooks and language globs.
    pub(super) config: ReplaceConfig,
    /// A retired config file that was ignored, to hint about.
    pub(super) ignored_legacy_file: Option<crate::config::IgnoredLegacyFile>,
}

impl ResolvedSettings {
    /// Options for one rewrite command: its apply mode plus the resolved walk settings.
    ///
    /// `--yes` applies like `--apply` unless `--stage` asks to stage.
    pub(super) fn rewrite_options(
        &self,
        output: OutputOptions,
        apply: &ApplyArgs,
        yes: bool,
        rename_files: bool,
    ) -> RewriteOptions {
        RewriteOptions {
            stage_policy: apply.stage_policy(yes),
            rename_files,
            output,
            walk_options: self.walk_options(),
            backup: apply.backup,
            ast_options: None,
        }
    }

    pub(super) fn walk_options(&self) -> crate::walk::WalkOptions {
        crate::walk::WalkOptions {
            include: self.include.clone(),
            exclude: self.exclude.clone(),
            include_hidden: self.include_hidden,
            no_ignore: self.no_ignore,
            no_default_ignores: self.no_default_ignores,
            max_file_size: self.limits.max_file_size,
            max_files: self.limits.max_files,
            max_matches: self.limits.max_matches,
            max_matches_per_file: self.limits.max_matches_per_file,
        }
    }
}

impl Cli {
    /// Limits given as CLI flags (highest precedence).
    fn flag_limits(&self) -> Result<Limits, ReplaceError> {
        Ok(Limits {
            max_file_size: self
                .max_file_size
                .as_deref()
                .map(parse_file_size)
                .transpose()?,
            max_files: self.max_files,
            max_matches: self.max_matches,
            max_matches_per_file: self.max_matches_per_file,
            lock_timeout: self
                .lock_timeout
                .as_deref()
                .map(parse_lock_timeout)
                .transpose()?,
        })
    }

    /// Resolve settings once: CLI flags, then the merged config (flag file, or local over global).
    pub(super) fn resolve_settings(&self) -> Result<ResolvedSettings, ReplaceError> {
        let loaded = ConfigLocations::from_environment(
            self.config_path
                .as_deref()
                .map(camino::Utf8Path::as_std_path),
        )?
        .load()?;
        let config = loaded.config;
        let first_non_empty = |cli: &[String], configured: &[String]| {
            if cli.is_empty() {
                configured.to_vec()
            } else {
                cli.to_vec()
            }
        };

        let mut limits = self.flag_limits()?.or(config.limits);
        limits.max_files = limits.max_files.or_else(crate::env_vars::ast_max_files);

        Ok(ResolvedSettings {
            limits,
            include: first_non_empty(&self.include, &config.include),
            exclude: first_non_empty(&self.exclude, &config.exclude),
            include_hidden: self.walk_flags.hidden || config.include_hidden,
            no_ignore: self.walk_flags.no_ignore,
            no_default_ignores: self.walk_flags.no_default_ignores,
            ignored_legacy_file: loaded.ignored_legacy_file,
            config,
        })
    }
}
