//! Which files an AST rewrite may touch: symlinks, binaries, encodings, size caps, flags and ignores.
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use assert_fs::TempDir;
use predicates::prelude::*;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::{replace_cmd, replace_with_config};

const GO_PRINT: &str =
    "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"hello\")\n}\n";
const GO_LOG: &str =
    "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tlog.Println(\"hello\")\n}\n";

fn apply_println(temp: &TempDir, args: &[&str]) -> assert_cmd::assert::Assert {
    let mut full = vec!["ast", "fmt.Println($A)", "log.Println($A)", "--apply"];
    full.extend_from_slice(args);
    replace_cmd(temp).args(full).assert()
}

#[test]
fn a_symlink_target_is_left_unchanged_when_the_link_is_rewritten() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("target.go"), GO_PRINT).expect("fixture");
    std::os::unix::fs::symlink(temp.path().join("target.go"), temp.path().join("link.go"))
        .expect("symlink");

    apply_println(&temp, &["link.go"]).success();

    assert_eq!(
        fs::read_to_string(temp.path().join("target.go")).expect("read"),
        GO_PRINT
    );
}

#[test]
fn a_symlink_skip_is_labelled_and_exits_zero() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("target.go"), GO_PRINT).expect("fixture");
    std::os::unix::fs::symlink(temp.path().join("target.go"), temp.path().join("link.go"))
        .expect("symlink");

    apply_println(&temp, &["link.go"])
        .success()
        .stderr(predicate::str::contains("REPLACE_W_SKIPPED_SYMLINK"));
}

#[test]
fn a_binary_file_is_left_unchanged() {
    let temp = TempDir::new().expect("temp dir");
    let bytes = b"\x00\x01fmt.Println(\"hello\")\x02";
    fs::write(temp.path().join("binary.bin"), bytes).expect("fixture");

    apply_println(&temp, &["binary.bin"]).success();

    assert_eq!(
        fs::read(temp.path().join("binary.bin")).expect("read"),
        bytes
    );
}

#[test]
fn a_binary_skip_is_labelled_and_exits_zero() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(
        temp.path().join("binary.bin"),
        b"\x00\x01fmt.Println(\"hello\")\x02",
    )
    .expect("fixture");

    apply_println(&temp, &["binary.bin"])
        .success()
        .stderr(predicate::str::contains("REPLACE_W_SKIPPED_BINARY"));
}

#[test]
fn a_non_utf8_file_is_left_unchanged() {
    let temp = TempDir::new().expect("temp dir");
    let utf16_bytes = b"\xff\xfep\x00a\x00c\x00k\x00a\x00g\x00e\x00 \x00m\x00a\x00i\x00n\x00\n\x00";
    fs::write(temp.path().join("utf16.go"), utf16_bytes).expect("fixture");

    apply_println(&temp, &["utf16.go"]).success();

    assert_eq!(
        fs::read(temp.path().join("utf16.go")).expect("read"),
        utf16_bytes
    );
}

#[test]
fn a_non_utf8_skip_is_labelled_and_exits_zero() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(
        temp.path().join("utf16.go"),
        b"\xff\xfep\x00a\x00c\x00k\x00a\x00g\x00e\x00",
    )
    .expect("fixture");

    apply_println(&temp, &["utf16.go"])
        .success()
        .stderr(predicate::str::contains("REPLACE_W_SKIPPED_ENCODING"));
}

#[test]
fn a_file_over_the_size_cap_is_left_unchanged() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("large.go"), GO_PRINT).expect("fixture");

    replace_cmd(&temp)
        .args([
            "--max-file-size",
            "1B",
            "ast",
            "fmt.Println($A)",
            "log.Println($A)",
            "--apply",
            "large.go",
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(temp.path().join("large.go")).expect("read"),
        GO_PRINT
    );
}

#[test]
fn a_size_cap_skip_is_labelled_and_exits_zero() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("large.go"), GO_PRINT).expect("fixture");

    replace_cmd(&temp)
        .args([
            "--max-file-size",
            "1B",
            "ast",
            "fmt.Println($A)",
            "log.Println($A)",
            "--apply",
            "large.go",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("REPLACE_W_SKIPPED_LARGE_FILE"));
}

#[test]
fn zero_matches_is_labelled_and_exits_zero() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("zero.go"), GO_PRINT).expect("fixture");

    replace_cmd(&temp)
        .args(["ast", "fmt.Errorf($A)", "log.Printf($A)", "zero.go"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No matches"))
        .stderr(predicate::str::contains("REPLACE_W_ZERO_MATCHES"));
}

#[cfg(unix)]
#[test]
fn file_permissions_survive_an_applied_rewrite() {
    let temp = TempDir::new().expect("temp dir");
    let path = temp.path().join("mode.go");
    fs::write(&path, GO_PRINT).expect("fixture");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");

    apply_println(&temp, &["mode.go"]).success();

    assert_eq!(
        fs::metadata(&path).expect("meta").permissions().mode() & 0o777,
        0o755
    );
    assert_eq!(fs::read_to_string(&path).expect("read"), GO_LOG);
}

#[test]
fn a_path_after_double_dash_is_taken_verbatim_even_when_it_starts_with_a_dash() {
    let temp = TempDir::new().expect("temp dir");
    let dash_dir = temp.path().join("--flag-like-dir");
    fs::create_dir(&dash_dir).expect("mkdir");
    fs::write(dash_dir.join("main.go"), GO_PRINT).expect("fixture");

    apply_println(&temp, &["--", "--flag-like-dir"]).success();

    assert_eq!(
        fs::read_to_string(dash_dir.join("main.go")).expect("read"),
        GO_LOG
    );
}

#[test]
fn include_and_exclude_globs_limit_which_files_are_rewritten() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("keep.go"), GO_PRINT).expect("fixture");
    fs::write(temp.path().join("skip.go"), GO_PRINT).expect("fixture");

    apply_println(&temp, &["--include", "*.go", "--exclude", "skip.go", "."]).success();

    assert_eq!(
        fs::read_to_string(temp.path().join("keep.go")).expect("read"),
        GO_LOG
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("skip.go")).expect("read"),
        GO_PRINT
    );
}

#[test]
fn backup_is_not_written_for_a_skipped_binary_file() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("binary.bin"), b"\x00binary").expect("fixture");

    replace_cmd(&temp)
        .args([
            "ast",
            "binary",
            "changed",
            "--backup",
            "--apply",
            "binary.bin",
        ])
        .assert()
        .success();

    assert!(!temp.path().join("binary.bin.orig").exists());
}

#[test]
fn a_config_exclude_list_keeps_the_listed_file_unchanged() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("generated.go"), GO_PRINT).expect("fixture");
    let config = temp.path().join("config.toml");
    fs::write(&config, "exclude = [\"generated.go\"]\n").expect("config");

    replace_with_config(temp.path(), &config)
        .args([
            "ast",
            "--lang",
            "go",
            "fmt.Println($A)",
            "log.Println($A)",
            "--apply",
            "generated.go",
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(temp.path().join("generated.go")).expect("read"),
        GO_PRINT
    );
}

#[test]
fn node_modules_is_skipped_by_default() {
    let temp = TempDir::new().expect("temp dir");
    fs::create_dir(temp.path().join("node_modules")).expect("mkdir");
    fs::write(temp.path().join("node_modules/main.go"), GO_PRINT).expect("fixture");

    apply_println(&temp, &["."]);

    assert_eq!(
        fs::read_to_string(temp.path().join("node_modules/main.go")).expect("read"),
        GO_PRINT
    );
}

#[test]
fn no_default_ignores_includes_node_modules() {
    let temp = TempDir::new().expect("temp dir");
    fs::create_dir(temp.path().join("node_modules")).expect("mkdir");
    fs::write(temp.path().join("node_modules/main.go"), GO_PRINT).expect("fixture");

    apply_println(&temp, &["--no-default-ignores", "."]).success();

    assert_eq!(
        fs::read_to_string(temp.path().join("node_modules/main.go")).expect("read"),
        GO_LOG
    );
}
