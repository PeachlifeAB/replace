//! Caps on locks, files, matches and file size, and binary-file skipping.
use assert_fs::TempDir;
use std::fs;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

#[test]
fn lock_timeout_flag_accepted() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("sample.txt");
    fs::write(&file, "hello world\n").expect("write fixture");

    // Test that --lock-timeout flag is accepted (without actually timing out)
    replace_cmd(&temp)
        .args(["--lock-timeout", "30s", "status"])
        .assert()
        .success()
        .stdout(predicates::str::contains("0 staged"));
}

#[test]
fn lock_timeout_short_format() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("sample.txt");
    fs::write(&file, "hello world\n").expect("write fixture");

    // Test shorthand format (just seconds without 's')
    replace_cmd(&temp)
        .args(["--lock-timeout", "10", "status"])
        .assert()
        .success()
        .stdout(predicates::str::contains("0 staged"));
}

#[test]
fn max_files_cap_limits_scanned_files() {
    let temp = TempDir::new().expect("temp dir");

    // Create multiple files
    for i in 0..5 {
        let file = temp.path().join(format!("file{i}.txt"));
        fs::write(&file, "hello world\n").expect("write fixture");
    }

    // Test --max-files cap
    replace_cmd(&temp)
        .args(["literal", "hello", "goodbye", "--max-files", "2", "--stage"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("Limit reached"))
        .stderr(predicates::str::contains("max: 2"));
}

#[test]
fn max_file_size_cap_skips_large_files() {
    let temp = TempDir::new().expect("temp dir");

    // Create a small file
    let small = temp.path().join("small.txt");
    fs::write(&small, "hello world\n").expect("write fixture");

    // Create a file larger than 100 bytes
    let large = temp.path().join("large.txt");
    let large_content = "hello ".repeat(50); // 400 bytes
    fs::write(&large, &large_content).expect("write fixture");

    // Test --max-file-size cap
    replace_cmd(&temp)
        .args([
            "literal",
            "hello",
            "goodbye",
            "--max-file-size",
            "100B",
            "--stage",
        ])
        .assert()
        .success()
        .stderr(predicates::str::contains("Skipping large file"));
}

#[test]
fn max_matches_cap_limits_total_matches() {
    let temp = TempDir::new().expect("temp dir");

    // Create two files with matches
    let file1 = temp.path().join("file1.txt");
    fs::write(&file1, "hello\n").expect("write fixture");

    let file2 = temp.path().join("file2.txt");
    fs::write(&file2, "hello\n").expect("write fixture");

    // Test --max-matches cap (allows only 1 match total)
    replace_cmd(&temp)
        .args([
            "literal",
            "hello",
            "goodbye",
            "--max-matches",
            "1",
            "--stage",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("Limit reached"))
        .stderr(predicates::str::contains("max: 1"));
}

#[test]
fn max_matches_per_file_cap() {
    let temp = TempDir::new().expect("temp dir");

    // Create a file with multiple matches
    let file = temp.path().join("multi.txt");
    fs::write(&file, "hello hello hello\n").expect("write fixture");

    // Test -m cap (per-file)
    replace_cmd(&temp)
        .args(["literal", "hello", "goodbye", "-m", "2", "--stage"])
        .assert()
        .success()
        .stderr(predicates::str::contains("exceed per-file limit"));
}

#[test]
fn binary_file_is_skipped() {
    let temp = TempDir::new().expect("temp dir");

    // Create a binary file with null bytes
    let binary_file = temp.path().join("binary.bin");
    fs::write(&binary_file, b"hello\x00world\x00binary\n").expect("write binary fixture");

    // Create a text file
    let text_file = temp.path().join("text.txt");
    fs::write(&text_file, "hello world\n").expect("write text fixture");

    // Test that binary file is skipped but text file is processed
    replace_cmd(&temp)
        .args(["literal", "hello", "goodbye", "--stage"])
        .assert()
        .success()
        .stderr(predicates::str::contains("skipping binary file"))
        .stdout(predicates::str::contains("Staged:"));
}
