//! Extra rewrite pairs in one run (`--also`) are not implemented in the Rust CLI yet (RPLC-30 unchecked).
use std::fs;

use assert_fs::TempDir;
use predicates::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

const CALLS: &str = "package main\n\nfunc main() {\n\tfoo(x)\n\tbaz(y)\n}\n";

#[test]
#[ignore = "bug: --also is not implemented (RPLC-30 unchecked); clap rejects the flag"]
fn also_appends_a_second_rewrite_pair() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("calls.go"), CALLS).expect("fixture");

    replace_cmd(&temp)
        .args([
            "ast", "foo($A)", "bar($A)", "--also", "baz($A)", "qux($A)", "calls.go",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("bar(x)"))
        .stdout(predicate::str::contains("qux(y)"));
}

#[test]
#[ignore = "bug: --also is not implemented (RPLC-30 unchecked); clap rejects the flag"]
fn also_rejects_a_duplicate_structural_pattern() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("calls.go"), CALLS).expect("fixture");

    replace_cmd(&temp)
        .args([
            "ast", "foo($A)", "bar($A)", "--also", "foo($A)", "zap($A)", "calls.go",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("duplicate pattern"));
}

#[test]
#[ignore = "bug: --also is not implemented (RPLC-30 unchecked); clap rejects the flag"]
fn also_rejects_overlapping_computed_edits() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("calls.go"), CALLS).expect("fixture");

    replace_cmd(&temp)
        .args([
            "ast", "foo($A)", "bar($A)", "--also", "baz($A)", "qux($A)", "--apply", "calls.go",
        ])
        .assert()
        .code(5)
        .stderr(predicate::str::contains("overlapping replacements"));
}
