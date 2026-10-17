//! The stage sidecar is the audit record of how a change was produced.
use std::fs;

use assert_fs::TempDir;
use assert_fs::prelude::*;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

fn stage_record(args: &[&str]) -> (TempDir, serde_json::Value) {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");
    replace_cmd(&temp).args(args).assert().success();
    let stage_dir = temp.path().join(".replace/stage");
    let sidecar = fs::read_dir(&stage_dir)
        .expect("stage dir")
        .map(|entry| entry.expect("entry").path())
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .expect("stage json");
    let record = serde_json::from_slice(&fs::read(sidecar).expect("read sidecar")).expect("json");
    (temp, record)
}

#[test]
fn stage_records_the_full_command_line() {
    let (_temp, record) = stage_record(&["literal", "foo", "bar", "--stage", "a.txt"]);
    let argv: Vec<&str> = record["command_argv"]
        .as_array()
        .expect("argv array")
        .iter()
        .map(|value| value.as_str().expect("argv entry"))
        .collect();
    assert!(
        argv.ends_with(&["literal", "foo", "bar", "--stage", "a.txt"]),
        "argv was {argv:?}"
    );
}

#[test]
fn stage_records_working_directory_mode_and_match_count() {
    let (temp, record) = stage_record(&["literal", "foo", "bar", "--stage", "a.txt"]);
    let expected_cwd = temp.path().canonicalize().expect("canonical cwd");
    assert_eq!(
        record["cwd"].as_str().expect("cwd"),
        expected_cwd.to_str().expect("utf8")
    );
    assert_eq!(record["mode"], "literal");
    assert_eq!(record["match_count"], 1);
}

#[test]
fn stage_timestamp_is_rfc3339() {
    let (_temp, record) = stage_record(&["literal", "foo", "bar", "--stage", "a.txt"]);
    let timestamp = record["timestamp"].as_str().expect("timestamp");
    OffsetDateTime::parse(timestamp, &Rfc3339)
        .unwrap_or_else(|error| panic!("{timestamp:?} is not RFC 3339: {error}"));
}

#[test]
fn explain_names_the_git_blame_author_of_a_committed_file() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");
    let git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .current_dir(temp.path())
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .args([
                "-c",
                "user.name=Blame Tester",
                "-c",
                "user.email=blame@example.com",
            ])
            .args(args)
            .status()
            .expect("git runs");
        assert!(status.success(), "git {args:?}");
    };
    git(&["init", "-q"]);
    git(&["add", "a.txt"]);
    git(&["commit", "-q", "-m", "add a"]);
    let staged = replace_cmd(&temp)
        .args(["--json", "literal", "foo", "bar", "--stage", "a.txt"])
        .output()
        .expect("stage runs");
    let json: serde_json::Value = serde_json::from_slice(&staged.stdout).expect("json");
    let id = json["stages"][0]["id"].as_str().expect("stage id");

    replace_cmd(&temp)
        .args(["explain", id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Last committed by: Blame Tester"));
}
