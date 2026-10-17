//! Smoke commands, literal staging and recursive or file-name rewrites over the dogfood fixtures.
use std::fs;

use assert_fs::TempDir;

use crate::common::{replace_cmd, stage_id};

pub fn run_smoke_commands(temp: &TempDir) {
    replace_cmd(temp)
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("replace FIND REPLACE [PATHS...]"));
    replace_cmd(temp)
        .arg("status")
        .assert()
        .success()
        .stdout(predicates::str::contains("replace status"));
    replace_cmd(temp)
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicates::str::contains("replace diagnostics: OK"));
    replace_cmd(temp)
        .arg("languages")
        .assert()
        .success()
        .stdout(predicates::str::contains("Supported languages"));
    replace_cmd(temp)
        .arg("log")
        .assert()
        .success()
        .stdout(predicates::str::contains("No history entries"));
    replace_cmd(temp)
        .args(["--yes", "gc"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Garbage collected"));
}

pub fn run_literal_lifecycle(temp: &TempDir) {
    let output = replace_cmd(temp)
        .args([
            "literal",
            "DOGFOOD_TOKEN",
            "verified",
            "workspace/notes.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("Staged:"))
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).expect("utf8 stdout");
    let literal_id = stage_id(&output).to_owned();
    replace_cmd(temp)
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains(&literal_id));
    replace_cmd(temp)
        .arg("show")
        .assert()
        .success()
        .stdout(predicates::str::contains(&literal_id));
    replace_cmd(temp)
        .args(["show", &literal_id])
        .assert()
        .success()
        .stdout(predicates::str::contains("verified"));
    replace_cmd(temp)
        .arg("check")
        .assert()
        .success()
        .stdout(predicates::str::contains("Check complete"));
    replace_cmd(temp)
        .args(["explain", &literal_id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Mode: Literal"));
    replace_cmd(temp)
        .args(["--yes", "discard", &literal_id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Discarded:"));
}

pub fn run_dot_literal_shorthand(temp: &TempDir) {
    let output = replace_cmd(temp)
        .args(["foo.", "bar.", "workspace/dot-literal.txt"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Staged:"))
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).expect("utf8 stdout");
    let dot_literal_id = stage_id(&output).to_owned();
    replace_cmd(temp)
        .args(["apply", &dot_literal_id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Applied:"));
    let dot_literal_content = fs::read_to_string(temp.path().join("workspace/dot-literal.txt"))
        .expect("read dot literal result");
    assert_eq!(
        dot_literal_content,
        "find bar. but keep foobar and foo without a dot\n"
    );
}

pub fn run_recursive_apply_all(temp: &TempDir) {
    replace_cmd(temp)
        .args([
            "all-files-token",
            "all-files-replaced",
            "workspace/recursive-only",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("Staged:"));
    replace_cmd(temp)
        .arg("apply")
        .assert()
        .success()
        .stdout(predicates::str::contains("Applied:"));
    let recursive_root = fs::read_to_string(temp.path().join("workspace/recursive-only/root.txt"))
        .expect("read recursive root result");
    let recursive_leaf = fs::read_to_string(
        temp.path()
            .join("workspace/recursive-only/nested/deeper/leaf.txt"),
    )
    .expect("read recursive leaf result");
    assert_eq!(recursive_root, "all-files-replaced at root\n");
    assert_eq!(recursive_leaf, "all-files-replaced in nested leaf\n");
}

pub fn run_replace_files(temp: &TempDir) {
    replace_cmd(temp)
        .args(["hello.", "hi."])
        .assert()
        .success()
        .stdout(predicates::str::contains("Staged:"));
    replace_cmd(temp)
        .arg("apply")
        .assert()
        .success()
        .stdout(predicates::str::contains("Applied:"));
    assert!(
        !temp
            .path()
            .join("workspace/include-paths/nested/hello.name.txt")
            .exists()
    );
    assert!(
        temp.path()
            .join("workspace/include-paths/nested/hi.name.txt")
            .exists()
    );
}
