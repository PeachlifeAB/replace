use std::fs;
use std::path::{Path, PathBuf};

use assert_fs::TempDir;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_in as replace_cmd;

const FIXTURES: &str = "tests/fixtures/synthetic";
const LANGUAGE_EXTENSIONS: [&str; 6] = ["go", "ts", "js", "py", "rs", "rb"];

fn copy_into(temp: &TempDir, fixture: &Path) -> PathBuf {
    let target = temp.path().join(fixture.file_name().expect("fixture name"));
    fs::copy(fixture, &target).expect("copy fixture");
    target
}

fn language_fixtures() -> Vec<PathBuf> {
    fs::read_dir(Path::new(FIXTURES).join("languages"))
        .expect("languages dir")
        .map(|entry| entry.expect("dir entry").path())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| LANGUAGE_EXTENSIONS.contains(&extension))
        })
        .collect()
}

#[test]
fn identity_rewrite_leaves_every_language_fixture_byte_identical() {
    let fixtures = language_fixtures();
    assert!(!fixtures.is_empty());
    for fixture in fixtures {
        let temp = TempDir::new().expect("temp dir");
        let copy = copy_into(&temp, &fixture);
        replace_cmd(temp.path())
            .args(["ast", "$X", "$X", "--apply"])
            .arg(&copy)
            .assert()
            .success();
        assert_eq!(
            fs::read(&copy).expect("read copy"),
            fs::read(&fixture).expect("read fixture"),
            "identity rewrite changed {}",
            fixture.display()
        );
    }
}

const THREE_PRINTS: &str =
    "fn main() {\n    println!(\"one\");\n    println!(\"two\");\n    println!(\"three\");\n}\n";

#[test]
fn opposite_rewrites_restore_the_original_source() {
    let temp = TempDir::new().expect("temp dir");
    let copy = temp.path().join("roundtrip.rs");
    fs::write(&copy, THREE_PRINTS).expect("write fixture");

    for (from, to) in [
        ("println!($A)", "eprintln!($A)"),
        ("eprintln!($A)", "println!($A)"),
    ] {
        replace_cmd(temp.path())
            .args(["ast", from, to, "--apply"])
            .arg(&copy)
            .assert()
            .success();
        if to.starts_with("eprintln") {
            let rewritten = fs::read_to_string(&copy).expect("read copy");
            assert_eq!(rewritten.matches("eprintln!").count(), 3);
        }
    }

    assert_eq!(fs::read_to_string(&copy).expect("read copy"), THREE_PRINTS);
}

#[test]
fn preview_replacement_count_equals_applied_count() {
    let temp = TempDir::new().expect("temp dir");
    let copy = temp.path().join("three_prints.rs");
    fs::write(&copy, THREE_PRINTS).expect("write fixture");

    let preview = replace_cmd(temp.path())
        .args(["--json", "ast", "println!($A)", "eprintln!($A)"])
        .arg(&copy)
        .output()
        .expect("preview runs");
    let preview: serde_json::Value = serde_json::from_slice(&preview.stdout).expect("json");
    let stage = &preview["stages"][0];
    let staged_count = stage["replacements"].as_u64().expect("replacement count");
    let stage_id = stage["id"].as_str().expect("stage id");
    assert_eq!(staged_count, 3);

    let applied = replace_cmd(temp.path())
        .args(["--json", "apply", stage_id])
        .output()
        .expect("apply runs");
    assert!(applied.status.success());
    let applied = String::from_utf8_lossy(&applied.stdout);
    assert!(
        applied.contains(&staged_count.to_string()),
        "apply output {applied:?} should report {staged_count} replacements"
    );
}

#[test]
fn bare_go_call_patterns_match_inside_function_bodies() {
    let temp = TempDir::new().expect("temp dir");
    let fixture = Path::new(FIXTURES).join("ast-edge-cases/multiple-matches.go");
    let copy = copy_into(&temp, &fixture);

    replace_cmd(temp.path())
        .args(["ast", "fmt.Println($A)", "log.Println($A)", "--apply"])
        .arg(&copy)
        .assert()
        .success();

    let rewritten = fs::read_to_string(&copy).expect("read copy");
    assert_eq!(rewritten.matches("log.Println").count(), 5);
    assert!(!rewritten.contains("fmt.Println"));
}

#[test]
fn rewrite_producing_identical_text_stages_nothing() {
    let temp = TempDir::new().expect("temp dir");
    let copy = temp.path().join("noop.rs");
    fs::write(&copy, THREE_PRINTS).expect("write fixture");

    replace_cmd(temp.path())
        .args(["ast", "println!($A)", "println!($A)"])
        .arg(&copy)
        .assert()
        .success()
        .stdout(predicates::str::contains("No matches"));

    assert!(!temp.path().join(".replace/stage").exists());
}

#[test]
fn apply_infers_language_from_extension_when_the_stage_recorded_none() {
    let temp = TempDir::new().expect("temp dir");
    let copy = temp.path().join("infer.rs");
    fs::write(&copy, THREE_PRINTS).expect("write fixture");

    let preview = replace_cmd(temp.path())
        .args(["--json", "ast", "println!($A)", "eprintln!($A)"])
        .arg(&copy)
        .output()
        .expect("preview runs");
    let preview: serde_json::Value = serde_json::from_slice(&preview.stdout).expect("json");
    let stage_id = preview["stages"][0]["id"].as_str().expect("stage id");

    let sidecar = temp.path().join(format!(".replace/stage/{stage_id}.json"));
    let mut record: serde_json::Value =
        serde_json::from_slice(&fs::read(&sidecar).expect("read sidecar")).expect("sidecar json");
    record["language"] = serde_json::Value::Null;
    fs::write(&sidecar, serde_json::to_vec(&record).expect("encode")).expect("write sidecar");

    replace_cmd(temp.path())
        .args(["apply", stage_id])
        .assert()
        .success();
    assert_eq!(
        fs::read_to_string(&copy)
            .expect("read")
            .matches("eprintln!")
            .count(),
        3
    );
}
