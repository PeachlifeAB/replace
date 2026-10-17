//! Staging that creates candidates from diagnostics, and the output formats staging prints.
use assert_fs::TempDir;
use std::fs;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::{replace_cmd, stage_id};

#[test]
fn from_error_stages_candidate_from_diagnostic() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("sample.ts");
    fs::write(&file, "const x = 1;\n").expect("write fixture");

    let diagnostic = format!(
        "{}/sample.ts:5:10: error TS2322: Type 'string' is not assignable to type 'number'",
        temp.path().display()
    );

    let output = replace_cmd(&temp)
        .args(["from-error", &diagnostic])
        .assert()
        .success()
        .stdout(predicates::str::contains("Staged:"))
        .stdout(predicates::str::contains("Confidence: low"))
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).expect("utf8 stdout");
    let id = stage_id(&output).to_owned();

    replace_cmd(&temp)
        .args(["explain", &id])
        .assert()
        .success()
        .stdout(predicates::str::contains("Mode: FromError"));
}

#[test]
fn literal_json_outputs_machine_readable_summary() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("sample.txt");
    fs::write(&file, "omega beta\n").expect("write fixture");

    replace_cmd(&temp)
        .args(["--json", "literal", "omega", "alpha", "sample.txt"])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"stages\""))
        .stdout(predicates::str::contains("\"replacements\":1"));
}

#[test]
fn literal_stats_outputs_summary() {
    let temp = TempDir::new().expect("temp dir");
    let file = temp.path().join("sample.txt");
    fs::write(&file, "omega beta omega\n").expect("write fixture");

    replace_cmd(&temp)
        .args(["--stats", "literal", "omega", "alpha", "sample.txt"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Stats: 1 files, 2 replacements"));
}

#[test]
fn staging_output_includes_advisory_lines() {
    let temp = TempDir::new().expect("temp dir");

    // Create a file
    let file = temp.path().join("test.txt");
    fs::write(&file, "hello world\n").expect("write fixture");

    // Stage a rewrite
    let output = replace_cmd(&temp)
        .args(["literal", "hello", "goodbye", "--stage"])
        .output()
        .expect("replace stage");

    assert!(
        output.status.success(),
        "stage should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify advisory lines are present
    assert!(stdout.contains("Staged:"), "should contain 'Staged:'");
    assert!(
        stdout.contains("Next: replace show"),
        "should contain 'Next: replace show'"
    );
    assert!(
        stdout.contains("replace apply"),
        "should contain 'replace apply'"
    );
    assert!(
        stdout.contains("replace discard"),
        "should contain 'replace discard'"
    );

    // Extract stage ID
    let stage_id = stdout
        .lines()
        .find_map(|line| line.strip_prefix("Staged: "))
        .and_then(|rest| rest.split_whitespace().next())
        .expect("stage id in output")
        .to_owned();

    // Verify advisory lines contain the stage ID
    for line in stdout.lines() {
        if line.starts_with("Next:") || line.starts_with("      replace") {
            assert!(
                line.contains(&stage_id),
                "advisory line should contain stage ID: {line}"
            );
        }
    }
}
