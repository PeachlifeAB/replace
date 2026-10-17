//! The AST language comes from the file extension, the CLI flag, or a `lang_by_glob` config rule, in that order.
use std::fs;

use assert_fs::TempDir;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::{replace_cmd, replace_with_config, stage_id};

const GO_PRINT: &str =
    "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"hello\")\n}\n";

/// The `Language:` line `replace explain` reports for a staged AST rewrite of `file`.
fn explained_language(temp: &TempDir, file: &str, extra_args: &[&str]) -> String {
    let mut args = vec!["ast"];
    args.extend_from_slice(extra_args);
    args.extend_from_slice(&["fmt.Println($A)", "log.Println($A)", file]);
    let staged = replace_cmd(temp)
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let id = stage_id(&String::from_utf8(staged).expect("utf8")).to_owned();
    let explained = replace_cmd(temp)
        .args(["explain", &id])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    String::from_utf8(explained)
        .expect("utf8")
        .lines()
        .find_map(|line| line.strip_prefix("Language: "))
        .expect("Language line")
        .to_owned()
}

#[test]
fn the_language_is_inferred_from_the_file_extension() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.go"), GO_PRINT).expect("fixture");

    assert_eq!(explained_language(&temp, "main.go", &[]), "go");
}

#[test]
fn the_cli_language_flag_overrides_the_extension() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.go"), GO_PRINT).expect("fixture");

    assert_eq!(
        explained_language(&temp, "main.go", &["--lang", "go"]),
        "go"
    );
}

#[test]
fn a_lang_by_glob_rule_picks_the_language_for_an_unknown_extension() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.customgo"), GO_PRINT).expect("fixture");
    let config = temp.path().join("config.toml");
    fs::write(
        &config,
        "[[lang_by_glob]]\nglob = \"*.customgo\"\nlanguage = \"go\"\n",
    )
    .expect("config");

    let staged = replace_with_config(temp.path(), &config)
        .args(["ast", "fmt.Println($A)", "log.Println($A)", "main.customgo"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let id = stage_id(&String::from_utf8(staged).expect("utf8")).to_owned();
    let explained = replace_with_config(temp.path(), &config)
        .args(["explain", &id])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert!(
        String::from_utf8(explained)
            .expect("utf8")
            .contains("Language: go")
    );
}

#[test]
fn the_cli_language_flag_overrides_a_lang_by_glob_rule() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("main.txt"), GO_PRINT).expect("fixture");
    let config = temp.path().join("config.toml");
    fs::write(
        &config,
        "[[lang_by_glob]]\nglob = \"*.txt\"\nlanguage = \"python\"\n",
    )
    .expect("config");

    let staged = replace_with_config(temp.path(), &config)
        .args([
            "ast",
            "--lang",
            "go",
            "fmt.Println($A)",
            "log.Println($A)",
            "main.txt",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let id = stage_id(&String::from_utf8(staged).expect("utf8")).to_owned();
    let explained = replace_with_config(temp.path(), &config)
        .args(["explain", &id])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert!(
        String::from_utf8(explained)
            .expect("utf8")
            .contains("Language: go")
    );
}
