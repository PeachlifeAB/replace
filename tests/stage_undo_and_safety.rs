//! Refusing unsafe destructive stage commands, undoing applied rewrites, and backup files.
use assert_fs::TempDir;
use std::fs;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::{replace_cmd, stage_id};

#[test]
fn destructive_commands_require_yes_when_non_interactive() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("sample.txt");
    fs::write(&file, "alpha beta\n").expect("write fixture");

    let output = replace_cmd(&temp)
        .args(["literal", "alpha", "omega", "sample.txt"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).expect("utf8 stdout");
    let id = stage_id(&output).to_owned();

    replace_cmd(&temp)
        .args(["discard", &id])
        .assert()
        .failure()
        .stderr(predicates::str::contains("requires --yes"));

    replace_cmd(&temp)
        .args(["apply", &id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Applied:"));

    replace_cmd(&temp)
        .args(["undo", &id])
        .assert()
        .failure()
        .stderr(predicates::str::contains("requires --yes"));

    replace_cmd(&temp)
        .arg("gc")
        .assert()
        .failure()
        .stderr(predicates::str::contains("requires --yes"));
}

#[test]
fn undo_restores_literal_rewrite() {
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

    assert_eq!(fs::read_to_string(&file).unwrap(), "alpha beta\n");

    // Undo the change
    replace_cmd(&temp)
        .args(["--yes", "undo", &id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Undid:"));

    assert_eq!(fs::read_to_string(&file).unwrap(), "omega beta\n");
}

#[test]
fn backup_flag_creates_orig_files() {
    let temp = TempDir::new().expect("temp dir");

    // Create a file
    let file = temp.path().join("backup_test.txt");
    let original_content = "hello world\n";
    fs::write(&file, original_content).expect("write fixture");

    // Apply a rewrite with --backup (using --apply to bypass stage policy)
    let output = replace_cmd(&temp)
        .args(["literal", "hello", "goodbye", "--apply", "--backup"])
        .output()
        .expect("replace apply");

    assert!(
        output.status.success(),
        "apply should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify .orig file was created
    let backup_file = temp.path().join("backup_test.txt.orig");
    assert!(backup_file.exists(), ".orig file should exist");

    // Verify .orig content matches original
    let backup_content = fs::read_to_string(&backup_file).expect("read .orig file");
    assert_eq!(
        backup_content, original_content,
        ".orig content should match original"
    );

    // Verify the original file was modified
    let modified_content = fs::read_to_string(&file).expect("read modified file");
    assert_eq!(
        modified_content, "goodbye world\n",
        "file should be modified"
    );
}
