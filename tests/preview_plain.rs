//! The default preview is a plain diff; anchors are opt-in for people who select hunks.
use assert_fs::TempDir;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::{replace_cmd, stage_foo_to_bar};

fn workspace() -> TempDir {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo one\n").expect("fixture");
    temp
}

#[test]
fn the_default_preview_is_a_plain_diff() {
    let temp = workspace();

    replace_cmd(&temp)
        .args(["literal", "foo", "bar", "a.txt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-foo one"))
        .stdout(predicate::str::contains("+bar one"))
        .stdout(predicate::str::contains("¶").not());
}

#[test]
fn the_anchored_diff_mode_shows_an_anchor_before_each_changed_line() {
    let temp = workspace();

    replace_cmd(&temp)
        .args(["--diff", "anchored", "literal", "foo", "bar", "a.txt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("¶a.txt#"));
}

#[test]
fn show_is_plain_by_default_and_anchored_on_request() {
    let temp = workspace();
    let staged = stage_foo_to_bar(&temp, "a.txt");

    replace_cmd(&temp)
        .args(["show", &staged.id])
        .assert()
        .success()
        .stdout(predicate::str::contains("-foo one"))
        .stdout(predicate::str::contains("¶").not());
    replace_cmd(&temp)
        .args(["--diff", "anchored", "show", &staged.id])
        .assert()
        .success()
        .stdout(predicate::str::contains("¶a.txt#"));
}

#[test]
fn json_output_always_carries_the_anchors() {
    let temp = workspace();
    let staged = stage_foo_to_bar(&temp, "a.txt");

    assert_eq!(staged.anchors.len(), 2);
}
