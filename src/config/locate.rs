//! Where config layers live: the flag file, the project's `.replace/`, and the user's global dir.
use std::path::{Path, PathBuf};

use super::{CONFIG_FILE_NAME, ConfigLayer, ReplaceConfig};
use crate::error::ReplaceError;

/// Directory inside a project (and the global fallback home) that holds `replace` state.
pub const REPLACE_DIR_NAME: &str = ".replace";
const GLOBAL_DIR_NAME: &str = "replace";
const DOT_CONFIG_DIR_NAME: &str = ".config";
const LEGACY_PROJECT_FILES: [&str; 2] = ["replace.toml", ".replace.toml"];
const MAX_ANCESTORS_SEARCHED: usize = 5;

/// Where the global config lives under `$HOME` when `XDG_CONFIG_HOME` is unset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeFallback {
    /// `~/.replace/`: the hidden-folder convention on macOS.
    HiddenDir,
    /// `~/.config/replace/`: the XDG default on Linux and other Unixes.
    DotConfig,
}

impl HomeFallback {
    /// The convention of the platform this binary runs on.
    #[must_use]
    pub const fn for_current_platform() -> Self {
        if cfg!(target_os = "macos") {
            Self::HiddenDir
        } else {
            Self::DotConfig
        }
    }
}

/// Everything that decides which config files apply, resolved once at the boundary.
#[derive(Debug, Clone)]
pub struct ConfigLocations {
    /// `--config-path`: when set, the only config that is read.
    pub flag_file: Option<PathBuf>,
    /// Directory the local search starts from.
    pub project_dir: PathBuf,
    pub xdg_config_home: Option<PathBuf>,
    pub home: Option<PathBuf>,
    pub home_fallback: HomeFallback,
}

/// A config file from an earlier release that is no longer read, and where it belongs now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoredLegacyFile {
    pub path: PathBuf,
    pub move_to: PathBuf,
}

/// The merged configuration plus a pointer to a retired config file, if one was found.
#[derive(Debug)]
pub struct LoadedConfig {
    pub config: ReplaceConfig,
    pub ignored_legacy_file: Option<IgnoredLegacyFile>,
}

impl ConfigLocations {
    /// Read the environment and working directory once.
    ///
    /// # Errors
    ///
    /// Returns an error if the working directory cannot be determined.
    pub fn from_environment(flag_file: Option<&Path>) -> Result<Self, ReplaceError> {
        Ok(Self {
            flag_file: flag_file.map(Path::to_path_buf),
            project_dir: std::env::current_dir()?,
            xdg_config_home: crate::env_vars::xdg_config_home(),
            home: crate::env_vars::home_dir(),
            home_fallback: HomeFallback::for_current_platform(),
        })
    }

    /// Global config: `$XDG_CONFIG_HOME/replace/config.toml`, else under `$HOME` by platform
    /// (`~/.replace/` on macOS, `~/.config/replace/` elsewhere).
    #[must_use]
    pub fn global_file(&self) -> Option<PathBuf> {
        let global_dir = match (&self.xdg_config_home, &self.home) {
            (Some(xdg), _) => xdg.join(GLOBAL_DIR_NAME),
            (None, Some(home)) => match self.home_fallback {
                HomeFallback::HiddenDir => home.join(REPLACE_DIR_NAME),
                HomeFallback::DotConfig => home.join(DOT_CONFIG_DIR_NAME).join(GLOBAL_DIR_NAME),
            },
            (None, None) => return None,
        };
        Some(global_dir.join(CONFIG_FILE_NAME))
    }

    /// Nearest `.replace/config.toml` at or above the project directory.
    #[must_use]
    pub fn local_file(&self) -> Option<PathBuf> {
        self.ancestors()
            .map(|dir| dir.join(REPLACE_DIR_NAME).join(CONFIG_FILE_NAME))
            .find(|candidate| candidate.is_file())
    }

    fn legacy_project_file(&self) -> Option<IgnoredLegacyFile> {
        let (dir, path) = self.ancestors().find_map(|dir| {
            LEGACY_PROJECT_FILES
                .map(|name| dir.join(name))
                .into_iter()
                .find(|candidate| candidate.is_file())
                .map(|path| (dir, path))
        })?;
        Some(IgnoredLegacyFile {
            path,
            move_to: dir.join(REPLACE_DIR_NAME).join(CONFIG_FILE_NAME),
        })
    }

    /// `~/.replace/config.toml` left from when Linux shared the macOS location.
    fn legacy_global_file(&self) -> Option<IgnoredLegacyFile> {
        let home = self.home.as_ref()?;
        let current = self.global_file()?;
        let legacy = home.join(REPLACE_DIR_NAME).join(CONFIG_FILE_NAME);
        let stranded = self.xdg_config_home.is_none()
            && self.home_fallback == HomeFallback::DotConfig
            && legacy.is_file()
            && !current.is_file();
        stranded.then_some(IgnoredLegacyFile {
            path: legacy,
            move_to: current,
        })
    }

    fn ancestors(&self) -> impl Iterator<Item = &Path> {
        self.project_dir
            .ancestors()
            .take(MAX_ANCESTORS_SEARCHED + 1)
    }

    /// Merge flag, local and global layers.
    ///
    /// # Errors
    ///
    /// Returns an error if the flag file is missing or not a file, or if any file read is
    /// not valid config.
    pub fn load(&self) -> Result<LoadedConfig, ReplaceError> {
        if let Some(flag_file) = &self.flag_file {
            if !flag_file.is_file() {
                return Err(ReplaceError::InvalidArgs(format!(
                    "--config-path is not a config file: {}",
                    flag_file.display()
                )));
            }
            return Ok(LoadedConfig {
                config: ConfigLayer::from_file(flag_file)?.resolve(),
                ignored_legacy_file: None,
            });
        }

        let local_file = self.local_file();
        let local = read_optional(local_file.as_deref())?;
        let global = read_optional(self.global_file().as_deref())?;
        let ignored_legacy_file = if local_file.is_none() {
            self.legacy_project_file()
        } else {
            None
        }
        .or_else(|| self.legacy_global_file());
        Ok(LoadedConfig {
            config: local.or(global).resolve(),
            ignored_legacy_file,
        })
    }
}

fn read_optional(path: Option<&Path>) -> Result<ConfigLayer, ReplaceError> {
    match path {
        Some(path) if path.is_file() => ConfigLayer::from_file(path),
        _ => Ok(ConfigLayer::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locations(xdg_config_home: Option<&str>, home_fallback: HomeFallback) -> ConfigLocations {
        ConfigLocations {
            flag_file: None,
            project_dir: PathBuf::from("/project"),
            xdg_config_home: xdg_config_home.map(PathBuf::from),
            home: Some(PathBuf::from("/home/dev")),
            home_fallback,
        }
    }

    #[test]
    fn xdg_config_home_wins_on_every_platform() {
        for fallback in [HomeFallback::HiddenDir, HomeFallback::DotConfig] {
            assert_eq!(
                locations(Some("/xdg"), fallback).global_file(),
                Some(PathBuf::from("/xdg/replace/config.toml"))
            );
        }
    }

    #[test]
    fn macos_style_fallback_is_a_hidden_dir_in_home() {
        assert_eq!(
            locations(None, HomeFallback::HiddenDir).global_file(),
            Some(PathBuf::from("/home/dev/.replace/config.toml"))
        );
    }

    #[test]
    fn linux_style_fallback_is_dot_config_in_home() {
        assert_eq!(
            locations(None, HomeFallback::DotConfig).global_file(),
            Some(PathBuf::from("/home/dev/.config/replace/config.toml"))
        );
    }

    fn home_with_legacy_global_config() -> tempfile::TempDir {
        let home = tempfile::tempdir().expect("temp home");
        std::fs::create_dir_all(home.path().join(".replace")).expect("legacy dir");
        std::fs::write(home.path().join(".replace/config.toml"), "").expect("legacy config");
        home
    }

    fn locations_in(home: &Path, home_fallback: HomeFallback) -> ConfigLocations {
        ConfigLocations {
            flag_file: None,
            project_dir: home.join("project"),
            xdg_config_home: None,
            home: Some(home.to_path_buf()),
            home_fallback,
        }
    }

    #[test]
    fn linux_style_home_with_a_macos_style_global_config_is_told_where_to_move_it() {
        let home = home_with_legacy_global_config();

        let loaded = locations_in(home.path(), HomeFallback::DotConfig)
            .load()
            .expect("load");

        let ignored = loaded
            .ignored_legacy_file
            .expect("legacy global config is reported");
        assert_eq!(ignored.path, home.path().join(".replace/config.toml"));
        assert_eq!(
            ignored.move_to,
            home.path().join(".config/replace/config.toml")
        );
    }

    #[test]
    fn macos_style_home_reads_that_file_and_reports_nothing() {
        let home = home_with_legacy_global_config();

        let loaded = locations_in(home.path(), HomeFallback::HiddenDir)
            .load()
            .expect("load");

        assert!(loaded.ignored_legacy_file.is_none());
    }

    #[test]
    fn nothing_is_reported_once_the_new_global_config_exists() {
        let home = home_with_legacy_global_config();
        std::fs::create_dir_all(home.path().join(".config/replace")).expect("dir");
        std::fs::write(home.path().join(".config/replace/config.toml"), "").expect("config");

        let loaded = locations_in(home.path(), HomeFallback::DotConfig)
            .load()
            .expect("load");

        assert!(loaded.ignored_legacy_file.is_none());
    }
}
