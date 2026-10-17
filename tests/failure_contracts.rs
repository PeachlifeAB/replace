//! Stable exit codes and error codes for stage, apply and undo failure paths.
use std::fs;
use std::os::unix::fs::PermissionsExt;

use assert_fs::TempDir;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

const EXIT_IO: i32 = 1;
const EXIT_INVALID_ARGS: i32 = 2;
const EXIT_AST: i32 = 4;
const EXIT_DRIFT: i32 = 5;
const EXIT_ROLLBACK: i32 = 6;
const UNKNOWN_STAGE_ID: &str = "deadbeef1234";

fn stage_literal(temp: &TempDir, path: &str) -> String {
    let output = replace_cmd(temp)
        .args(["literal", "foo", "bar", "--stage", path])
        .output()
        .expect("stage runs");
    assert!(output.status.success(), "staging failed: {output:?}");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("Staged: "))
        .and_then(|rest| rest.split_whitespace().next())
        .expect("stage id in output")
        .to_owned()
}

fn workspace_with(name: &str, content: &str) -> TempDir {
    let temp = TempDir::new().expect("temp dir");
    temp.child(name).write_str(content).expect("fixture");
    temp
}

#[test]
fn apply_with_unknown_anchor_fails_without_touching_the_file() {
    let temp = workspace_with("a.txt", "foo\n");
    let stage = stage_literal(&temp, "a.txt");
    replace_cmd(&temp)
        .args(["apply", &stage, "--only-anchors", "a.txt#deadbeef"])
        .assert()
        .failure()
        .code(EXIT_DRIFT);
    temp.child("a.txt").assert("foo\n");
}

#[test]
fn applying_the_same_stage_twice_is_rejected_as_unknown() {
    let temp = workspace_with("a.txt", "foo\n");
    let stage = stage_literal(&temp, "a.txt");
    replace_cmd(&temp)
        .args(["apply", &stage])
        .assert()
        .success();
    replace_cmd(&temp)
        .args(["apply", &stage])
        .assert()
        .failure()
        .code(EXIT_INVALID_ARGS)
        .stderr(predicates::str::contains("REPLACE_E_UNKNOWN_STAGE"));
}

#[test]
fn discard_of_unknown_stage_fails_cleanly() {
    let temp = workspace_with("a.txt", "foo\n");
    replace_cmd(&temp)
        .args(["discard", UNKNOWN_STAGE_ID, "--yes"])
        .assert()
        .failure()
        .code(EXIT_INVALID_ARGS)
        .stderr(predicates::str::contains("REPLACE_E_UNKNOWN_STAGE"));
}

#[test]
fn undo_of_unknown_history_id_fails_cleanly() {
    let temp = workspace_with("a.txt", "foo\n");
    replace_cmd(&temp)
        .args(["undo", UNKNOWN_STAGE_ID, "--yes"])
        .assert()
        .failure()
        .code(EXIT_INVALID_ARGS)
        .stderr(predicates::str::contains("REPLACE_E_UNKNOWN_HISTORY"));
}

#[test]
fn apply_blocks_when_file_drifted_after_staging() {
    let temp = workspace_with("a.txt", "foo\n");
    let stage = stage_literal(&temp, "a.txt");
    temp.child("a.txt")
        .write_str("foo\nextra edit\n")
        .expect("drift");
    replace_cmd(&temp)
        .args(["apply", &stage])
        .assert()
        .failure()
        .code(EXIT_DRIFT);
    temp.child("a.txt").assert("foo\nextra edit\n");
}

#[test]
fn undo_blocks_when_file_drifted_after_apply() {
    let temp = workspace_with("a.txt", "foo\n");
    let stage = stage_literal(&temp, "a.txt");
    replace_cmd(&temp)
        .args(["apply", &stage])
        .assert()
        .success();
    temp.child("a.txt")
        .write_str("bar\ndrift\n")
        .expect("drift");
    replace_cmd(&temp)
        .args(["undo", &stage, "--yes"])
        .assert()
        .failure()
        .code(EXIT_DRIFT)
        .stderr(predicates::str::contains("REPLACE_E_ANCHOR_MISMATCH"));
    temp.child("a.txt").assert("bar\ndrift\n");
}

fn make_directory_readonly(temp: &TempDir, name: &str) -> std::path::PathBuf {
    let dir = temp.path().join(name);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).expect("chmod dir");
    dir
}

fn restore_directory(dir: &std::path::Path) {
    fs::set_permissions(dir, fs::Permissions::from_mode(0o755)).expect("chmod dir back");
}

#[test]
fn unwritable_directory_fails_apply_with_io_error_and_keeps_content() {
    let temp = workspace_with("locked/a.txt", "foo\n");
    let dir = make_directory_readonly(&temp, "locked");
    let result = replace_cmd(&temp)
        .args(["literal", "foo", "bar", "--apply", "locked/a.txt"])
        .assert()
        .failure();
    restore_directory(&dir);
    result
        .code(EXIT_IO)
        .stderr(predicates::str::contains("REPLACE_E_IO"));
    temp.child("locked/a.txt").assert("foo\n");
}

#[test]
fn partial_write_failure_rolls_back_files_already_written() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a_writable/a.txt")
        .write_str("foo\n")
        .expect("fixture");
    temp.child("b_locked/b.txt")
        .write_str("foo\n")
        .expect("fixture");
    let locked = make_directory_readonly(&temp, "b_locked");
    let result = replace_cmd(&temp)
        .args(["literal", "foo", "bar", "--apply", "."])
        .assert()
        .failure();
    restore_directory(&locked);
    result
        .code(EXIT_ROLLBACK)
        .stderr(predicates::str::contains("REPLACE_E_ROLLBACK"));
    temp.child("a_writable/a.txt").assert("foo\n");
}

#[test]
fn syntax_error_in_target_blocks_ast_apply() {
    let temp = workspace_with("broken.rs", "fn main( {\n");
    replace_cmd(&temp)
        .args(["ast", "main($A)", "main($A)", "--apply", "broken.rs"])
        .assert()
        .failure()
        .code(EXIT_AST)
        .stderr(predicates::str::contains("REPLACE_E_PARSE_BLOCKED"));
    temp.child("broken.rs").assert("fn main( {\n");
}

#[test]
fn stage_of_nonexistent_path_reports_a_missing_path() {
    let temp = workspace_with("a.txt", "foo\n");
    replace_cmd(&temp)
        .args(["literal", "foo", "bar", "--stage", "nope.txt"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("nope.txt").or(predicates::str::contains("not found")));
}
