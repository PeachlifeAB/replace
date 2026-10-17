//! A multi-file write that fails part way restores the files it already wrote.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use assert_fs::TempDir;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

const ORIGINAL_CONTENT: &str = "foo\n";
const WRITABLE_DIR_MODE: u32 = 0o755;
const READ_ONLY_DIR_MODE: u32 = 0o555;

/// `first/one.txt` is written before `second/two.txt`; the directory of the second file is then
/// made read-only, so writing the second file fails after the first one succeeded.
struct Workspace {
    temp: TempDir,
    first_file: PathBuf,
    second_dir: PathBuf,
}

impl Workspace {
    fn create() -> Self {
        let temp = TempDir::new().expect("temp dir");
        let first_dir = temp.path().join("first");
        let second_dir = temp.path().join("second");
        fs::create_dir_all(&first_dir).expect("create first dir");
        fs::create_dir_all(&second_dir).expect("create second dir");
        let first_file = first_dir.join("one.txt");
        fs::write(&first_file, ORIGINAL_CONTENT).expect("write first file");
        fs::write(second_dir.join("two.txt"), ORIGINAL_CONTENT).expect("write second file");
        Self {
            temp,
            first_file,
            second_dir,
        }
    }

    fn lock_second_dir(&self) {
        set_dir_mode(&self.second_dir, READ_ONLY_DIR_MODE);
    }

    fn unlock_second_dir(&self) {
        set_dir_mode(&self.second_dir, WRITABLE_DIR_MODE);
    }
}

fn set_dir_mode(dir: &Path, mode: u32) {
    fs::set_permissions(dir, fs::Permissions::from_mode(mode)).expect("set dir mode");
}

#[test]
fn direct_apply_restores_first_file_when_second_write_fails() {
    let workspace = Workspace::create();
    workspace.lock_second_dir();

    let output = replace_cmd(&workspace.temp)
        .args(["literal", "foo", "bar", "--apply", "first", "second"])
        .output()
        .expect("apply runs");

    workspace.unlock_second_dir();
    assert!(!output.status.success(), "apply must fail");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("rolled back"),
        "stderr must report the rollback: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(&workspace.first_file).expect("read first file"),
        ORIGINAL_CONTENT
    );
}

#[test]
fn staged_batch_apply_restores_first_file_when_second_write_fails() {
    let workspace = Workspace::create();
    replace_cmd(&workspace.temp)
        .args(["literal", "foo", "bar", "--stage", "first", "second"])
        .assert()
        .success();
    workspace.lock_second_dir();

    let output = replace_cmd(&workspace.temp)
        .arg("apply")
        .output()
        .expect("apply runs");

    workspace.unlock_second_dir();
    assert!(!output.status.success(), "batch apply must fail");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("rolled back"),
        "stderr must report the rollback: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(&workspace.first_file).expect("read first file"),
        ORIGINAL_CONTENT
    );
}
