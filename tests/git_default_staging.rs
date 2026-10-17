//! Inside a git repository rewrites stage by default; `--stage` / `--apply` override.
use assert_fs::TempDir;
use std::fs;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

#[test]
fn literal_defaults_to_staging_inside_git_repo() {
    let temp = TempDir::new().expect("temp dir");
    fs::create_dir(temp.path().join(".git")).expect("create git marker");
    let file = temp.path().join("sample.txt");
    fs::write(&file, "foo.\n").expect("write fixture");

    let output = replace_cmd(&temp)
        .args(["literal", "foo.", "bar.", "sample.txt"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).expect("utf8 stdout");

    assert!(output.contains("Staged:"));
    assert!(!output.contains("Applied:"));
    assert_eq!(fs::read_to_string(file).expect("read result"), "foo.\n");
}

#[test]
fn literal_stage_overrides_git_repo_auto_apply() {
    let temp = TempDir::new().expect("temp dir");
    fs::create_dir(temp.path().join(".git")).expect("create git marker");
    let file = temp.path().join("sample.txt");
    fs::write(&file, "foo.\n").expect("write fixture");

    replace_cmd(&temp)
        .args(["literal", "foo.", "bar.", "--stage", "sample.txt"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Staged:"));

    assert_eq!(fs::read_to_string(file).expect("read result"), "foo.\n");
}
