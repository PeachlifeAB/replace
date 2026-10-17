//! Checking staged changes for drift, explaining stages and resolving anchors in show.
use assert_fs::TempDir;
use std::fs;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::{replace_cmd, stage_id};

#[test]
fn check_reports_clean_when_no_drift() {
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
    let _output = String::from_utf8(output).expect("utf8 stdout");

    // Check should report clean (no drift)
    replace_cmd(&temp)
        .arg("check")
        .assert()
        .success()
        .stdout(predicates::str::contains("1 clean"));
}

#[test]
fn check_reports_drift_when_file_modified() {
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
    let _output = String::from_utf8(output).expect("utf8 stdout");

    // Modify the file to cause drift
    fs::write(&file, "modified content\n").expect("modify file");

    // Check should report drift
    replace_cmd(&temp)
        .arg("check")
        .assert()
        .failure()
        .stderr(predicates::str::contains("drifted"));
}

#[test]
fn explain_shows_stage_metadata() {
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
        .args(["explain", &id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Mode: Literal"))
        .stdout(predicates::str::contains("Pattern: omega"))
        .stdout(predicates::str::contains("Rewrite: alpha"));
}

#[test]
fn anchor_resolution_in_show_command() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("sample.txt");
    fs::write(&file, "hello world\n").expect("write fixture");

    // Stage a rewrite
    let output = replace_cmd(&temp)
        .args([
            "--diff",
            "anchored",
            "literal",
            "hello",
            "goodbye",
            "--stage",
            "sample.txt",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).expect("utf8 stdout");

    // Extract an anchor from the output (format: ¶path#hash)
    let anchor = output
        .lines()
        .find_map(|line| line.strip_prefix("¶"))
        .expect("anchor in output");

    // Show the stage by anchor
    replace_cmd(&temp)
        .args(["show", anchor])
        .assert()
        .success()
        .stdout(predicates::str::contains("Stage:"))
        .stdout(predicates::str::contains(anchor));
}
