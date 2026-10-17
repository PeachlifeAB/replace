//! Regex capture groups, invalid regex diagnostics, and immediate literal apply.
use std::fs;

use assert_fs::TempDir;
use predicates::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

/// `file.txt` holding `source` after `replace ARGS --apply file.txt`.
fn applied_text(source: &str, args: &[&str]) -> String {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("file.txt"), source).expect("fixture");
    replace_cmd(&temp)
        .args(args)
        .args(["--apply", "file.txt"])
        .assert()
        .success();
    fs::read_to_string(temp.path().join("file.txt")).expect("read")
}

#[test]
fn regex_replacement_uses_capture_groups() {
    let written = applied_text("hello 123\n", &["regex", "hello ([0-9]+)", "goodbye $1"]);

    assert_eq!(written, "goodbye 123\n");
}

#[test]
fn an_invalid_regex_fails_with_the_regex_code_and_writes_nothing() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("file.txt"), "hello\n").expect("fixture");

    replace_cmd(&temp)
        .args(["regex", "[", "x", "file.txt"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("REPLACE_E_REGEX"));

    assert_eq!(
        fs::read_to_string(temp.path().join("file.txt")).expect("read"),
        "hello\n"
    );
}

#[test]
fn literal_with_apply_rewrites_the_file_immediately() {
    let written = applied_text("hello world\n", &["literal", "hello", "goodbye"]);

    assert_eq!(written, "goodbye world\n");
}
