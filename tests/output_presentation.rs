//! Preview output shows every changed line when piped; folding and truncation are planned, not shipped.
use std::fmt::Write as _;
use std::fs;

use assert_fs::TempDir;
use predicates::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

fn many_prints(count: usize) -> String {
    let mut source = String::from("package main\n\nimport \"fmt\"\n\nfunc main() {\n");
    for number in 1..=count {
        writeln!(source, "\tfmt.Println(\"{number}\")").expect("write to string");
    }
    source.push_str("}\n");
    source
}

#[test]
fn piped_preview_shows_the_last_changed_line_in_full() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.go"), many_prints(30)).expect("fixture");

    replace_cmd(&temp)
        .args(["ast", "fmt.Println($A)", "log.Println($A)", "main.go"])
        .assert()
        .success()
        .stdout(predicate::str::contains("fmt.Println(\"30\")"))
        .stdout(predicate::str::contains("folded").not());
}

#[test]
fn a_long_line_is_truncated_in_the_preview_with_a_marker() {
    let temp = TempDir::new().expect("temp dir");
    let long_argument = "x".repeat(220);
    fs::write(
        temp.path().join("main.go"),
        format!("package main\n\nimport \"fmt\"\n\nfunc main() {{\n\tfmt.Println(\"{long_argument}\")\n}}\n"),
    )
    .expect("fixture");

    replace_cmd(&temp)
        .args(["ast", "fmt.Println($A)", "log.Println($A)", "main.go"])
        .assert()
        .success()
        .stdout(predicate::str::contains("..."));
}

#[test]
fn a_large_preview_folds_by_default_and_names_expand() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.go"), many_prints(50)).expect("fixture");

    replace_cmd(&temp)
        .args(["ast", "fmt.Println($A)", "log.Println($A)", "main.go"])
        .assert()
        .success()
        .stdout(predicate::str::contains("folded"))
        .stdout(predicate::str::contains("--expand"));
}

#[test]
fn expand_shows_the_full_preview() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.go"), many_prints(20)).expect("fixture");

    replace_cmd(&temp)
        .args([
            "ast",
            "--expand",
            "fmt.Println($A)",
            "log.Println($A)",
            "main.go",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("fmt.Println(\"20\")"));
}

#[test]
fn expand_turns_off_folding_and_truncation() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.go"), many_prints(50)).expect("fixture");

    replace_cmd(&temp)
        .args(["--expand", "foo", "bar", "main.go"])
        .assert()
        .success();
    replace_cmd(&temp)
        .args([
            "--expand",
            "ast",
            "fmt.Println($A)",
            "log.Println($A)",
            "main.go",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("fmt.Println(\"50\")"))
        .stdout(predicate::str::contains("folded").not());
}

#[test]
fn show_prints_a_folded_preview_in_full() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.go"), many_prints(50)).expect("fixture");
    let staged = replace_cmd(&temp)
        .args([
            "--json",
            "ast",
            "fmt.Println($A)",
            "log.Println($A)",
            "main.go",
        ])
        .output()
        .expect("stage runs");
    let json: serde_json::Value = serde_json::from_slice(&staged.stdout).expect("json");
    let id = json["stages"][0]["id"].as_str().expect("id");

    replace_cmd(&temp)
        .args(["show", id])
        .assert()
        .success()
        .stdout(predicate::str::contains("fmt.Println(\"50\")"))
        .stdout(predicate::str::contains("folded").not());
}

#[test]
fn json_output_is_never_folded() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.go"), many_prints(50)).expect("fixture");

    replace_cmd(&temp)
        .args([
            "--json",
            "ast",
            "fmt.Println($A)",
            "log.Println($A)",
            "main.go",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("folded").not());
}
