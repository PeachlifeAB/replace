//! `replace ast` rule files and argument validation.
use assert_fs::TempDir;
use std::fs;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

#[test]
fn rule_flag_loads_astgrep_yaml() {
    let temp = TempDir::new().expect("temp dir");

    // Create a TypeScript file
    let file = temp.path().join("test.tsx");
    fs::write(&file, "var x = 1;\n").expect("write fixture");

    // Create an ast-grep YAML rule file
    let rule_file = temp.path().join("rule.yml");
    let rule = "id: replace-var\nlanguage: TypeScript\nrule:\n  pattern: var $A = $B;\nfix: let $A = $B;\n";
    fs::write(&rule_file, rule).expect("write rule");

    // Stage a rewrite using the rule file
    let output = replace_cmd(&temp)
        .args(["ast", "--rule", rule_file.to_str().unwrap(), "--stage"])
        .output()
        .expect("replace ast");

    assert!(
        output.status.success(),
        "stage should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Staged:"), "should contain 'Staged:'");

    // Extract stage ID
    let stage_id = stdout
        .lines()
        .find_map(|line| line.strip_prefix("Staged: "))
        .and_then(|rest| rest.split_whitespace().next())
        .expect("stage id in output")
        .to_owned();

    // Apply the stage
    let output = replace_cmd(&temp)
        .args(["apply", &stage_id])
        .output()
        .expect("replace apply");

    assert!(
        output.status.success(),
        "apply should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify the file was modified
    let modified_content = fs::read_to_string(&file).expect("read modified file");
    assert_eq!(modified_content, "let x = 1;\n", "file should be modified");
    assert_eq!(
        fs::read_to_string(&rule_file).expect("read rule file"),
        rule,
        "rule file should not be rewritten"
    );
}

#[test]
fn ast_rejects_legacy_sg_arg_before_staging() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("test.ts");
    fs::write(&file, "const value = 1;\n").expect("write fixture");

    replace_cmd(&temp)
        .args([
            "ast",
            "const $A = $B;",
            "let $A = $B;",
            "--sg-arg=--debug-query",
            "test.ts",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("unsupported"));

    assert!(!temp.path().join(".replace/stage").exists());
}

#[test]
fn ast_rejects_conflicting_selector_aliases_before_staging() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("test.ts");
    fs::write(&file, "const value = 1;\n").expect("write fixture");

    replace_cmd(&temp)
        .args([
            "ast",
            "const $A = $B;",
            "let $A = $B;",
            "--selector",
            "lexical_declaration",
            "--kind",
            "identifier",
            "test.ts",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("conflicts"));

    assert!(!temp.path().join(".replace/stage").exists());
}

#[test]
fn rule_flag_rejects_conflicting_language() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("test.ts");
    fs::write(&file, "var value = 1;\n").expect("write fixture");
    let rule_file = temp.path().join("rule.yml");
    fs::write(
        &rule_file,
        "id: replace-var\nlanguage: TypeScript\nrule:\n  pattern: var $A = $B;\nfix: let $A = $B;\n",
    )
    .expect("write rule");

    replace_cmd(&temp)
        .args([
            "ast",
            "--rule",
            rule_file.to_str().expect("utf8 path"),
            "--lang",
            "go",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("conflicts"));

    assert!(!temp.path().join(".replace/stage").exists());
}

#[test]
fn rule_flag_rejects_invalid_yaml() {
    let temp = TempDir::new().expect("temp dir");

    // Create an invalid YAML rule file
    let rule_file = temp.path().join("invalid.yml");
    fs::write(&rule_file, "not: [valid yaml").expect("write invalid rule");

    // Try to stage a rewrite using the invalid rule file
    let output = replace_cmd(&temp)
        .args(["ast", "--rule", rule_file.to_str().unwrap(), "--stage"])
        .output()
        .expect("replace ast");

    assert!(
        !output.status.success(),
        "stage should fail with invalid YAML"
    );
}
