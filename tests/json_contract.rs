//! With `--json`, stdout is one JSON document that matches `schema/replace.output.json`;
//! notes and warnings stay on stderr.
use std::fs;

use assert_fs::TempDir;
use assert_fs::prelude::*;
use serde_json::Value;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

fn workspace() -> TempDir {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");
    temp
}

fn json_stdout(temp: &TempDir, args: &[&str]) -> Value {
    let output = replace_cmd(temp)
        .arg("--json")
        .args(args)
        .output()
        .expect("replace runs");
    assert!(output.status.success(), "{args:?}: {output:?}");
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{args:?} stdout is not one JSON document: {error}"))
}

fn assert_matches_schema(definition: &str, instance: &Value) {
    let schema: Value =
        serde_json::from_str(&fs::read_to_string("schema/replace.output.json").expect("schema"))
            .expect("schema json");
    let entry_point = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$defs": schema["$defs"],
        "$ref": format!("#/$defs/{definition}"),
    });
    let validator = jsonschema::draft202012::new(&entry_point).expect("compile schema");
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|error| error.to_string())
        .collect();
    assert!(errors.is_empty(), "{definition}: {errors:?}\n{instance}");
}

#[test]
fn log_lists_applied_changes_as_json() {
    let temp = workspace();
    let applied = json_stdout(&temp, &["-y", "foo", "bar"]);
    let id = applied["stages"][0]["id"].as_str().expect("id");

    let history = json_stdout(&temp, &["log"]);

    assert_eq!(history[0]["id"], id);
}

#[test]
fn log_without_history_is_an_empty_json_list() {
    let temp = workspace();

    assert_eq!(json_stdout(&temp, &["log"]), serde_json::json!([]));
}

#[test]
fn stdout_stays_one_json_document_when_a_warning_is_printed() {
    let temp = workspace();

    let run = json_stdout(&temp, &["nomatch", "x"]);

    assert_eq!(run["warnings"][0]["code"], "REPLACE_W_ZERO_MATCHES");
}

#[test]
fn stdout_stays_one_json_document_when_outdated_stages_are_discarded() {
    let temp = workspace();
    replace_cmd(&temp).args(["foo", "bar"]).assert().success();

    let run = json_stdout(&temp, &["-y", "foo", "baz"]);

    assert_eq!(run["stages"][0]["applied"], true);
}

#[test]
fn a_staged_run_matches_the_output_schema() {
    let temp = workspace();

    assert_matches_schema("run_output", &json_stdout(&temp, &["foo", "bar"]));
}

#[test]
fn an_applied_run_matches_the_output_schema() {
    let temp = workspace();

    assert_matches_schema("run_output", &json_stdout(&temp, &["-y", "foo", "bar"]));
}

#[test]
fn a_run_with_warnings_matches_the_output_schema() {
    let temp = workspace();

    assert_matches_schema("run_output", &json_stdout(&temp, &["nomatch", "x"]));
}

#[test]
fn list_matches_the_output_schema() {
    let temp = workspace();
    replace_cmd(&temp).args(["foo", "bar"]).assert().success();

    assert_matches_schema("stage_list", &json_stdout(&temp, &["list"]));
}

#[test]
fn log_matches_the_output_schema() {
    let temp = workspace();
    replace_cmd(&temp)
        .args(["-y", "foo", "bar"])
        .assert()
        .success();

    assert_matches_schema("stage_list", &json_stdout(&temp, &["log"]));
}

#[test]
fn from_error_matches_the_output_schema() {
    let temp = workspace();

    let run = json_stdout(&temp, &["from-error", "a.txt:1: broken"]);

    assert_matches_schema("run_output", &run);
    assert_eq!(run["stages"][0]["mode"], "fromerror");
}

fn staged_id(temp: &TempDir) -> String {
    let run = json_stdout(temp, &["foo", "bar"]);
    run["stages"][0]["id"].as_str().expect("id").to_owned()
}

#[test]
fn show_prints_one_stage_with_its_patch() {
    let temp = workspace();
    let id = staged_id(&temp);

    let stage = json_stdout(&temp, &["show", &id]);

    assert_matches_schema("stage_detail", &stage);
    assert!(stage["patch"].as_str().expect("patch").contains("-foo"));
}

#[test]
fn explain_prints_the_stage_record() {
    let temp = workspace();
    let id = staged_id(&temp);

    let stage = json_stdout(&temp, &["explain", &id]);

    assert_matches_schema("stage_entry", &stage);
    assert_eq!(stage["id"], id);
}

#[test]
fn check_reports_clean_stages() {
    let temp = workspace();
    staged_id(&temp);

    let report = json_stdout(&temp, &["check"]);

    assert_matches_schema("check_report", &report);
    assert_eq!(report["clean"], 1);
}

#[test]
fn check_reports_a_drifted_file_and_still_exits_5() {
    let temp = workspace();
    staged_id(&temp);
    temp.child("a.txt").write_str("changed\n").expect("drift");

    let output = replace_cmd(&temp)
        .args(["--json", "check"])
        .output()
        .expect("replace runs");

    assert_eq!(output.status.code(), Some(5));
    let report: Value = serde_json::from_slice(&output.stdout).expect("one JSON document");
    assert_matches_schema("check_report", &report);
    assert_eq!(report["drifted"][0]["file"], "./a.txt");
}

#[test]
fn a_run_that_renames_matches_the_output_schema() {
    let temp = workspace();
    temp.child("foo.txt").write_str("foo\n").expect("fixture");

    assert_matches_schema("run_output", &json_stdout(&temp, &["foo", "bar"]));
}
