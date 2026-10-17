//! Warnings are labelled, machine-readable and non-fatal, unless the caller asks for `--strict`.
use std::fs;

use assert_fs::TempDir;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

fn folder_with_text_and_binary() -> TempDir {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");
    fs::write(temp.path().join("b.bin"), b"\x00foo\x01").expect("binary fixture");
    temp
}

#[test]
fn json_output_lists_every_warning_with_its_code_and_path() {
    let temp = folder_with_text_and_binary();

    let output = replace_cmd(&temp)
        .args(["--json", "literal", "foo", "bar", "--apply", "."])
        .output()
        .expect("runs");

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    let warnings = json["warnings"].as_array().expect("warnings array");
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0]["code"], "REPLACE_W_SKIPPED_BINARY");
    assert!(
        warnings[0]["path"]
            .as_str()
            .expect("path")
            .ends_with("b.bin"),
        "warning names the skipped file"
    );
}

#[test]
fn json_output_labels_zero_matches() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");

    let output = replace_cmd(&temp)
        .args(["--json", "literal", "nomatch", "bar", "a.txt"])
        .output()
        .expect("runs");

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(json["warnings"][0]["code"], "REPLACE_W_ZERO_MATCHES");
}

#[test]
fn strict_turns_a_skip_into_exit_one_after_the_work_is_done() {
    let temp = folder_with_text_and_binary();

    replace_cmd(&temp)
        .args(["--strict", "literal", "foo", "bar", "--apply", "."])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("REPLACE_E_STRICT"))
        .stderr(predicate::str::contains("REPLACE_W_SKIPPED_BINARY"));

    temp.child("a.txt").assert("bar\n");
}

#[test]
fn strict_turns_zero_matches_into_exit_one() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");

    replace_cmd(&temp)
        .args(["--strict", "literal", "nomatch", "bar", "a.txt"])
        .assert()
        .code(1);
}

#[test]
fn strict_is_quiet_when_nothing_was_skipped() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");

    replace_cmd(&temp)
        .args(["--strict", "literal", "foo", "bar", "--apply", "a.txt"])
        .assert()
        .success();
}

#[test]
fn strict_also_applies_to_the_shorthand_form() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");

    replace_cmd(&temp)
        .args(["--strict", "nomatch", "bar", "a.txt"])
        .assert()
        .code(1);
}
