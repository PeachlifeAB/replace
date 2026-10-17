//! An AST rewrite changes only the matched call; shell-sensitive text around it is kept byte for byte.
use std::fs;

use assert_fs::TempDir;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

#[test]
fn shell_sensitive_arguments_survive_an_applied_rewrite_byte_for_byte() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("escaping.go");
    fs::write(
        &file,
        "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"$VALUE\", \"double quote\", `raw backtick`, '\\'')\n}\n",
    )
    .expect("fixture");

    replace_cmd(&temp)
        .args([
            "ast",
            "fmt.Println($$$ARGS)",
            "log.Println($$$ARGS)",
            "--apply",
            "escaping.go",
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&file).expect("read"),
        "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tlog.Println(\"$VALUE\", \"double quote\", `raw backtick`, '\\'')\n}\n"
    );
}
