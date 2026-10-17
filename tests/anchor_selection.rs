//! Anchors select one specific match among identical-shape matches in one file.
use std::fs;

use assert_fs::TempDir;
use serde_json::Value;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

const THREE_PRINTS: &str = "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"one\")\n\tfmt.Println(\"two\")\n\tfmt.Println(\"three\")\n}\n";

/// Stage `fmt.Println($A)` -> `log.Println($A)` as JSON and return the stage id and its anchors.
fn stage_println_as_json(temp: &TempDir, file: &str) -> (String, Vec<String>) {
    let output = replace_cmd(temp)
        .args(["--json", "ast", "fmt.Println($A)", "log.Println($A)", file])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("stage json");
    let stage = &json["stages"][0];
    let id = stage["id"].as_str().expect("id").to_owned();
    let anchors = stage["anchors"]
        .as_array()
        .expect("anchors")
        .iter()
        .map(|anchor| anchor.as_str().expect("anchor").to_owned())
        .collect();
    (id, anchors)
}

#[test]
fn only_the_second_anchor_rewrites_only_the_second_match() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("main.go");
    fs::write(&file, THREE_PRINTS).expect("fixture");

    let (id, anchors) = stage_println_as_json(&temp, "main.go");
    assert_eq!(anchors.len(), 6, "an anchor per removed and per added line");
    let second_match_anchor = &anchors[1];

    replace_cmd(&temp)
        .args(["apply", &id, "--only-anchors", second_match_anchor])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&file).expect("read"),
        "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"one\")\n\tlog.Println(\"two\")\n\tfmt.Println(\"three\")\n}\n"
    );
}

#[test]
fn skipping_the_second_anchor_rewrites_every_other_match() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("main.go");
    fs::write(&file, THREE_PRINTS).expect("fixture");

    let (id, anchors) = stage_println_as_json(&temp, "main.go");
    let second_match_anchor = &anchors[1];

    replace_cmd(&temp)
        .args(["apply", &id, "--skip-anchors", second_match_anchor])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&file).expect("read"),
        "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tlog.Println(\"one\")\n\tfmt.Println(\"two\")\n\tlog.Println(\"three\")\n}\n"
    );
}

#[test]
fn expect_anchors_applies_when_the_anchor_still_exists() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("main.go");
    fs::write(&file, THREE_PRINTS).expect("fixture");
    let (id, anchors) = stage_println_as_json(&temp, "main.go");

    replace_cmd(&temp)
        .args(["apply", &id, "--expect-anchors", &anchors[0]])
        .assert()
        .success();

    assert!(
        fs::read_to_string(&file)
            .expect("read")
            .contains("log.Println(\"one\")")
    );
}

#[test]
fn expect_anchors_refuses_to_apply_after_the_file_drifted() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("main.go");
    fs::write(&file, THREE_PRINTS).expect("fixture");
    let (id, anchors) = stage_println_as_json(&temp, "main.go");
    fs::write(&file, format!("{THREE_PRINTS}// drift\n")).expect("drift");

    replace_cmd(&temp)
        .args(["apply", &id, "--expect-anchors", &anchors[0]])
        .assert()
        .code(5)
        .stderr(predicates::str::contains("REPLACE_E_ANCHOR_MISMATCH"));
}

#[test]
fn identical_lines_in_different_functions_get_distinct_anchors() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(
        temp.path().join("main.go"),
        "package main\n\nimport \"fmt\"\n\nfunc one() {\n\tfmt.Println(\"same\")\n}\n\nfunc two() {\n\tfmt.Println(\"same\")\n}\n",
    )
    .expect("fixture");

    let (_, anchors) = stage_println_as_json(&temp, "main.go");

    let mut unique = anchors.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(anchors.len(), 4, "a removed and an added anchor per match");
    assert_eq!(unique.len(), 4, "anchors must not collide: {anchors:?}");
}

#[test]
fn an_added_line_anchor_selects_the_same_match() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("main.go");
    fs::write(&file, THREE_PRINTS).expect("fixture");
    let (id, anchors) = stage_println_as_json(&temp, "main.go");

    replace_cmd(&temp)
        .args(["apply", &id, "--only-anchors", &anchors[4]])
        .assert()
        .success();

    assert!(
        fs::read_to_string(&file)
            .expect("read")
            .contains("fmt.Println(\"one\")\n\tlog.Println(\"two\")\n\tfmt.Println(\"three\")")
    );
}

#[test]
fn the_matches_left_out_stay_staged_and_apply_later() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("main.go");
    fs::write(&file, THREE_PRINTS).expect("fixture");
    let (id, anchors) = stage_println_as_json(&temp, "main.go");
    replace_cmd(&temp)
        .args(["apply", &id, "--only-anchors", &anchors[1]])
        .assert()
        .success();

    replace_cmd(&temp).arg("apply").assert().success();

    assert_eq!(
        fs::read_to_string(&file).expect("read"),
        THREE_PRINTS.replace("fmt.Println", "log.Println")
    );
}

#[test]
fn selecting_one_match_works_without_a_trailing_newline() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("main.go");
    let source = THREE_PRINTS.trim_end();
    fs::write(&file, source).expect("fixture");
    let (id, anchors) = stage_println_as_json(&temp, "main.go");

    replace_cmd(&temp)
        .args(["apply", &id, "--only-anchors", &anchors[1]])
        .assert()
        .success();

    let written = fs::read_to_string(&file).expect("read");
    assert!(written.contains("log.Println(\"two\")") && !written.ends_with('\n'));
}
