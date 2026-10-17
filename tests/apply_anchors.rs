//! `replace apply --only-anchors/--skip-anchors` writes exactly the chosen hunks and keeps the rest staged.
use assert_fs::TempDir;
use assert_fs::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::{replace_cmd, stage_foo_to_bar};

const FILLER: &str = "keep 1\nkeep 2\nkeep 3\nkeep 4\nkeep 5\nkeep 6\nkeep 7\nkeep 8\n";
const ORIGINAL_TWO_HUNKS: &str =
    "foo head\nkeep 1\nkeep 2\nkeep 3\nkeep 4\nkeep 5\nkeep 6\nkeep 7\nkeep 8\nfoo tail\n";
const ONLY_HEAD_APPLIED: &str =
    "bar head\nkeep 1\nkeep 2\nkeep 3\nkeep 4\nkeep 5\nkeep 6\nkeep 7\nkeep 8\nfoo tail\n";
const BOTH_APPLIED: &str =
    "bar head\nkeep 1\nkeep 2\nkeep 3\nkeep 4\nkeep 5\nkeep 6\nkeep 7\nkeep 8\nbar tail\n";

fn two_hunk_workspace() -> TempDir {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt")
        .write_str(ORIGINAL_TWO_HUNKS)
        .expect("fixture");
    temp
}

/// Anchors of the changed lines whose text mentions `word` (`-foo head` / `+bar head` ...).
fn anchors_of_hunk(temp: &TempDir, stage_id: &str, word: &str) -> Vec<String> {
    let output = replace_cmd(temp)
        .args(["--diff", "anchored", "show", stage_id])
        .output()
        .expect("show runs");
    let shown = String::from_utf8(output.stdout).expect("utf8");
    let lines: Vec<&str> = shown.lines().collect();
    lines
        .windows(2)
        .filter(|pair| pair[0].starts_with('¶') && pair[1].contains(word))
        .map(|pair| pair[0].trim_start_matches('¶').to_owned())
        .collect()
}

#[test]
fn only_anchors_writes_just_that_hunk_and_keeps_the_rest_staged() {
    let temp = two_hunk_workspace();
    let staged = stage_foo_to_bar(&temp, "a.txt");
    let head = anchors_of_hunk(&temp, &staged.id, "head").join(",");

    replace_cmd(&temp)
        .args(["apply", &staged.id, "--only-anchors", &head])
        .assert()
        .success();

    temp.child("a.txt").assert(ONLY_HEAD_APPLIED);
    let tail = anchors_of_hunk(&temp, &staged.id, "tail");
    assert_eq!(
        tail.len(),
        2,
        "the skipped hunk stays staged under {}",
        staged.id
    );
}

#[test]
fn the_remaining_hunk_can_be_applied_afterwards() {
    let temp = two_hunk_workspace();
    let staged = stage_foo_to_bar(&temp, "a.txt");
    let head = anchors_of_hunk(&temp, &staged.id, "head").join(",");
    replace_cmd(&temp)
        .args(["apply", &staged.id, "--only-anchors", &head])
        .assert()
        .success();

    replace_cmd(&temp)
        .args(["apply", &staged.id])
        .assert()
        .success();

    temp.child("a.txt").assert(BOTH_APPLIED);
}

#[test]
fn skip_anchors_leaves_that_hunk_unapplied() {
    let temp = two_hunk_workspace();
    let staged = stage_foo_to_bar(&temp, "a.txt");
    let tail = anchors_of_hunk(&temp, &staged.id, "tail").join(",");

    replace_cmd(&temp)
        .args(["apply", &staged.id, "--skip-anchors", &tail])
        .assert()
        .success();

    temp.child("a.txt").assert(ONLY_HEAD_APPLIED);
}

#[test]
fn a_partial_apply_can_be_undone() {
    let temp = two_hunk_workspace();
    let staged = stage_foo_to_bar(&temp, "a.txt");
    let head = anchors_of_hunk(&temp, &staged.id, "head").join(",");
    replace_cmd(&temp)
        .args(["apply", &staged.id, "--only-anchors", &head])
        .assert()
        .success();
    let history = replace_cmd(&temp).arg("log").output().expect("log runs");
    let applied_id = String::from_utf8(history.stdout)
        .expect("utf8")
        .split_whitespace()
        .find(|word| word.len() == 12 && word.chars().all(|c| c.is_ascii_hexdigit()))
        .expect("applied stage id in log")
        .to_owned();

    replace_cmd(&temp)
        .args(["--yes", "undo", &applied_id])
        .assert()
        .success();

    temp.child("a.txt").assert(ORIGINAL_TWO_HUNKS);
}

#[test]
fn anchors_across_stages_apply_all_or_nothing() {
    let temp = two_hunk_workspace();
    temp.child("b.txt")
        .write_str(&format!("foo b\n{FILLER}"))
        .expect("fixture");
    let first = stage_foo_to_bar(&temp, "a.txt");
    let second = stage_foo_to_bar(&temp, "b.txt");
    let anchors = [
        anchors_of_hunk(&temp, &first.id, "head"),
        anchors_of_hunk(&temp, &second.id, "b"),
    ]
    .concat()
    .join(",");
    temp.child("b.txt")
        .write_str("drifted\n")
        .expect("drift the second file");

    replace_cmd(&temp)
        .args(["apply", "--only-anchors", &anchors])
        .assert()
        .failure();

    temp.child("a.txt").assert(ORIGINAL_TWO_HUNKS);
}

#[test]
fn a_rule_that_would_rewrite_its_own_output_cannot_be_applied_in_parts() {
    let temp = TempDir::new().expect("temp dir");
    let original = format!("foo head\n{FILLER}foo tail\n");
    temp.child("a.txt").write_str(&original).expect("fixture");
    let output = replace_cmd(&temp)
        .args(["--json", "literal", "foo", "foofoo", "--stage", "a.txt"])
        .output()
        .expect("stage runs");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    let id = json["stages"][0]["id"].as_str().expect("id").to_owned();
    let head = anchors_of_hunk(&temp, &id, "head").join(",");

    replace_cmd(&temp)
        .args(["apply", &id, "--only-anchors", &head])
        .assert()
        .failure()
        .code(2);

    temp.child("a.txt").assert(original);
}
