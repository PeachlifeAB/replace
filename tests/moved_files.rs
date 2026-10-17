//! A staged file that moved is found and rewritten at its new location.
use assert_fs::TempDir;
use std::fs;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

#[test]
fn moved_file_detection_applies_to_moved_location() {
    let temp = TempDir::new().expect("temp dir");

    // Initialize git repo
    replace_cmd(&temp).args(["--help"]).assert().success();

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&temp)
        .output()
        .expect("git init");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&temp)
        .output()
        .expect("git config");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&temp)
        .output()
        .expect("git config");

    // Create and commit a file
    let file1 = temp.path().join("original.txt");
    fs::write(&file1, "hello world\n").expect("write fixture");

    std::process::Command::new("git")
        .args(["add", "original.txt"])
        .current_dir(&temp)
        .output()
        .expect("git add");

    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(&temp)
        .output()
        .expect("git commit");

    // Stage a rewrite
    let output = replace_cmd(&temp)
        .args(["literal", "hello", "goodbye", "--stage"])
        .output()
        .expect("replace stage");

    assert!(
        output.status.success(),
        "stage should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stage_id = String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("Staged: "))
        .and_then(|rest| rest.split_whitespace().next())
        .expect("stage id in output")
        .to_owned();

    // Move the file (simulating git mv)
    let file2 = temp.path().join("moved.txt");
    std::process::Command::new("git")
        .args(["mv", "original.txt", "moved.txt"])
        .current_dir(&temp)
        .output()
        .expect("git mv");

    // Apply the stage - should detect the moved file
    let output = replace_cmd(&temp)
        .args(["apply", &stage_id])
        .output()
        .expect("replace apply");

    assert!(
        output.status.success(),
        "apply should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify the file was modified at the new location
    let content = fs::read_to_string(&file2).expect("read moved file");
    assert!(
        content.contains("goodbye"),
        "moved file should contain replacement"
    );
    assert!(
        !content.contains("hello"),
        "moved file should not contain original"
    );

    // Verify the original location doesn't exist
    assert!(!file1.exists(), "original file should not exist");
}
