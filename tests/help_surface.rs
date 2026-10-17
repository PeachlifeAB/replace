//! The root `--help` is all a shorthand user (`replace FIND REPLACE`) ever sees, so it lists every flag that works there.
use assert_fs::TempDir;
use predicates::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

#[test]
fn root_help_lists_the_flags_shorthand_rewrites_accept() {
    let temp = TempDir::new().expect("temp dir");

    replace_cmd(&temp)
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--skip-paths"))
        .stdout(predicate::str::contains("--apply"))
        .stdout(predicate::str::contains("--stage"))
        .stdout(predicate::str::contains("--backup"));
}

#[test]
fn flags_given_before_the_subcommand_reach_the_rewrite() {
    let temp = TempDir::new().expect("temp dir");
    std::fs::write(temp.path().join("foo.txt"), "foo\n").expect("fixture");

    replace_cmd(&temp)
        .args(["--apply", "--skip-paths", "literal", "foo", "bar"])
        .assert()
        .success();

    assert!(temp.path().join("foo.txt").exists());
    assert_eq!(
        std::fs::read_to_string(temp.path().join("foo.txt")).expect("read"),
        "bar\n"
    );
}
