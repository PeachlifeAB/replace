//! Text rewrites stage renames next to content rewrites (`--skip-paths` opts out); both apply, in order, and can be explained.
use assert_fs::TempDir;
use assert_fs::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

fn workspace_with_foo_file() -> TempDir {
    let temp = TempDir::new().expect("temp dir");
    temp.child("foo.txt").write_str("foo\n").expect("fixture");
    temp
}

fn stage_content_and_rename(temp: &TempDir) -> Vec<String> {
    let output = replace_cmd(temp)
        .args(["--json", "literal", "foo", "bar", "--stage", "."])
        .output()
        .expect("stage runs");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    json["stages"]
        .as_array()
        .expect("stages")
        .iter()
        .map(|stage| stage["id"].as_str().expect("id").to_owned())
        .collect()
}

#[test]
fn applying_everything_rewrites_the_content_and_then_renames_the_file() {
    let temp = workspace_with_foo_file();
    assert_eq!(
        stage_content_and_rename(&temp).len(),
        2,
        "content and rename"
    );

    replace_cmd(&temp).arg("apply").assert().success();

    temp.child("foo.txt").assert(predicates::path::missing());
    temp.child("bar.txt").assert("bar\n");
}

#[test]
fn explaining_a_rename_stage_does_not_crash() {
    let temp = workspace_with_foo_file();
    for id in stage_content_and_rename(&temp) {
        replace_cmd(&temp).args(["explain", &id]).assert().success();
    }
}

#[test]
fn a_staged_rename_that_leaves_its_folder_is_refused_at_apply() {
    let temp = workspace_with_foo_file();
    temp.child("sub").create_dir_all().expect("sub dir");
    temp.child("sub/foo.txt").write_str("x\n").expect("fixture");
    let output = replace_cmd(&temp)
        .args(["--json", "literal", "foo", "bar", "--stage", "sub/foo.txt"])
        .output()
        .expect("stage runs");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    let rename = json["stages"]
        .as_array()
        .expect("stages")
        .iter()
        .find(|stage| stage["mode"] == "renamefile")
        .expect("rename stage")["id"]
        .as_str()
        .expect("id")
        .to_owned();
    let path = temp.path().join(format!(".replace/stage/{rename}.json"));
    let edited = std::fs::read_to_string(&path)
        .expect("stage json")
        .replace("sub/bar.txt", "escaped.txt");
    std::fs::write(&path, edited).expect("edit stage");

    replace_cmd(&temp)
        .args(["apply", &rename])
        .assert()
        .failure();

    temp.child("escaped.txt")
        .assert(predicates::path::missing());
    temp.child("sub/foo.txt").assert(predicates::path::exists());
}

#[test]
fn matching_names_are_renamed_without_any_flag() {
    let temp = workspace_with_foo_file();

    replace_cmd(&temp)
        .args(["foo", "bar", "--apply"])
        .assert()
        .success();

    temp.child("bar.txt").assert("bar\n");
    temp.child("foo.txt").assert(predicates::path::missing());
}

#[test]
fn skip_paths_rewrites_content_and_keeps_the_names() {
    let temp = workspace_with_foo_file();

    replace_cmd(&temp)
        .args(["foo", "bar", "--skip-paths", "--apply"])
        .assert()
        .success();

    temp.child("foo.txt").assert("bar\n");
}

#[test]
fn ast_rewrites_never_rename_paths() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("foo.js").write_str("foo(1)\n").expect("fixture");

    replace_cmd(&temp)
        .args(["ast", "foo($A)", "bar($A)", "--apply"])
        .assert()
        .success();

    temp.child("foo.js").assert("bar(1)\n");
}

fn stdout_of(temp: &TempDir, args: &[&str]) -> String {
    let output = replace_cmd(temp).args(args).output().expect("replace runs");
    assert!(output.status.success(), "{args:?}: {output:?}");
    String::from_utf8(output.stdout).expect("utf8")
}

#[test]
fn a_staged_rename_shows_the_old_and_the_new_name() {
    let temp = workspace_with_foo_file();

    let shown = stdout_of(&temp, &["foo", "bar"]);

    assert!(shown.contains("./foo.txt -> ./bar.txt (rename)"), "{shown}");
}

#[test]
fn the_run_reports_how_many_names_it_renames() {
    let temp = workspace_with_foo_file();

    let shown = stdout_of(&temp, &["foo", "bar"]);

    assert!(
        shown.contains("Renames: 1 (--skip-paths keeps names)"),
        "{shown}"
    );
}

#[test]
fn no_rename_line_when_no_name_matches() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");

    let shown = stdout_of(&temp, &["foo", "bar"]);

    assert!(!shown.contains("Renames:"), "{shown}");
}

#[test]
fn stats_count_content_replacements_and_renames_apart() {
    let temp = workspace_with_foo_file();

    let shown = stdout_of(&temp, &["--stats", "foo", "bar"]);

    assert!(
        shown.contains("Stats: 1 files, 1 replacements, 1 renames"),
        "{shown}"
    );
}

#[test]
fn json_stats_carry_the_rename_count() {
    let temp = workspace_with_foo_file();

    let shown = stdout_of(&temp, &["--json", "foo", "bar"]);

    let run: serde_json::Value = serde_json::from_str(&shown).expect("json");
    assert_eq!(run["stats"]["renames"], 1);
    assert_eq!(run["stats"]["replacements"], 1);
}
