use assert_fs::TempDir;
use assert_fs::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

const SOURCE: &str = "fn main() { println!(\"x\"); }\n";
const RULE: &str =
    "id: print-to-stderr\nlanguage: Rust\nrule:\n  pattern: println!($A)\nfix: eprintln!($A)\n";

fn workspace() -> TempDir {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.rs").write_str(SOURCE).expect("fixture");
    temp
}

#[test]
fn rule_file_scopes_the_rewrite_to_the_named_paths() {
    let temp = workspace();
    temp.child("kept/b.rs").write_str(SOURCE).expect("fixture");
    temp.child("print.yml").write_str(RULE).expect("rule");
    replace_cmd(&temp)
        .args(["ast", "--rule", "print.yml", "--apply", "kept"])
        .assert()
        .success();
    temp.child("kept/b.rs")
        .assert(predicates::str::contains("eprintln!"));
    temp.child("a.rs").assert(SOURCE);
}

#[test]
fn pattern_and_rewrite_files_match_inline_arguments() {
    let temp = workspace();
    temp.child("pattern.txt")
        .write_str("println!($A)\n")
        .expect("pattern");
    temp.child("rewrite.txt")
        .write_str("eprintln!($A)\n")
        .expect("rewrite");
    replace_cmd(&temp)
        .args([
            "ast",
            "--pattern-file",
            "pattern.txt",
            "--rewrite-file",
            "rewrite.txt",
            "--apply",
            "a.rs",
        ])
        .assert()
        .success();
    temp.child("a.rs")
        .assert("fn main() { eprintln!(\"x\"); }\n");
}

#[test]
fn pattern_file_conflicts_with_rule_file() {
    let temp = workspace();
    temp.child("pattern.txt")
        .write_str("println!($A)")
        .expect("pattern");
    replace_cmd(&temp)
        .args([
            "ast",
            "--pattern-file",
            "pattern.txt",
            "println!($A)",
            "eprintln!($A)",
            "a.rs",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn missing_pattern_file_is_an_invalid_argument() {
    let temp = workspace();
    replace_cmd(&temp)
        .args([
            "ast",
            "--pattern-file",
            "nope.txt",
            "--rewrite",
            "x",
            "a.rs",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn pattern_file_requires_rewrite_file() {
    let temp = workspace();
    temp.child("pattern.txt")
        .write_str("println!($A)")
        .expect("pattern");
    replace_cmd(&temp)
        .args(["ast", "--pattern-file", "pattern.txt", "a.rs"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicates::str::contains("--rewrite-file"));
}

fn run_with_stdin(temp: &TempDir, stdin: &[u8], args: &[&str]) -> assert_cmd::assert::Assert {
    replace_cmd(temp)
        .args(args)
        .write_stdin(stdin.to_vec())
        .assert()
}

#[test]
fn files0_from_stdin_reads_nul_delimited_paths_including_spaces() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a b.txt").write_str("foo\n").expect("fixture");
    temp.child("c.txt").write_str("foo\n").expect("fixture");
    temp.child("untouched.txt")
        .write_str("foo\n")
        .expect("fixture");
    run_with_stdin(
        &temp,
        b"a b.txt\0c.txt",
        &["--files0-from", "-", "literal", "foo", "bar", "--apply"],
    )
    .success();
    temp.child("a b.txt").assert("bar\n");
    temp.child("c.txt").assert("bar\n");
    temp.child("untouched.txt").assert("foo\n");
}

#[test]
fn files0_from_file_reads_the_list_from_disk() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");
    temp.child("other.txt").write_str("foo\n").expect("fixture");
    temp.child("list").write_binary(b"a.txt\0").expect("list");
    replace_cmd(&temp)
        .args(["--files0-from", "list", "literal", "foo", "bar", "--apply"])
        .assert()
        .success();
    temp.child("a.txt").assert("bar\n");
    temp.child("other.txt").assert("foo\n");
}

#[test]
fn empty_files0_list_touches_nothing_instead_of_defaulting_to_cwd() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");
    run_with_stdin(
        &temp,
        b"",
        &["--files0-from", "-", "literal", "foo", "bar", "--apply"],
    )
    .success()
    .stdout(predicates::str::contains("No matches"));
    temp.child("a.txt").assert("foo\n");
}

#[test]
fn files0_from_conflicts_with_positional_paths() {
    let temp = workspace();
    run_with_stdin(
        &temp,
        b"a.rs\0",
        &["--files0-from", "-", "literal", "foo", "bar", "a.rs"],
    )
    .failure()
    .code(2)
    .stderr(predicates::str::contains("--files0-from"));
}
