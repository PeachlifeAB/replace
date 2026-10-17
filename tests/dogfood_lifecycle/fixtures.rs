//! Fixture setup and the expected-tree assertion for the dogfood lifecycle.
use std::fs;
use std::path::Path;

use assert_fs::TempDir;

pub fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create target directory");
    for entry in fs::read_dir(from).expect("read fixture directory") {
        let entry = entry.expect("read fixture entry");
        let source = entry.path();
        let target = to.join(entry.file_name());
        if source.is_dir() {
            copy_tree(&source, &target);
        } else {
            fs::copy(&source, &target).expect("copy fixture file");
        }
    }
}

pub fn assert_tree_matches(actual: &Path, expected: &Path) {
    for entry in fs::read_dir(expected).expect("read expected directory") {
        let entry = entry.expect("read expected entry");
        if entry.file_name() == ".replace" {
            continue;
        }
        let expected_path = entry.path();
        let actual_path = actual.join(entry.file_name());
        if expected_path.is_dir() {
            assert_tree_matches(&actual_path, &expected_path);
        } else {
            let actual_content = fs::read_to_string(&actual_path)
                .unwrap_or_else(|error| panic!("read {}: {error}", actual_path.display()));
            let expected_content = fs::read_to_string(&expected_path).expect("read expected file");
            assert_eq!(
                actual_content,
                expected_content,
                "{}",
                actual_path.display()
            );
        }
    }
}

pub fn setup_fixtures(temp: &TempDir, fixture: &Path) {
    copy_tree(&fixture.join("input"), temp.path());
    fs::copy(
        fixture.join("replace.toml"),
        temp.path().join("replace.toml"),
    )
    .expect("copy dogfood config");
    fs::write(temp.path().join("workspace/code.tsx"), "var answer = 42;\n")
        .expect("write ast fixture");
    fs::write(temp.path().join("workspace/regex-only.txt"), "demo-123\n")
        .expect("write regex fixture");
    fs::write(
        temp.path().join("workspace/dot-literal.txt"),
        "find foo. but keep foobar and foo without a dot\n",
    )
    .expect("write dot literal fixture");
    fs::create_dir_all(temp.path().join("workspace/recursive-only/nested/deeper"))
        .expect("create recursive fixture dirs");
    fs::write(
        temp.path().join("workspace/recursive-only/root.txt"),
        "all-files-token at root\n",
    )
    .expect("write recursive root fixture");
    fs::write(
        temp.path()
            .join("workspace/recursive-only/nested/deeper/leaf.txt"),
        "all-files-token in nested leaf\n",
    )
    .expect("write recursive leaf fixture");
    fs::create_dir_all(temp.path().join("workspace/include-paths/nested"))
        .expect("create include-paths fixture dirs");
    fs::write(
        temp.path()
            .join("workspace/include-paths/nested/hello.name.txt"),
        "filename-only fixture\n",
    )
    .expect("write include-paths fixture");
}

pub fn cleanup_and_assert(temp: &TempDir, fixture: &Path) {
    fs::remove_file(temp.path().join("workspace/code.tsx")).expect("remove extra ast fixture");
    fs::remove_file(temp.path().join("workspace/regex-only.txt"))
        .expect("remove extra regex fixture");
    fs::remove_file(temp.path().join("workspace/dot-literal.txt"))
        .expect("remove extra dot literal fixture");
    fs::remove_dir_all(temp.path().join("workspace/recursive-only"))
        .expect("remove extra recursive fixture");
    fs::remove_dir_all(temp.path().join("workspace/include-paths"))
        .expect("remove extra include-paths fixture");
    assert_tree_matches(temp.path(), &fixture.join("expected"));
}
