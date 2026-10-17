//! A rewrite changes the matched text and nothing else about the file.
use std::fs;
use std::time::{Duration, SystemTime};

use assert_fs::TempDir;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

const BOM: &[u8] = b"\xEF\xBB\xBF";

fn apply_foo_to_bar(temp: &TempDir, file: &str) {
    replace_cmd(temp)
        .args(["literal", "foo", "bar", "--apply", file])
        .assert()
        .success();
}

#[test]
fn a_file_with_a_bom_keeps_exactly_one_bom() {
    let temp = TempDir::new().expect("temp dir");
    let path = temp.path().join("bom.txt");
    fs::write(&path, [BOM, b"foo\n"].concat()).expect("fixture");

    apply_foo_to_bar(&temp, "bom.txt");

    assert_eq!(fs::read(&path).expect("read"), [BOM, b"bar\n"].concat());
}

#[test]
fn mixed_line_endings_stay_as_they_were_on_untouched_lines() {
    let temp = TempDir::new().expect("temp dir");
    let path = temp.path().join("mixed.txt");
    fs::write(&path, b"foo\r\nkeep\nfoo\n").expect("fixture");

    apply_foo_to_bar(&temp, "mixed.txt");

    assert_eq!(fs::read(&path).expect("read"), b"bar\r\nkeep\nbar\n");
}

#[test]
fn a_crlf_file_stays_crlf_when_the_replacement_adds_line_breaks() {
    let temp = TempDir::new().expect("temp dir");
    let path = temp.path().join("crlf.txt");
    fs::write(&path, b"foo\r\nkeep\r\n").expect("fixture");

    replace_cmd(&temp)
        .args(["literal", "foo", "one\ntwo", "--apply", "crlf.txt"])
        .assert()
        .success();

    assert_eq!(fs::read(&path).expect("read"), b"one\r\ntwo\r\nkeep\r\n");
}

#[test]
fn an_applied_rewrite_gets_a_fresh_modification_time() {
    let temp = TempDir::new().expect("temp dir");
    let path = temp.path().join("old.txt");
    fs::write(&path, "foo\n").expect("fixture");
    let long_ago = SystemTime::now() - Duration::from_hours(1);
    filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(long_ago))
        .expect("age the file");

    apply_foo_to_bar(&temp, "old.txt");

    let modified = fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("mtime");
    assert!(
        modified > long_ago + Duration::from_mins(30),
        "build tools rely on mtime to see that the file changed"
    );
}
