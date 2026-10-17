use super::*;
use time::OffsetDateTime;

fn temp_root() -> std::io::Result<(tempfile::TempDir, Utf8PathBuf)> {
    let tmp = tempfile::tempdir()?;
    let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).map_err(|path| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("non-utf8 temp dir: {}", path.display()),
        )
    })?;

    Ok((tmp, root))
}

#[test]
fn stage_id_from_bytes_is_deterministic() {
    let id1 = StageId::from_stage_bytes(b"test");
    let id2 = StageId::from_stage_bytes(b"test");
    assert_eq!(id1, id2);
    assert_eq!(id1.as_str().len(), 12);
}

#[test]
fn stage_manager_list_empty() -> std::io::Result<()> {
    let (_tmp, root) = temp_root()?;
    let manager = StageManager::new(&root);

    let stages = manager.list_stages();
    assert!(stages.is_empty());

    Ok(())
}

/// A staged Go rewrite with the given id, file and source hash; everything else is fixed.
fn go_stage(id: &str, file: &str, source_sha: &str) -> std::io::Result<StageEntry> {
    Ok(StageEntry {
        ignore_case: false,
        id: StageId(id.to_owned()),
        mode: StageMode::Ast,
        pattern: "var $A".to_owned(),
        rewrite: "let $A".to_owned(),
        file: Utf8PathBuf::from(file),
        match_count: 1,
        source_sha: Some(source_sha.to_owned()),
        after_sha: None,
        language: Some("go".to_owned()),
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
        cwd: std::env::current_dir()?.to_string_lossy().to_string(),
        author: None,
        timestamp: OffsetDateTime::now_utc().to_string(),
        created_at: OffsetDateTime::now_utc(),
    })
}

#[test]
fn stage_manager_write_and_read() -> std::io::Result<()> {
    let (_tmp, root) = temp_root()?;
    let manager = StageManager::new(&root);
    let mut entry = go_stage("abc123def456", "test.go", "abc123")?;
    entry.id = StageId::from_stage_bytes(b"test");

    manager.write_stage(&mut entry, "diff --git a/test.go", false)?;
    let read = manager.read_stage(entry.id.as_str()).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "stage entry not found")
    })?;

    assert_eq!(read.id, entry.id);
    assert_eq!(read.pattern, entry.pattern);

    Ok(())
}

#[test]
fn stage_manager_collision_appends_suffix() -> std::io::Result<()> {
    let (_tmp, root) = temp_root()?;
    let manager = StageManager::new(&root);

    let mut first = go_stage("abc123def456", "test1.go", "sha1")?;
    manager.write_stage(&mut first, "patch1", false)?;
    assert_eq!(first.id.as_str(), "abc123def456");

    let mut second = go_stage("abc123def456", "test2.go", "sha2")?;
    manager.write_stage(&mut second, "patch2", true)?;
    assert_eq!(second.id.as_str(), "abc123def456-2");

    assert!(manager.read_stage("abc123def456").is_some());
    assert!(manager.read_stage("abc123def456-2").is_some());

    let mut third = go_stage("abc123def456", "test3.go", "sha3")?;
    manager.write_stage(&mut third, "patch3", true)?;
    assert_eq!(third.id.as_str(), "abc123def456-3");

    Ok(())
}
