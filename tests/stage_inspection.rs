//! Stage history, status and garbage collection.
use assert_fs::TempDir;
use std::fs;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::{make_stage_old, replace_cmd, stage_id};

#[test]
fn log_shows_history_after_apply() {
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

    // Apply the stage
    replace_cmd(&temp)
        .args(["apply", &id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Applied:"));

    // Log should show the history entry
    replace_cmd(&temp)
        .arg("log")
        .assert()
        .success()
        .stdout(predicates::str::contains(&id))
        .stdout(predicates::str::contains("Literal"));
}

#[test]
fn status_shows_stage_counts() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("sample.txt");
    fs::write(&file, "omega beta\n").expect("write fixture");

    // Initially no stages
    replace_cmd(&temp)
        .arg("status")
        .assert()
        .success()
        .stdout(predicates::str::contains("0 staged"));

    let output = replace_cmd(&temp)
        .args(["literal", "omega", "alpha", "sample.txt"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).expect("utf8 stdout");
    let id = stage_id(&output).to_owned();

    // Now 1 staged
    replace_cmd(&temp)
        .arg("status")
        .assert()
        .success()
        .stdout(predicates::str::contains("1 staged"));

    // Apply and check history
    replace_cmd(&temp)
        .args(["apply", &id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Applied:"));

    replace_cmd(&temp)
        .arg("status")
        .assert()
        .success()
        .stdout(predicates::str::contains("0 staged"))
        .stdout(predicates::str::contains("1 in history"));
}

#[test]
fn gc_respects_age_filter_for_stages_and_preserves_history() {
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
        .args(["--yes", "gc"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "0 staged entries older than 14 days removed (1 kept); history preserved",
        ));

    replace_cmd(&temp)
        .arg("status")
        .assert()
        .success()
        .stdout(predicates::str::contains("1 staged"));

    make_stage_old(&temp, &id);

    replace_cmd(&temp)
        .args(["--yes", "gc"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "1 staged entries older than 14 days removed (0 kept); history preserved",
        ));

    replace_cmd(&temp)
        .arg("status")
        .assert()
        .success()
        .stdout(predicates::str::contains("0 staged"));

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
        .args(["apply", &id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Applied:"));

    replace_cmd(&temp)
        .args(["--yes", "gc", "--age", "0"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "0 staged entries older than 0 days removed (0 kept); history preserved",
        ));

    replace_cmd(&temp)
        .arg("log")
        .assert()
        .success()
        .stdout(predicates::str::contains("sample.txt"));
}
