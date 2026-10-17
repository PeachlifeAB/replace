//! Every public subcommand answers `--help`; unknown input exits with the invalid-arguments code.
use assert_fs::TempDir;
use predicates::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

#[test]
fn every_public_subcommand_prints_usage_for_help() {
    let temp = TempDir::new().expect("temp dir");
    for subcommand in [
        "ast",
        "literal",
        "regex",
        "list",
        "show",
        "apply",
        "discard",
        "undo",
        "status",
        "log",
        "gc",
        "doctor",
        "languages",
        "check",
        "explain",
        "from-error",
    ] {
        replace_cmd(&temp)
            .args([subcommand, "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Usage:"));
    }
}

#[test]
fn an_unknown_command_exits_with_invalid_arguments() {
    let temp = TempDir::new().expect("temp dir");

    replace_cmd(&temp)
        .arg("wat")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("REPLACE_E_INVALID_ARGS"));
}

#[test]
#[ignore = "bug: doctor prints one line; bash contract lists replace, ast-grep, config and custom languages"]
fn doctor_reports_dependencies_and_config() {
    let temp = TempDir::new().expect("temp dir");

    replace_cmd(&temp)
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("ast-grep:"))
        .stdout(predicate::str::contains("config:"))
        .stdout(predicate::str::contains("custom languages:"));
}
