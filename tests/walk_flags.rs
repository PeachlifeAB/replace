use assert_fs::TempDir;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

#[test]
fn hidden_files_are_skipped_unless_hidden_flag_is_given() {
    let temp = TempDir::new().expect("temp dir");
    temp.child(".hidden_file.txt")
        .write_str("foo\n")
        .expect("fixture");

    replace_cmd(&temp)
        .args(["literal", "foo", "baz", "."])
        .assert()
        .success()
        .stdout(predicates::str::contains("Staged:").not());

    replace_cmd(&temp)
        .args(["--hidden", "literal", "foo", "baz", "--apply", "."])
        .assert()
        .success();
    temp.child(".hidden_file.txt").assert("baz\n");
}

#[test]
fn hidden_flag_never_descends_into_dot_git() {
    let temp = TempDir::new().expect("temp dir");
    temp.child(".git/config")
        .write_str("foo\n")
        .expect("fixture");

    replace_cmd(&temp)
        .args(["--hidden", "literal", "foo", "baz", "--apply", "."])
        .assert()
        .success();
    temp.child(".git/config").assert("foo\n");
}

#[test]
fn gitignored_files_are_skipped_unless_no_ignore_flag_is_given() {
    let temp = TempDir::new().expect("temp dir");
    std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(temp.path())
        .status()
        .expect("git init");
    temp.child(".gitignore")
        .write_str("ignored_file.txt\n")
        .expect("gitignore");
    temp.child("ignored_file.txt")
        .write_str("foo\n")
        .expect("fixture");

    replace_cmd(&temp)
        .args(["literal", "foo", "baz", "."])
        .assert()
        .success()
        .stdout(predicates::str::contains("Staged:").not());

    replace_cmd(&temp)
        .args(["--no-ignore", "literal", "foo", "baz", "--apply", "."])
        .assert()
        .success();
    temp.child("ignored_file.txt").assert("baz\n");
}

#[test]
fn hidden_flag_never_rewrites_replace_state() {
    let temp = TempDir::new().expect("temp dir");
    temp.child(".replace/history/old.patch")
        .write_str("foo\n")
        .expect("fixture");
    temp.child("a.txt").write_str("foo\n").expect("fixture");

    replace_cmd(&temp)
        .args(["--hidden", "literal", "foo", "baz", "--apply", "."])
        .assert()
        .success();

    temp.child("a.txt").assert("baz\n");
    temp.child(".replace/history/old.patch").assert("foo\n");
}

#[cfg(unix)]
#[test]
fn symlinked_files_are_not_rewrite_targets() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");
    std::os::unix::fs::symlink(temp.path().join("a.txt"), temp.path().join("link.txt"))
        .expect("symlink");

    replace_cmd(&temp)
        .args(["literal", "foo", "baz", "--apply", "."])
        .assert()
        .success();

    temp.child("a.txt").assert("baz\n");
    assert!(
        std::fs::symlink_metadata(temp.path().join("link.txt"))
            .expect("link metadata")
            .file_type()
            .is_symlink(),
        "the symlink is still a symlink"
    );
}

#[test]
fn max_files_counts_only_files_that_would_be_scanned() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");
    for name in ["x1.log", "x2.log", "x3.log"] {
        temp.child(name).write_str("foo\n").expect("fixture");
    }
    temp.child("sub/deeper.log")
        .write_str("foo\n")
        .expect("fixture");

    replace_cmd(&temp)
        .args([
            "--max-files",
            "1",
            "--exclude",
            "**/*.log",
            "literal",
            "foo",
            "baz",
            "--apply",
            ".",
        ])
        .assert()
        .success();

    temp.child("a.txt").assert("baz\n");
}

#[test]
fn shorthand_replacement_honors_walk_flags() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("kept.txt").write_str("foo\n").expect("fixture");
    temp.child("skipped.txt")
        .write_str("foo\n")
        .expect("fixture");

    replace_cmd(&temp)
        .args(["--exclude", "skipped.txt", "foo", "bar", "."])
        .assert()
        .success()
        .stdout(predicates::str::contains("kept.txt"))
        .stdout(predicates::str::contains("skipped.txt").not());
}
