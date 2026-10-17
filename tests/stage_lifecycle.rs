//! Staging, applying and discarding a rewrite.
use assert_fs::TempDir;
use std::fs;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::{replace_cmd, stage_id};

#[test]
fn literal_stages_and_applies_one_file() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("sample.txt");
    fs::write(&file, "omega beta omega\n").expect("write fixture");

    let output = replace_cmd(&temp)
        .args(["literal", "omega", "alpha", "sample.txt"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).expect("utf8 stdout");
    let id = stage_id(&output).to_owned();

    replace_cmd(&temp)
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains(&id));

    replace_cmd(&temp)
        .args(["show", &id])
        .assert()
        .success()
        .stdout(predicates::str::contains("alpha"));

    replace_cmd(&temp)
        .args(["apply", &id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Applied:"));

    let content = fs::read_to_string(file).expect("read result");
    assert_eq!(content, "alpha beta alpha\n");
}

#[test]
fn literal_discard_removes_stage_without_changing_file() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("sample.txt");
    fs::write(&file, "omega beta\n").expect("write fixture");

    let output = replace_cmd(&temp)
        .args(["literal", "omega", "alpha", "sample.txt"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).expect("utf8 stdout");
    let id = stage_id(&output).to_owned();

    replace_cmd(&temp)
        .args(["--yes", "discard", &id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Discarded:"));

    replace_cmd(&temp)
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("No stages yet"));

    let content = fs::read_to_string(file).expect("read result");
    assert_eq!(content, "omega beta\n");
}

#[test]
fn regex_stages_and_applies_one_file() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("sample.txt");
    fs::write(&file, "omega-123 beta-456\n").expect("write fixture");

    let output = replace_cmd(&temp)
        .args(["regex", r"\d+", "999", "sample.txt"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).expect("utf8 stdout");
    let id = stage_id(&output).to_owned();

    replace_cmd(&temp)
        .args(["apply", &id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Applied:"));

    let content = fs::read_to_string(file).expect("read result");
    assert_eq!(content, "omega-999 beta-999\n");
}

#[test]
fn ast_stages_and_applies_one_file() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("sample.tsx");
    fs::write(&file, "var answer = 42;\n").expect("write fixture");

    let output = replace_cmd(&temp)
        .args([
            "ast",
            "var $NAME = $VALUE;",
            "let $NAME = $VALUE;",
            "sample.tsx",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).expect("utf8 stdout");
    let id = stage_id(&output).to_owned();

    replace_cmd(&temp)
        .args(["apply", &id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Applied:"));

    let content = fs::read_to_string(file).expect("read result");
    assert_eq!(content, "let answer = 42;\n");
}

#[test]
fn ast_check_after_rejects_cascading_rewrite() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("sample.tsx");
    fs::write(&file, "var answer = 42;\n").expect("write fixture");

    replace_cmd(&temp)
        .args([
            "ast",
            "--check-after",
            "var $NAME = $VALUE;",
            "var $NAME = $VALUE;",
            "sample.tsx",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cascading rewrite"));
}
