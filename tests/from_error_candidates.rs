//! A diagnostic becomes a low-confidence candidate that is staged for review and never applied directly.
use std::fs;

use assert_fs::TempDir;
use predicates::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::{replace_cmd, stage_id};

const GO_PRINT: &str =
    "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"hello\")\n}\n";

#[test]
fn from_error_reads_the_diagnostic_from_a_file_and_names_the_source() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.go"), GO_PRINT).expect("fixture");
    fs::write(
        temp.path().join("diagnostic.txt"),
        "main.go:1:1: tree-sitter: missing node\n",
    )
    .expect("diagnostic");

    replace_cmd(&temp)
        .args(["from-error", "diagnostic.txt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Source diagnostic:"));
}

#[test]
fn a_from_error_candidate_is_refused_by_apply() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.go"), GO_PRINT).expect("fixture");

    let output = replace_cmd(&temp)
        .args(["from-error", "main.go:4:2: undefined: fmt"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let id = stage_id(&String::from_utf8(output).expect("utf8")).to_owned();

    replace_cmd(&temp)
        .args(["apply", &id])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("REPLACE_E_INVALID_ARGS"));

    assert_eq!(
        fs::read_to_string(temp.path().join("main.go")).expect("read"),
        GO_PRINT
    );
}
