//! Plain-language hints tell a person what the run left out; programs get warnings instead.
use assert_fs::TempDir;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

#[test]
fn matching_file_names_are_mentioned_when_paths_are_skipped() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("foo.txt").write_str("foo\n").expect("fixture");

    replace_cmd(&temp)
        .args(["literal", "foo", "bar", "--skip-paths", "foo.txt"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "1 file or folder name also matches",
        ))
        .stderr(predicate::str::contains("drop --skip-paths"));
}

#[test]
fn no_rename_hint_when_paths_are_renamed() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("foo.txt").write_str("foo\n").expect("fixture");

    replace_cmd(&temp)
        .args(["literal", "foo", "bar", "foo.txt"])
        .assert()
        .success()
        .stderr(predicate::str::contains("also match").not());
}

#[test]
fn zero_matches_names_the_flag_that_reaches_a_skipped_hidden_match() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt")
        .write_str("nothing here\n")
        .expect("fixture");
    temp.child(".env").write_str("foo\n").expect("fixture");

    replace_cmd(&temp)
        .args(["literal", "foo", "bar", "."])
        .assert()
        .success()
        .stderr(predicate::str::contains("add --hidden to include them"));
}

#[test]
fn zero_matches_names_the_flag_that_reaches_a_skipped_ignored_match() {
    let temp = TempDir::new().expect("temp dir");
    temp.child(".ignore")
        .write_str("build/\n")
        .expect("fixture");
    temp.child("build/out.txt")
        .write_str("foo\n")
        .expect("fixture");

    replace_cmd(&temp)
        .args(["literal", "foo", "bar", "."])
        .assert()
        .success()
        .stderr(predicate::str::contains("add --no-ignore to include them"));
}

#[test]
fn no_scope_hint_when_nothing_skipped_matches() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt")
        .write_str("nothing here\n")
        .expect("fixture");
    temp.child(".env").write_str("nothing\n").expect("fixture");

    replace_cmd(&temp)
        .args(["literal", "foo", "bar", "."])
        .assert()
        .success()
        .stderr(predicate::str::contains("Hint").not());
}

#[test]
fn no_scope_hint_when_hidden_and_ignored_files_were_included() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt")
        .write_str("nothing here\n")
        .expect("fixture");

    replace_cmd(&temp)
        .args(["--hidden", "--no-ignore", "literal", "foo", "bar", "."])
        .assert()
        .success()
        .stderr(predicate::str::contains("Hint").not());
}

#[test]
fn json_output_carries_no_prose_hints() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("foo.txt").write_str("foo\n").expect("fixture");

    replace_cmd(&temp)
        .args(["--json", "literal", "foo", "bar", "foo.txt"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Hint").not());
}
