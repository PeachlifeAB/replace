use assert_cmd::Command;
use assert_fs::TempDir;
use assert_fs::prelude::*;
use predicates::prelude::*;

const LARGE_FILE_BYTES: usize = 3 * 1024;

struct Workspace {
    project: TempDir,
    xdg_config_home: TempDir,
}

impl Workspace {
    fn new() -> Self {
        let project = TempDir::new().expect("project dir");
        let body = format!("foo\n{}\n", "x".repeat(LARGE_FILE_BYTES));
        project
            .child("large.txt")
            .write_str(&body)
            .expect("fixture");
        Self {
            project,
            xdg_config_home: TempDir::new().expect("xdg dir"),
        }
    }

    fn write_user_config(&self, toml: &str) {
        self.xdg_config_home
            .child("replace/config.toml")
            .write_str(toml)
            .expect("user config");
    }

    fn write_project_config(&self, toml: &str) {
        self.project
            .child(".replace/config.toml")
            .write_str(toml)
            .expect("project config");
    }

    fn replace(&self) -> Command {
        let mut command = Command::cargo_bin("replace").expect("binary exists");
        command
            .current_dir(self.project.path())
            .env("XDG_CONFIG_HOME", self.xdg_config_home.path())
            .env("HOME", self.xdg_config_home.path());
        command
    }

    fn discard_stages(&self) {
        self.replace().args(["discard", "-y"]).assert().success();
    }
}

const LITERAL_RULE: &str = "[[rules]]\ntype = \"literal\"\nfind = \"foo\"\nreplace = \"bar\"\n";

#[test]
fn user_config_limits_skip_large_files() {
    let workspace = Workspace::new();
    workspace.write_user_config("[limits]\nmax_file_size = \"1KiB\"\n");
    workspace
        .replace()
        .args(["literal", "foo", "bar", "."])
        .assert()
        .success()
        .stderr(predicates::str::contains("Skipping large file"));
}

#[test]
fn cli_flag_overrides_user_config() {
    let workspace = Workspace::new();
    workspace.write_user_config("[limits]\nmax_file_size = \"1KiB\"\n");
    workspace
        .replace()
        .args(["--max-file-size", "1MiB", "literal", "foo", "bar", "."])
        .assert()
        .success()
        .stdout(predicates::str::contains("Staged:"));
}

#[test]
fn project_config_overrides_user_config() {
    let workspace = Workspace::new();
    workspace.write_user_config("[limits]\nmax_file_size = \"1KiB\"\n");
    workspace.write_project_config(&format!(
        "[limits]\nmax_file_size = \"1MiB\"\n{LITERAL_RULE}"
    ));
    workspace
        .replace()
        .arg("run")
        .assert()
        .success()
        .stdout(predicates::str::contains("large.txt"));
}

#[test]
fn user_config_rejects_unknown_keys() {
    let workspace = Workspace::new();
    workspace.write_user_config("bogus = 1\n");
    workspace
        .replace()
        .arg("status")
        .assert()
        .failure()
        .stderr(predicates::str::contains("REPLACE_E_CONFIG"))
        .stderr(predicates::str::contains("bogus"));
}

#[test]
fn missing_user_config_changes_nothing() {
    let workspace = Workspace::new();
    workspace
        .replace()
        .args(["literal", "foo", "bar", "."])
        .assert()
        .success()
        .stdout(predicates::str::contains("Staged:"));
    workspace.discard_stages();
}

#[test]
fn user_config_exclude_and_lock_timeout_apply() {
    let workspace = Workspace::new();
    workspace.write_user_config("exclude = [\"large.txt\"]\n[limits]\nlock_timeout = \"30s\"\n");
    workspace
        .replace()
        .args(["literal", "foo", "bar", "."])
        .assert()
        .success()
        .stdout(predicates::str::contains("Staged:").not());
}

#[test]
fn shipped_example_config_validates_against_schema() {
    let schema: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string("schema/replace.config.json").expect("schema"),
    )
    .expect("schema json");
    let example: toml::Value =
        toml::from_str(&std::fs::read_to_string("config.example.toml").expect("example"))
            .expect("example toml");
    let instance = serde_json::to_value(example).expect("toml to json");
    let validator = jsonschema::draft202012::new(&schema).expect("compile schema");
    assert!(validator.is_valid(&instance), "example violates schema");
}

const TINY_LIMIT_CONFIG: &str = "[limits]\nmax_file_size = \"1KiB\"\n";
const ROOMY_LIMIT_CONFIG: &str = "[limits]\nmax_file_size = \"1MiB\"\n";

#[test]
fn config_path_flag_names_the_only_config_that_is_read() {
    let workspace = Workspace::new();
    workspace.write_user_config("[limits]\nmax_file_size = \"1KiB\"\nmax_files = 1\n");
    workspace
        .project
        .child("second.txt")
        .write_str("foo\n")
        .expect("fixture");
    workspace
        .project
        .child(".replace/config.toml")
        .write_str(TINY_LIMIT_CONFIG)
        .expect("local config");
    let explicit = TempDir::new().expect("explicit dir");
    explicit
        .child("ci.toml")
        .write_str(ROOMY_LIMIT_CONFIG)
        .expect("explicit config");
    workspace
        .replace()
        .arg("--config-path")
        .arg(explicit.child("ci.toml").path())
        .args(["literal", "foo", "bar", "."])
        .assert()
        .success()
        .stdout(predicates::str::contains("Staged:"));
}

#[test]
fn config_path_to_a_missing_file_is_an_invalid_argument() {
    let workspace = Workspace::new();
    workspace
        .replace()
        .args(["--config-path", "/definitely/not/here.toml", "status"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicates::str::contains("/definitely/not/here.toml"));
}

#[test]
fn config_path_to_a_directory_is_an_invalid_argument() {
    let workspace = Workspace::new();
    let directory = TempDir::new().expect("dir");
    workspace
        .replace()
        .arg("--config-path")
        .arg(directory.path())
        .arg("status")
        .assert()
        .failure()
        .code(2)
        .stderr(predicates::str::contains("REPLACE_E_INVALID_ARGS"));
}

#[test]
fn config_path_with_invalid_toml_is_a_config_error() {
    let workspace = Workspace::new();
    let broken = TempDir::new().expect("dir");
    broken
        .child("bad.toml")
        .write_str("not = [valid")
        .expect("bad config");
    workspace
        .replace()
        .arg("--config-path")
        .arg(broken.child("bad.toml").path())
        .arg("status")
        .assert()
        .failure()
        .stderr(predicates::str::contains("REPLACE_E_CONFIG"));
}

#[test]
fn local_config_overrides_global_per_key_and_inherits_the_rest() {
    let workspace = Workspace::new();
    workspace.write_user_config("[limits]\nmax_file_size = \"1KiB\"\nmax_files = 1\n");
    workspace.write_project_config(ROOMY_LIMIT_CONFIG);
    workspace
        .project
        .child("second.txt")
        .write_str("foo\n")
        .expect("fixture");
    workspace
        .replace()
        .args(["literal", "foo", "bar", "."])
        .assert()
        .failure()
        .stderr(predicates::str::contains("Limit reached: scanned 1 files"));
}

#[test]
fn local_config_overrides_global_for_the_keys_it_sets() {
    let workspace = Workspace::new();
    workspace.write_user_config(TINY_LIMIT_CONFIG);
    workspace.write_project_config(ROOMY_LIMIT_CONFIG);
    workspace
        .replace()
        .args(["literal", "foo", "bar", "."])
        .assert()
        .success()
        .stdout(predicates::str::contains("large.txt"));
}

/// Where the global config lives under `$HOME` when `XDG_CONFIG_HOME` is unset.
const HOME_FALLBACK_CONFIG: &str = if cfg!(target_os = "macos") {
    ".replace/config.toml"
} else {
    ".config/replace/config.toml"
};

#[test]
fn global_config_falls_back_to_the_platform_dir_in_home_when_xdg_is_unset() {
    let workspace = Workspace::new();
    workspace
        .xdg_config_home
        .child(HOME_FALLBACK_CONFIG)
        .write_str(TINY_LIMIT_CONFIG)
        .expect("home config");
    workspace
        .replace()
        .env_remove("XDG_CONFIG_HOME")
        .args(["literal", "foo", "bar", "."])
        .assert()
        .success()
        .stderr(predicates::str::contains("Skipping large file"));
}

#[test]
fn local_config_is_found_from_a_subdirectory() {
    let workspace = Workspace::new();
    workspace.write_project_config(TINY_LIMIT_CONFIG);
    workspace
        .project
        .child("sub/inner.txt")
        .write_str("foo\n")
        .expect("fixture");
    workspace
        .replace()
        .current_dir(workspace.project.child("sub").path())
        .args(["literal", "foo", "bar", ".."])
        .assert()
        .success()
        .stderr(predicates::str::contains("Skipping large file"));
}

#[test]
fn retired_replace_toml_is_not_read_but_explained() {
    let workspace = Workspace::new();
    workspace
        .project
        .child("replace.toml")
        .write_str(TINY_LIMIT_CONFIG)
        .expect("legacy config");
    workspace
        .replace()
        .args(["literal", "foo", "bar", "."])
        .assert()
        .success()
        .stdout(predicates::str::contains("large.txt"))
        .stderr(predicates::str::contains(".replace/config.toml"));
}

#[test]
fn run_applies_rules_from_the_local_config() {
    let workspace = Workspace::new();
    workspace.write_project_config(LITERAL_RULE);
    workspace
        .replace()
        .arg("run")
        .assert()
        .success()
        .stdout(predicates::str::contains("Run complete"));
}

#[test]
fn run_without_any_rules_fails_with_guidance() {
    let workspace = Workspace::new();
    workspace
        .replace()
        .arg("run")
        .assert()
        .failure()
        .code(2)
        .stderr(predicates::str::contains(".replace/config.toml"));
}
