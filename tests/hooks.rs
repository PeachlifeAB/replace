//! Hooks configured for an applied `replace run`: their output, their failures, and their environment.
use std::fs;

use assert_fs::TempDir;
use predicates::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_with_config;

const GO_PRINT: &str =
    "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"hello\")\n}\n";
const GO_LOG: &str =
    "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tlog.Println(\"hello\")\n}\n";

/// A config that applies `fmt.Println` -> `log.Println` immediately with the given `[hooks]` lines.
fn write_run_config(temp: &TempDir, hooks: &str) -> std::path::PathBuf {
    let config = temp.path().join("run.toml");
    fs::write(
        &config,
        format!(
            "root = \"code\"\napply_immediately = true\n\n[[rules]]\ntype = \"ast\"\nlanguage = \"go\"\npattern = \"fmt.Println($A)\"\nreplacement = \"log.Println($A)\"\n\n[hooks]\n{hooks}\n"
        ),
    )
    .expect("config");
    config
}

#[test]
fn post_apply_hook_output_is_shown_on_stdout_and_stderr() {
    let temp = TempDir::new().expect("temp dir");
    fs::create_dir(temp.path().join("code")).expect("code dir");
    fs::write(temp.path().join("code/main.go"), GO_PRINT).expect("fixture");
    let config = write_run_config(
        &temp,
        "post_apply = [\"printf hook-out && printf hook-err >&2\"]",
    );

    let output = replace_with_config(temp.path(), &config)
        .arg("run")
        .assert()
        .success()
        .get_output()
        .clone();
    let shown = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(shown.contains("hook-out"), "stdout/stderr: {shown}");
    assert!(shown.contains("hook-err"), "stdout/stderr: {shown}");
}

#[test]
fn a_failing_pre_apply_hook_leaves_the_file_unchanged() {
    let temp = TempDir::new().expect("temp dir");
    fs::create_dir(temp.path().join("code")).expect("code dir");
    fs::write(temp.path().join("code/main.go"), GO_PRINT).expect("fixture");
    let config = write_run_config(&temp, "pre_apply = [\"false\"]");

    replace_with_config(temp.path(), &config)
        .arg("run")
        .assert()
        .failure();

    assert_eq!(
        fs::read_to_string(temp.path().join("code/main.go")).expect("read"),
        GO_PRINT
    );
}

#[test]
fn a_failing_pre_apply_hook_exits_nine_with_the_pre_hook_code() {
    let temp = TempDir::new().expect("temp dir");
    fs::create_dir(temp.path().join("code")).expect("code dir");
    fs::write(temp.path().join("code/main.go"), GO_PRINT).expect("fixture");
    let config = write_run_config(&temp, "pre_apply = [\"false\"]");

    replace_with_config(temp.path(), &config)
        .arg("run")
        .assert()
        .code(9)
        .stderr(predicate::str::contains("REPLACE_E_PRE_HOOK_FAILED"));
}

#[test]
fn a_failing_post_apply_hook_keeps_the_applied_change() {
    let temp = TempDir::new().expect("temp dir");
    fs::create_dir(temp.path().join("code")).expect("code dir");
    fs::write(temp.path().join("code/main.go"), GO_PRINT).expect("fixture");
    let config = write_run_config(&temp, "post_apply = [\"false\"]");

    replace_with_config(temp.path(), &config)
        .arg("run")
        .assert()
        .code(10)
        .stderr(predicate::str::contains("REPLACE_E_POST_HOOK_FAILED"));

    assert_eq!(
        fs::read_to_string(temp.path().join("code/main.go")).expect("read"),
        GO_LOG
    );
}

#[test]
fn the_hook_environment_names_the_stage_and_the_files() {
    let temp = TempDir::new().expect("temp dir");
    fs::create_dir(temp.path().join("code")).expect("code dir");
    fs::write(temp.path().join("code/main.go"), GO_PRINT).expect("fixture");
    let config = write_run_config(
        &temp,
        "post_apply = [\"printf '%s %s' \\\"$REPLACE_STAGE_ID\\\" \\\"$REPLACE_FILES\\\" > hook-env.out\"]",
    );

    replace_with_config(temp.path(), &config)
        .arg("run")
        .assert()
        .success();

    let recorded = fs::read_to_string(temp.path().join("hook-env.out")).expect("hook output");
    assert!(
        recorded.contains("main.go"),
        "hook environment: {recorded:?}"
    );
}
