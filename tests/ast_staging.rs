//! Rewrites stage by default and leave the file untouched until a staged apply archives them.
use std::fs;

use assert_fs::TempDir;
use predicates::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::{replace_cmd, stage_id};

const GO_PRINT: &str =
    "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"hi\")\n}\n";

fn stage_output(temp: &TempDir, args: &[&str]) -> String {
    let output = replace_cmd(temp)
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    String::from_utf8(output).expect("utf8 stdout")
}

#[test]
fn literal_stages_by_default_and_leaves_the_file_unchanged() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("sample.txt"), "hello world\n").expect("fixture");

    let output = stage_output(&temp, &["literal", "hello", "goodbye", "sample.txt"]);

    assert!(output.contains("Staged:"));
    assert_eq!(
        fs::read_to_string(temp.path().join("sample.txt")).expect("read"),
        "hello world\n"
    );
}

#[test]
fn regex_stages_by_default_and_leaves_the_file_unchanged() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("sample.txt"), "hello 123\n").expect("fixture");

    let output = stage_output(&temp, &["regex", "hello ([0-9]+)", "bye $1", "sample.txt"]);

    assert!(output.contains("Staged:"));
    assert_eq!(
        fs::read_to_string(temp.path().join("sample.txt")).expect("read"),
        "hello 123\n"
    );
}

#[test]
fn ast_stages_by_default_and_leaves_the_file_unchanged() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.go"), GO_PRINT).expect("fixture");

    let output = stage_output(
        &temp,
        &["ast", "fmt.Println($A)", "log.Println($A)", "main.go"],
    );

    assert!(output.contains("Staged:"));
    assert_eq!(
        fs::read_to_string(temp.path().join("main.go")).expect("read"),
        GO_PRINT
    );
}

#[test]
fn applying_a_staged_regex_archives_its_stage_to_history() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("sample.txt"), "hello 123\n").expect("fixture");
    let output = stage_output(&temp, &["regex", "hello ([0-9]+)", "bye $1", "sample.txt"]);
    let id = stage_id(&output).to_owned();

    replace_cmd(&temp).args(["apply", &id]).assert().success();

    let history = temp.path().join(".replace/history");
    assert!(history.join(format!("{id}.patch")).is_file());
    assert!(history.join(format!("{id}.json")).is_file());
    assert_eq!(
        fs::read_to_string(temp.path().join("sample.txt")).expect("read"),
        "bye 123\n"
    );
}

#[test]
fn applying_a_staged_ast_rewrite_archives_its_stage_to_history() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.go"), GO_PRINT).expect("fixture");
    let output = stage_output(
        &temp,
        &["ast", "fmt.Println($A)", "log.Println($A)", "main.go"],
    );
    let id = stage_id(&output).to_owned();

    replace_cmd(&temp).args(["apply", &id]).assert().success();

    assert!(
        temp.path()
            .join(format!(".replace/history/{id}.patch"))
            .is_file()
    );
}

#[test]
fn explain_names_the_language_of_a_staged_typescript_rewrite() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("calls.ts"), "const value = foo(name)\n").expect("fixture");
    let output = stage_output(&temp, &["ast", "foo($A)", "bar($A)", "calls.ts"]);
    let id = stage_id(&output).to_owned();

    replace_cmd(&temp)
        .args(["explain", &id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Language: typescript"))
        .stdout(predicates::str::contains("Language: go").not());
}

#[test]
fn an_invalid_ast_pattern_is_rejected_as_invalid_arguments() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.go"), GO_PRINT).expect("fixture");

    replace_cmd(&temp)
        .args(["ast", "fmt.Println(", "log.Println($A)", "main.go"])
        .assert()
        .code(2)
        .stderr(predicates::str::contains("REPLACE_E_INVALID_ARGS"));
}

#[test]
fn a_stage_id_is_twelve_lowercase_hex_characters() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.go"), GO_PRINT).expect("fixture");

    let output = stage_output(
        &temp,
        &["ast", "fmt.Println($A)", "log.Println($A)", "main.go"],
    );
    let id = stage_id(&output);

    assert_eq!(id.len(), 12);
    assert!(
        id.chars()
            .all(|character| matches!(character, '0'..='9' | 'a'..='f'))
    );
}

#[test]
fn a_multi_file_preview_lists_every_changed_file() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("group_first.go"), GO_PRINT).expect("fixture");
    fs::write(temp.path().join("group_second.go"), GO_PRINT).expect("fixture");

    replace_cmd(&temp)
        .args(["ast", "fmt.Println($A)", "log.Println($A)", "."])
        .assert()
        .success()
        .stdout(predicates::str::contains("group_first.go"))
        .stdout(predicates::str::contains("group_second.go"));
}

#[test]
fn stats_with_no_diff_prints_the_summary_without_the_changed_lines() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("stat.go"), GO_PRINT).expect("fixture");

    replace_cmd(&temp)
        .args([
            "--stats",
            "--diff",
            "none",
            "ast",
            "fmt.Println($A)",
            "log.Println($A)",
            "stat.go",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("1 replacements"))
        .stdout(predicates::str::contains("fmt.Println").not());
}

#[test]
fn applying_a_staged_literal_archives_its_stage_to_history() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("sample.txt"), "hello world\n").expect("fixture");
    let output = stage_output(&temp, &["literal", "hello", "goodbye", "sample.txt"]);
    let id = stage_id(&output).to_owned();

    replace_cmd(&temp).args(["apply", &id]).assert().success();

    assert!(
        temp.path()
            .join(format!(".replace/history/{id}.patch"))
            .is_file()
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("sample.txt")).expect("read"),
        "goodbye world\n"
    );
}
