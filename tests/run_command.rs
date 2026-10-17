//! `replace run`: rules from config, immediate apply, hooks.
use assert_fs::TempDir;
use std::fs;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::{replace_cmd, replace_with_config};

#[test]
fn run_stages_rules_from_config() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("sample.txt");
    fs::write(&file, "omega beta\n").expect("write fixture");

    let config = r#"
[[rules]]
type = "literal"
find = "omega"
replace = "alpha"
"#;
    fs::write(temp.path().join("replace.toml"), config).expect("write config");

    replace_with_config(temp.path(), &temp.path().join("replace.toml"))
        .arg("run")
        .assert()
        .success()
        .stdout(predicates::str::contains("Staged:"))
        .stdout(predicates::str::contains("sample.txt"));

    let content = fs::read_to_string(&file).expect("read result");
    assert_eq!(content, "omega beta\n"); // Not applied, only staged
}

#[test]
fn run_with_apply_immediately_modifies_files() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("sample.txt");
    fs::write(&file, "omega beta\n").expect("write fixture");

    let config = r#"
apply_immediately = true

[[rules]]
type = "literal"
find = "omega"
replace = "alpha"
"#;
    fs::write(temp.path().join("replace.toml"), config).expect("write config");

    replace_with_config(temp.path(), &temp.path().join("replace.toml"))
        .arg("run")
        .assert()
        .success()
        .stdout(predicates::str::contains("Applied:"));

    let content = fs::read_to_string(file).expect("read result");
    assert_eq!(content, "alpha beta\n");
}

#[test]
fn run_with_no_config_fails() {
    let temp = TempDir::new().expect("temp dir");

    replace_cmd(&temp)
        .arg("run")
        .assert()
        .failure()
        .stderr(predicates::str::contains("no rules configured"));
}

#[test]
fn run_apply_runs_post_hook_after_rule_failure() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("sample.txt"), "omega beta\n").expect("write fixture");
    let post_path = temp.path().join("post.txt");
    let config = format!(
        r#"
apply_immediately = true

[hooks]
post_apply = ["printf post > {post_path}"]

[[rules]]
type = "literal"
find = "omega"
replace = "alpha"

[[rules]]
type = "regex"
pattern = "["
replacement = "broken"
"#,
        post_path = post_path.display()
    );
    fs::write(temp.path().join("replace.toml"), config).expect("write config");

    replace_with_config(temp.path(), &temp.path().join("replace.toml"))
        .arg("run")
        .assert()
        .failure();

    assert_eq!(
        fs::read_to_string(post_path).expect("read post hook"),
        "post"
    );
}

const RENAMING_RULE: &str = r#"
apply_immediately = true

[[rules]]
type = "literal"
find = "omega"
replace = "alpha"
"#;

fn run_in_workspace_with_omega_file(extra_args: &[&str]) -> TempDir {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("omega.txt"), "omega\n").expect("write fixture");
    fs::write(temp.path().join("replace.toml"), RENAMING_RULE).expect("write config");
    replace_with_config(temp.path(), &temp.path().join("replace.toml"))
        .args(extra_args)
        .arg("run")
        .assert()
        .success();
    temp
}

#[test]
fn run_renames_matching_names_like_a_direct_rewrite() {
    let temp = run_in_workspace_with_omega_file(&[]);

    assert_eq!(
        fs::read_to_string(temp.path().join("alpha.txt")).expect("renamed file"),
        "alpha\n"
    );
}

#[test]
fn run_with_skip_paths_keeps_the_names() {
    let temp = run_in_workspace_with_omega_file(&["--skip-paths"]);

    assert_eq!(
        fs::read_to_string(temp.path().join("omega.txt")).expect("kept file"),
        "alpha\n"
    );
}
