//! The `--json` stage record: stdout is one JSON document and carries captures and enclosing names.
use std::fs;

use assert_fs::TempDir;
use serde_json::Value;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

const OTHER_FUNCTION_PRINT: &str =
    "package main\n\nimport \"fmt\"\n\nfunc other() {\n\tfmt.Println(\"hello\")\n}\n";

fn json_stage(temp: &TempDir) -> Value {
    let output = replace_cmd(temp)
        .args([
            "--json",
            "ast",
            "fmt.Println($A)",
            "log.Println($A)",
            "main.go",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).expect("stdout is one JSON document")
}

#[test]
fn json_stdout_is_a_single_document_and_carries_the_stage_metadata_keys() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.go"), OTHER_FUNCTION_PRINT).expect("fixture");

    let stage = &json_stage(&temp)["stages"][0];

    for key in [
        "anchors",
        "captures",
        "parent_function",
        "parent_class",
        "injected_language",
    ] {
        assert!(stage.get(key).is_some(), "missing key {key}");
    }
}

#[test]
fn json_captures_hold_the_matched_source_and_parent_function_names_the_enclosing_function() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.go"), OTHER_FUNCTION_PRINT).expect("fixture");

    let stage = &json_stage(&temp)["stages"][0];

    assert_eq!(stage["captures"]["$A"], "\"hello\"");
    assert_eq!(stage["parent_function"], "other");
}

#[test]
fn json_stage_record_names_the_pattern_and_rewrite() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.go"), OTHER_FUNCTION_PRINT).expect("fixture");

    let stage = &json_stage(&temp)["stages"][0];

    assert_eq!(stage["pattern"], "fmt.Println($A)");
    assert_eq!(stage["rewrite"], "log.Println($A)");
}
