//! `-i` / `--ignore-case` matches regardless of letter case, and a staged change keeps that choice.
use assert_fs::TempDir;
use assert_fs::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

const MIXED_CASE: &str = "Foo one\nFOO two\nfoo three\nfood four\n";

fn workspace() -> TempDir {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str(MIXED_CASE).expect("fixture");
    temp
}

#[test]
fn literal_matches_every_case_with_ignore_case() {
    let temp = workspace();

    replace_cmd(&temp)
        .args(["-i", "literal", "foo", "bar", "--apply", "a.txt"])
        .assert()
        .success();

    temp.child("a.txt")
        .assert("bar one\nbar two\nbar three\nbard four\n");
}

#[test]
fn without_the_flag_case_still_matters() {
    let temp = workspace();

    replace_cmd(&temp)
        .args(["literal", "foo", "bar", "--apply", "a.txt"])
        .assert()
        .success();

    temp.child("a.txt")
        .assert("Foo one\nFOO two\nbar three\nbard four\n");
}

#[test]
fn regex_matches_every_case_with_ignore_case() {
    let temp = workspace();

    replace_cmd(&temp)
        .args([
            "--ignore-case",
            "regex",
            "(?m)^foo\\b",
            "bar",
            "--apply",
            "a.txt",
        ])
        .assert()
        .success();

    temp.child("a.txt")
        .assert("bar one\nbar two\nbar three\nfood four\n");
}

#[test]
fn shorthand_accepts_ignore_case() {
    let temp = workspace();

    replace_cmd(&temp)
        .args(["-i", "foo", "bar", "a.txt", "--apply"])
        .assert()
        .success();

    temp.child("a.txt")
        .assert("bar one\nbar two\nbar three\nbard four\n");
}

#[test]
fn a_staged_ignore_case_change_applies_the_same_way_later() {
    let temp = workspace();
    replace_cmd(&temp)
        .args(["-i", "literal", "foo", "bar", "--stage", "a.txt"])
        .assert()
        .success();

    replace_cmd(&temp).arg("apply").assert().success();

    temp.child("a.txt")
        .assert("bar one\nbar two\nbar three\nbard four\n");
}
