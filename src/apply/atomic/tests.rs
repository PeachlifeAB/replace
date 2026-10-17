use time::OffsetDateTime;

use camino::Utf8PathBuf;

use super::*;
use crate::stage::{StageEntry, StageId, StageMode};

fn apply_whole(manager: &StageManager, ids: &[String]) -> Result<Vec<ApplyResult>, ReplaceError> {
    let lock = manager.lock_exclusive().expect("stage lock");
    let selections: Vec<StageSelection> = ids.iter().map(StageSelection::whole).collect();
    apply_stages_atomic(manager, &lock, &selections)
}

fn stage_entry(id_seed: &[u8], file: Utf8PathBuf, find: &str, replace: &str) -> StageEntry {
    StageEntry {
        ignore_case: false,
        id: StageId::from_stage_bytes(id_seed),
        mode: StageMode::Literal,
        pattern: find.to_owned(),
        rewrite: replace.to_owned(),
        file,
        match_count: 1,
        source_sha: Some(content_sha256("foo\n")),
        after_sha: None,
        language: None,
        anchors: Vec::new(),
        captures: None,
        parent_function: None,
        parent_class: None,
        injected_language: None,
        git_blame_author: None,
        selector: None,
        kind: None,
        strictness: None,
        sg_args: vec![],
        ast_rule_yaml: None,
        command_argv: vec!["replace".to_owned()],
        cwd: std::env::current_dir()
            .expect("current dir")
            .to_string_lossy()
            .to_string(),
        author: None,
        timestamp: OffsetDateTime::now_utc().to_string(),
        created_at: OffsetDateTime::now_utc(),
    }
}

#[test]
fn atomic_apply_fails_when_patch_is_missing() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).expect("utf8 temp");
    let file = root.join("sample.txt");
    std::fs::write(&file, "foo\n").expect("write fixture");
    let manager = StageManager::new(&root);
    let mut entry = stage_entry(b"missing-patch", file.clone(), "foo", "bar");
    manager
        .write_stage(&mut entry, "patch", false)
        .expect("write stage");
    std::fs::remove_file(
        root.join(".replace/stage")
            .join(format!("{}.patch", entry.id.as_str())),
    )
    .expect("remove patch");

    let result = apply_whole(&manager, &[entry.id.as_str().to_owned()]);

    assert!(result.is_err());
    assert_eq!(
        std::fs::read_to_string(file).expect("read fixture"),
        "foo\n"
    );
}

#[test]
fn atomic_apply_rejects_duplicate_paths() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).expect("utf8 temp");
    let file = root.join("sample.txt");
    std::fs::write(&file, "foo\n").expect("write fixture");
    let manager = StageManager::new(&root);
    let mut first = stage_entry(b"first", file.clone(), "foo", "bar");
    let mut second = stage_entry(b"second", file.clone(), "foo", "baz");
    manager
        .write_stage(&mut first, "patch 1", false)
        .expect("write first");
    manager
        .write_stage(&mut second, "patch 2", false)
        .expect("write second");

    let result = apply_whole(
        &manager,
        &[first.id.as_str().to_owned(), second.id.as_str().to_owned()],
    );

    assert!(result.is_err());
    assert_eq!(
        std::fs::read_to_string(file).expect("read fixture"),
        "foo\n"
    );
}

#[test]
fn atomic_apply_detects_moved_files() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).expect("utf8 temp");

    let original = root.join("original.txt");
    std::fs::write(&original, "foo\n").expect("write fixture");

    let manager = StageManager::new(&root);
    let mut entry = stage_entry(b"moved", original.clone(), "foo", "bar");
    manager
        .write_stage(&mut entry, "patch", false)
        .expect("write stage");

    let moved = root.join("moved.txt");
    std::fs::rename(&original, &moved).expect("rename file");

    let result = apply_whole(&manager, &[entry.id.as_str().to_owned()]);

    assert!(result.is_err(), "expected error when file is missing");
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(
        error_msg.contains("deleted") || error_msg.contains("missing"),
        "expected file deletion or missing error: {error_msg}"
    );
}
