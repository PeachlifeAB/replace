//! The stage lock is held for a whole command and `--lock-timeout` bounds the wait for it.
use std::time::{Duration, Instant};

use assert_fs::TempDir;
use assert_fs::prelude::*;
use camino::Utf8PathBuf;
use replace::stage::StageManager;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::{replace_cmd, stage_foo_to_bar};

fn root_of(temp: &TempDir) -> Utf8PathBuf {
    Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).expect("utf8 temp dir")
}

#[test]
fn a_manager_with_a_timeout_stops_waiting_for_a_held_lock() {
    let temp = TempDir::new().expect("temp dir");
    let holder = StageManager::new(root_of(&temp));
    let _held = holder.lock_exclusive().expect("first lock");
    let waiter =
        StageManager::new(root_of(&temp)).with_lock_timeout(Some(Duration::from_millis(200)));

    let started = Instant::now();
    let result = waiter.lock_exclusive();

    assert!(result.is_err(), "the lock is held elsewhere");
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn every_locking_command_gives_up_after_the_timeout_and_changes_nothing() {
    let temp = TempDir::new().expect("temp dir");
    temp.child("a.txt").write_str("foo\n").expect("fixture");
    let staged = stage_foo_to_bar(&temp, "a.txt");
    let holder = StageManager::new(root_of(&temp));
    let _held = holder.lock_exclusive().expect("hold the lock");

    for command in [
        vec!["apply"],
        vec!["discard"],
        vec!["gc"],
        vec!["undo", staged.id.as_str()],
    ] {
        replace_cmd(&temp)
            .args(["--yes", "--lock-timeout", "200ms"])
            .args(&command)
            .timeout(Duration::from_secs(10))
            .assert()
            .failure()
            .code(8);
    }

    temp.child("a.txt").assert("foo\n");
    assert_eq!(holder.list_stages().len(), 1, "the stage is still staged");
}
