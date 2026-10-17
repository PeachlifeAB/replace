//! Regex, AST, diagnostic and config-driven rewrites over the dogfood fixtures.
use assert_fs::TempDir;

use crate::common::{replace_cmd, replace_with_config, stage_id};

pub fn run_regex_apply_undo(temp: &TempDir) {
    let output = replace_cmd(temp)
        .args([
            "regex",
            "demo-[0-9]+",
            "demo-id",
            "workspace/regex-only.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("Staged:"))
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).expect("utf8 stdout");
    let regex_id = stage_id(&output).to_owned();
    replace_cmd(temp)
        .args(["apply", &regex_id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Applied:"));
    replace_cmd(temp)
        .args(["--yes", "undo", &regex_id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Undid:"));
}

pub fn run_ast_discard(temp: &TempDir) {
    let output = replace_cmd(temp)
        .args([
            "ast",
            "var $NAME = $VALUE;",
            "let $NAME = $VALUE;",
            "workspace/code.tsx",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("Staged:"))
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).expect("utf8 stdout");
    let ast_id = stage_id(&output).to_owned();
    replace_cmd(temp)
        .args(["--yes", "discard", &ast_id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Discarded:"));
}

pub fn run_from_error_diagnostic(temp: &TempDir) {
    let diagnostic = format!(
        "{}/workspace/code.tsx:1:5: error TS2322: Type 'string' is not assignable to type 'number'",
        temp.path().display()
    );
    let output = replace_cmd(temp)
        .args(["from-error", &diagnostic])
        .assert()
        .success()
        .stdout(predicates::str::contains("Confidence: low"))
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).expect("utf8 stdout");
    let from_error_id = stage_id(&output).to_owned();
    replace_cmd(temp)
        .args(["explain", &from_error_id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Mode: FromError"));
    replace_cmd(temp)
        .args(["--yes", "discard", &from_error_id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Discarded:"));
}

pub fn run_config_apply_log(temp: &TempDir) {
    replace_with_config(temp.path(), &temp.path().join("replace.toml"))
        .arg("run")
        .assert()
        .success()
        .stdout(predicates::str::contains("Run complete"));
    replace_cmd(temp)
        .arg("apply")
        .assert()
        .success()
        .stdout(predicates::str::contains("Applied:"));
    replace_cmd(temp).arg("log").assert().success();
}
