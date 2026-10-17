use super::*;

#[test]
fn only_files_with_nothing_but_crlf_count_as_crlf() {
    assert!(is_uniformly_crlf("a\r\nb\r\n"));
    assert!(!is_uniformly_crlf("a\nb\n"));
    assert!(!is_uniformly_crlf("a\r\nb\n"));
    assert!(!is_uniformly_crlf("no breaks"));
}

#[test]
fn bare_lf_becomes_crlf_without_doubling_existing_crlf() {
    assert_eq!(with_crlf_line_breaks("a\nb\r\nc\n"), "a\r\nb\r\nc\r\n");
}

#[test]
fn write_atomic_preserves_lf() {
    let temp = tempfile::tempdir().expect("temp dir");
    let file = Utf8PathBuf::from_path_buf(temp.path().join("test.txt")).unwrap();

    std::fs::write(&file, "line1\nline2\nline3\n").unwrap();

    write_atomic(&file, "line1\nmodified\nline3\n").unwrap();

    let content = std::fs::read_to_string(&file).unwrap();
    assert_eq!(content, "line1\nmodified\nline3\n");
    assert!(!content.contains("\r\n"));
}

#[test]
fn write_atomic_preserves_crlf() {
    let temp = tempfile::tempdir().expect("temp dir");
    let file = Utf8PathBuf::from_path_buf(temp.path().join("test.txt")).unwrap();

    std::fs::write(&file, "line1\r\nline2\r\nline3\r\n").unwrap();

    write_atomic(&file, "line1\nmodified\nline3\n").unwrap();

    let content = std::fs::read_to_string(&file).unwrap();
    assert_eq!(content, "line1\r\nmodified\r\nline3\r\n");
}
