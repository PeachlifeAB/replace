//! The everyday loop a person or agent runs: `replace FIND REPLACE` with flags anywhere, apply
//! now or later, undo, retry, and rename files and folders, without the stage queue wedging.
use assert_fs::TempDir;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

fn json_stages(output: &std::process::Output) -> Vec<serde_json::Value> {
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json output");
    json["stages"].as_array().expect("stages").clone()
}

fn staged_ids(temp: &TempDir) -> Vec<String> {
    let output = replace_cmd(temp)
        .args(["--json", "list"])
        .output()
        .expect("list");
    let stages: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).expect("json");
    stages
        .iter()
        .map(|stage| stage["id"].as_str().expect("id").to_owned())
        .collect()
}

#[test]
fn shorthand_takes_flags_after_find_and_replace() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("Foo foo\n").expect("fixture");

    replace_cmd(&temp)
        .args(["foo", "bar", "a.txt", "-i", "--diff", "none", "--apply"])
        .assert()
        .success();

    temp.child("a.txt").assert("bar bar\n");
}

#[test]
fn shorthand_honors_double_dash_for_patterns_that_look_like_flags() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt")
        .write_str("x --old y\n")
        .expect("fixture");

    replace_cmd(&temp)
        .args(["--apply", "--", "--old", "--new", "a.txt"])
        .assert()
        .success();

    temp.child("a.txt").assert("x --new y\n");
}

#[test]
fn a_lone_unknown_word_explains_the_shorthand() {
    let temp = TempDir::new().expect("temp dir");

    replace_cmd(&temp)
        .arg("wat")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("REPLACE_E_INVALID_ARGS"))
        .stderr(predicate::str::contains("replace FIND REPLACE"));
}

#[test]
fn a_direct_apply_can_be_undone_once() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");

    let output = replace_cmd(&temp)
        .args(["--json", "foo", "bar", "--apply"])
        .output()
        .expect("apply runs");
    let id = json_stages(&output)[0]["id"]
        .as_str()
        .expect("id")
        .to_owned();
    temp.child("a.txt").assert("bar\n");

    replace_cmd(&temp)
        .args(["-y", "undo", &id])
        .assert()
        .success();
    temp.child("a.txt").assert("foo\n");

    replace_cmd(&temp)
        .args(["-y", "undo", &id])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("REPLACE_E_UNKNOWN_HISTORY"));
}

#[test]
fn yes_applies_even_with_json_output() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");

    let output = replace_cmd(&temp)
        .args(["--json", "-y", "foo", "bar"])
        .output()
        .expect("runs");

    assert!(output.status.success());
    assert_eq!(json_stages(&output)[0]["applied"], true);
    temp.child("a.txt").assert("bar\n");
    assert!(staged_ids(&temp).is_empty(), "nothing left staged");
}

#[test]
fn rerunning_the_same_rewrite_restages_in_place() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");

    for _ in 0..2 {
        replace_cmd(&temp).args(["foo", "bar"]).assert().success();
    }

    assert_eq!(staged_ids(&temp).len(), 1);
    replace_cmd(&temp).arg("apply").assert().success();
    temp.child("a.txt").assert("bar\n");
}

#[test]
fn rewriting_a_file_discards_its_outdated_stages() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");
    temp.child("b.txt").write_str("foo\n").expect("fixture");

    replace_cmd(&temp)
        .args(["foo", "bar", "a.txt"])
        .assert()
        .success();
    replace_cmd(&temp)
        .args(["foo", "baz", "a.txt", "--apply"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "discarded 1 outdated staged change",
        ));
    replace_cmd(&temp)
        .args(["foo", "qux", "b.txt"])
        .assert()
        .success();

    replace_cmd(&temp).arg("apply").assert().success();
    temp.child("a.txt").assert("baz\n");
    temp.child("b.txt").assert("qux\n");
}

#[test]
fn staging_prints_next_steps_once_per_run() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");
    temp.child("b.txt").write_str("foo\n").expect("fixture");

    let output = replace_cmd(&temp)
        .args(["foo", "bar"])
        .output()
        .expect("runs");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(stdout.matches("Next:").count(), 1, "{stdout}");
    assert!(stdout.contains("replace apply"));
}

#[test]
fn ignore_case_also_renames_file_names() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("Foo.txt").write_str("x\n").expect("fixture");

    replace_cmd(&temp)
        .args(["-i", "foo", "bar", "--apply"])
        .assert()
        .success();

    temp.child("bar.txt").assert("x\n");
}

#[test]
fn two_renames_to_one_name_fail_before_staging_anything() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("x\n").expect("fixture");
    temp.child("b.txt").write_str("x\n").expect("fixture");

    replace_cmd(&temp)
        .args(["regex", r"^[ab]\.txt$", "x.txt"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("would both be renamed to"));

    assert!(
        staged_ids(&temp).is_empty(),
        "the failed run left no stages"
    );
}

#[test]
fn replace_files_renames_folders_after_their_contents() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("src/foo/foo.rs")
        .write_str("foo\n")
        .expect("fixture");

    replace_cmd(&temp).args(["foo", "bar"]).assert().success();
    replace_cmd(&temp).arg("apply").assert().success();

    temp.child("src/bar/bar.rs").assert("bar\n");
    temp.child("src/foo").assert(predicate::path::missing());
}

#[test]
fn the_searched_folder_keeps_its_name() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("foo/a.txt").write_str("x\n").expect("fixture");

    replace_cmd(&temp)
        .args(["foo", "bar", "foo", "--apply"])
        .assert()
        .success();

    temp.child("foo/a.txt").assert("x\n");
}

#[test]
fn regex_anchors_match_each_line() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("x\nfoo\n").expect("fixture");

    replace_cmd(&temp)
        .args(["regex", "^foo$", "bar", "--apply"])
        .assert()
        .success();

    temp.child("a.txt").assert("x\nbar\n");
}
