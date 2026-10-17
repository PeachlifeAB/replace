use assert_fs::TempDir;
use assert_fs::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

fn workspace() -> TempDir {
    let temp = TempDir::new().expect("temp dir");
    temp.child("src/main.rs")
        .write_str("fn main() { println!(\"foo\"); }\n")
        .expect("write fixture");
    temp
}

#[test]
fn max_file_size_accepts_every_unit_the_help_advertises() {
    let temp = workspace();
    for size in ["5MiB", "100KiB", "1KB", "1024B", "1024"] {
        replace_cmd(&temp)
            .args(["--max-file-size", size, "literal", "foo", "bar", "."])
            .assert()
            .success();
        replace_cmd(&temp)
            .args(["discard", "-y"])
            .assert()
            .success();
    }
}

#[test]
fn languages_lists_rust_and_every_listed_language_accepts_ast() {
    let temp = workspace();
    temp.child("empty.txt")
        .write_str("")
        .expect("empty fixture");
    let listing = replace_cmd(&temp)
        .args(["--json", "languages"])
        .output()
        .expect("languages runs");
    let json: serde_json::Value = serde_json::from_slice(&listing.stdout).expect("json output");
    let languages: Vec<&str> = json["languages"]
        .as_array()
        .expect("languages array")
        .iter()
        .map(|language| language.as_str().expect("language name"))
        .collect();
    assert!(
        languages.contains(&"rust"),
        "rust missing from {languages:?}"
    );
    for language in languages {
        replace_cmd(&temp)
            .args(["ast", "--lang", language, "zzz_no_match", "x", "empty.txt"])
            .assert()
            .success();
    }
}

#[test]
fn ast_infers_rust_from_extension() {
    let temp = workspace();
    replace_cmd(&temp)
        .args(["ast", "println!($A)", "eprintln!($A)", "src"])
        .assert()
        .success()
        .stdout(predicates::str::contains("eprintln!"));
}

#[test]
fn from_error_finds_path_in_rustc_and_dot_slash_diagnostics() {
    let temp = workspace();
    for diagnostic in [
        "error[E0425]: cannot find value `x`\n --> src/main.rs:1:1",
        "error at ./src/main.rs:2",
    ] {
        replace_cmd(&temp)
            .args(["from-error", diagnostic])
            .assert()
            .success()
            .stdout(predicates::str::contains("Staged:"));
        replace_cmd(&temp)
            .args(["discard", "-y"])
            .assert()
            .success();
    }
}
