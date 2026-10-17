use assert_fs::TempDir;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

fn workspace() -> TempDir {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");
    temp
}

#[test]
fn staging_prints_the_diff_by_default() {
    let temp = workspace();
    replace_cmd(&temp)
        .args(["literal", "foo", "bar", "--stage", "a.txt"])
        .assert()
        .success()
        .stdout(predicates::str::contains("-foo").and(predicates::str::contains("+bar")));
}

#[test]
fn diff_none_keeps_the_summary_and_hides_the_diff_body() {
    for rewrite in [vec!["literal", "foo", "bar"], vec!["regex", "fo+", "bar"]] {
        let temp = workspace();
        replace_cmd(&temp)
            .arg("--diff=none")
            .args(&rewrite)
            .args(["--stage", "a.txt"])
            .assert()
            .success()
            .stdout(predicates::str::contains("Staged:"))
            .stdout(predicates::str::contains("-foo").not())
            .stdout(predicates::str::contains("+bar").not());
    }
}

#[test]
fn diff_none_applies_to_ast_staging_too() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.rs")
        .write_str("fn main() { println!(\"x\"); }\n")
        .expect("fixture");
    replace_cmd(&temp)
        .args([
            "--diff",
            "none",
            "ast",
            "println!($A)",
            "eprintln!($A)",
            "a.rs",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("Staged:"))
        .stdout(predicates::str::contains("eprintln").not());
}

#[test]
fn unknown_diff_format_is_rejected() {
    let temp = workspace();
    replace_cmd(&temp)
        .args(["--diff", "sideways", "literal", "foo", "bar", "a.txt"])
        .assert()
        .failure()
        .code(2);
}
