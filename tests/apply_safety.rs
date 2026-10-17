//! Apply refuses unsafe batches: conflicting stages, deleted targets, and a lock released between commands.
use std::fs;

use assert_fs::TempDir;
use predicates::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::{replace_cmd, stage_id};

const GO_PRINT: &str =
    "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"hello\")\n}\n";

fn stage_ast(temp: &TempDir, rewrite: &str) -> String {
    let output = replace_cmd(temp)
        .args(["ast", "fmt.Println($A)", rewrite, "main.go"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    stage_id(&String::from_utf8(output).expect("utf8")).to_owned()
}

#[test]
fn applying_two_stages_that_rewrite_the_same_text_differently_is_refused() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("main.go");
    fs::write(&file, GO_PRINT).expect("fixture");
    stage_ast(&temp, "log.Println($A)");
    stage_ast(&temp, "zap.Println($A)");

    replace_cmd(&temp)
        .arg("apply")
        .assert()
        .code(5)
        .stderr(predicate::str::contains("REPLACE_E_STAGE_CONFLICT"));

    assert_eq!(fs::read_to_string(&file).expect("read"), GO_PRINT);
}

#[test]
fn a_staged_file_deleted_before_apply_fails_with_file_deleted() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("main.go");
    fs::write(&file, GO_PRINT).expect("fixture");
    let id = stage_ast(&temp, "log.Println($A)");
    fs::remove_file(&file).expect("delete");

    replace_cmd(&temp)
        .args(["apply", &id])
        .assert()
        .code(5)
        .stderr(predicate::str::contains("REPLACE_E_FILE_DELETED"));
}

#[test]
fn a_finished_command_releases_the_lock_for_the_next_command() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.go"), GO_PRINT).expect("fixture");
    let id = stage_ast(&temp, "log.Println($A)");

    replace_cmd(&temp)
        .args(["--lock-timeout", "1s", "apply", &id])
        .assert()
        .success();
}
