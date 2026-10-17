use std::fs;

use assert_fs::TempDir;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::ast_preview_replacements;

const SWIFT_RETURNS: &str = "func load(_ value: Int) -> Int {\n    return value * 2\n}\n\nfunc other() -> Int {\n    return 3\n}\n";

#[test]
fn bare_swift_return_pattern_matches_inside_function_bodies() {
    let temp = TempDir::new().expect("temp dir");
    let source = temp.path().join("returns.swift");
    fs::write(&source, SWIFT_RETURNS).expect("write fixture");

    let replacements =
        ast_preview_replacements(temp.path(), "return $A", "return wrap($A)", &source);

    assert_eq!(replacements, 2);
}
