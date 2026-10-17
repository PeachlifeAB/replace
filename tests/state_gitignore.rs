//! `.replace/` keeps scratch state next to the project config; only the state is git-ignored.
use std::fs;
use std::process::Command as Git;

use assert_fs::TempDir;
use assert_fs::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_in;

fn staged_workspace() -> TempDir {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");
    temp.child(".replace/config.toml")
        .write_str("")
        .expect("config");
    replace_in(temp.path())
        .args(["literal", "foo", "bar", "--stage", "a.txt"])
        .assert()
        .success();
    temp
}

fn git(temp: &TempDir, args: &[&str]) -> String {
    let output = Git::new("git")
        .args(args)
        .current_dir(temp.path())
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("XDG_CONFIG_HOME", temp.path().join("no-xdg"))
        .output()
        .expect("git runs");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn creating_state_writes_a_gitignore_for_the_state_only() {
    let temp = staged_workspace();
    let ignore = fs::read_to_string(temp.path().join(".replace/.gitignore")).expect("gitignore");
    for state in ["/stage/", "/history/", "/lock"] {
        assert!(
            ignore.lines().any(|line| line == state),
            "{state} missing from {ignore:?}"
        );
    }
    assert!(
        !ignore.contains("config"),
        "config must stay trackable: {ignore:?}"
    );
}

#[test]
fn git_tracks_the_config_and_ignores_the_state() {
    let temp = staged_workspace();
    git(&temp, &["init", "-q"]);
    let status = git(&temp, &["status", "--porcelain", "--untracked-files=all"]);
    assert!(
        status.contains(".replace/config.toml"),
        "config not tracked:\n{status}"
    );
    assert!(
        !status.contains(".replace/stage"),
        "stage leaked into git:\n{status}"
    );
    assert!(
        !status.contains(".replace/history"),
        "history leaked into git:\n{status}"
    );
    assert!(
        !status.contains(".replace/lock"),
        "lock leaked into git:\n{status}"
    );
}

#[test]
fn an_existing_gitignore_in_replace_is_left_alone() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");
    temp.child(".replace/.gitignore")
        .write_str("custom\n")
        .expect("custom ignore");
    replace_in(temp.path())
        .args(["literal", "foo", "bar", "--stage", "a.txt"])
        .assert()
        .success();
    temp.child(".replace/.gitignore").assert("custom\n");
}
