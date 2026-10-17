//! Shared harness: every `replace` call names its config file explicitly with `--config-path`,
//! so no developer's global or project config can change a result.
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use assert_fs::TempDir;

fn empty_config_file() -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"));
    fs::create_dir_all(dir).expect("create target tmp dir");
    let file = dir.join("empty-config.toml");
    fs::write(&file, "").expect("write empty config");
    file
}

/// A `replace` invocation rooted in `workdir` that reads exactly `config_file`.
pub fn replace_with_config(workdir: &Path, config_file: &Path) -> Command {
    let mut command = Command::cargo_bin("replace").expect("binary exists");
    command
        .current_dir(workdir)
        .arg("--config-path")
        .arg(config_file);
    command
}

/// A `replace` invocation rooted in `workdir` with an empty config.
pub fn replace_in(workdir: &Path) -> Command {
    replace_with_config(workdir, &empty_config_file())
}

/// [`replace_in`] for an `assert_fs` workspace.
pub fn replace_cmd(workspace: &TempDir) -> Command {
    replace_in(workspace.path())
}

/// Replacement count that `replace --json ast <pattern> <rewrite> <file>` previews for `file`.
pub fn ast_preview_replacements(workdir: &Path, pattern: &str, rewrite: &str, file: &Path) -> u64 {
    let output = replace_in(workdir)
        .args(["--json", "ast", pattern, rewrite])
        .arg(file)
        .output()
        .expect("ast preview runs");
    assert!(
        output.status.success(),
        "ast preview of {pattern:?} failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    json["stages"][0]["replacements"]
        .as_u64()
        .expect("replacement count")
}

/// A staged change as `--json` reports it.
pub struct Staged {
    pub id: String,
    pub anchors: Vec<String>,
}

/// Stage `foo` -> `bar` in `file` and return what `replace` recorded.
pub fn stage_foo_to_bar(workspace: &TempDir, file: &str) -> Staged {
    let output = replace_cmd(workspace)
        .args(["--json", "literal", "foo", "bar", "--stage", file])
        .output()
        .expect("stage runs");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    let record = &json["stages"][0];
    Staged {
        id: record["id"].as_str().expect("id").to_owned(),
        anchors: record["anchors"]
            .as_array()
            .expect("anchors")
            .iter()
            .map(|anchor| anchor.as_str().expect("anchor").to_owned())
            .collect(),
    }
}

pub fn stage_id(output: &str) -> &str {
    output
        .lines()
        .find_map(|line| line.strip_prefix("Staged: "))
        .and_then(|rest| rest.split_whitespace().next())
        .expect("stage id in output")
}

pub fn make_stage_old(temp: &TempDir, id: &str) {
    let path = temp.path().join(format!(".replace/stage/{id}.json"));
    let mut json = fs::read_to_string(&path).expect("read stage json");
    let start = json.find("\"created_at\":").expect("created_at field");
    let value_start = start + "\"created_at\":".len();
    let value_end = value_start + json[value_start..].find(']').expect("created_at array") + 1;
    json.replace_range(value_start..value_end, "[2020,1,0,0,0,0,0,0,0]");
    fs::write(path, json).expect("write stage json");
}
