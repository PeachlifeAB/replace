//! `replace apply` touches exactly the stages and anchors it was asked to.
use assert_fs::TempDir;
use assert_fs::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::{replace_cmd, stage_foo_to_bar as stage};

fn workspace() -> TempDir {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt")
        .write_str("foo one\nfoo two\n")
        .expect("fixture");
    temp.child("b.txt").write_str("foo\n").expect("fixture");
    temp
}

#[test]
fn apply_with_a_stage_id_leaves_other_stages_staged() {
    let temp = workspace();
    let first = stage(&temp, "a.txt");
    let second = stage(&temp, "b.txt");

    replace_cmd(&temp)
        .args(["apply", &first.id])
        .assert()
        .success();

    temp.child("a.txt").assert("bar one\nbar two\n");
    temp.child("b.txt").assert("foo\n");
    replace_cmd(&temp)
        .args(["show", &second.id])
        .assert()
        .success();
}

#[test]
fn apply_without_an_id_applies_every_stage() {
    let temp = workspace();
    stage(&temp, "a.txt");
    stage(&temp, "b.txt");

    replace_cmd(&temp).arg("apply").assert().success();

    temp.child("a.txt").assert("bar one\nbar two\n");
    temp.child("b.txt").assert("bar\n");
}

#[test]
fn only_anchors_applies_just_the_listed_hunk_of_the_chosen_stage() {
    let temp = workspace();
    let first = stage(&temp, "a.txt");
    stage(&temp, "b.txt");
    let one_hunk = first
        .anchors
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>()
        .join(",");

    replace_cmd(&temp)
        .args(["apply", &first.id, "--only-anchors", &one_hunk])
        .assert()
        .success();

    temp.child("b.txt").assert("foo\n");
}

#[test]
fn skip_anchors_with_a_stage_id_leaves_other_stages_alone() {
    let temp = workspace();
    let first = stage(&temp, "a.txt");
    stage(&temp, "b.txt");
    let skipped = first.anchors.last().expect("an anchor").clone();

    replace_cmd(&temp)
        .args(["apply", &first.id, "--skip-anchors", &skipped])
        .assert()
        .success();

    temp.child("b.txt").assert("foo\n");
}

#[test]
fn expect_anchors_must_exist_in_the_selected_stage() {
    let temp = workspace();
    let first = stage(&temp, "a.txt");
    let expected = first.anchors.join(",");

    replace_cmd(&temp)
        .args(["apply", &first.id, "--expect-anchors", &expected])
        .assert()
        .success();
    temp.child("a.txt").assert("bar one\nbar two\n");
}

#[test]
fn expect_anchors_that_no_stage_has_fail_without_writing() {
    let temp = workspace();
    let first = stage(&temp, "a.txt");

    replace_cmd(&temp)
        .args(["apply", &first.id, "--expect-anchors", "a.txt#dead"])
        .assert()
        .failure()
        .code(5);
    temp.child("a.txt").assert("foo one\nfoo two\n");
}

#[test]
fn conflicting_anchor_modes_are_rejected() {
    let temp = workspace();
    let first = stage(&temp, "a.txt");
    let anchor = first.anchors[0].clone();

    for (left, right) in [
        ("--expect-anchors", "--only-anchors"),
        ("--expect-anchors", "--skip-anchors"),
        ("--only-anchors", "--skip-anchors"),
    ] {
        replace_cmd(&temp)
            .args(["apply", &first.id, left, &anchor, right, &anchor])
            .assert()
            .failure()
            .code(2);
    }
}
