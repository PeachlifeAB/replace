//! The whole CLI, driven end to end over the dogfood fixtures.
use assert_fs::TempDir;
use std::path::Path;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
#[path = "dogfood_lifecycle/fixtures.rs"]
mod fixtures;
#[path = "dogfood_lifecycle/rewrite_modes.rs"]
mod rewrite_modes;
#[path = "dogfood_lifecycle/smoke_and_literal.rs"]
mod smoke_and_literal;

use fixtures::{cleanup_and_assert, setup_fixtures};
use rewrite_modes::{
    run_ast_discard, run_config_apply_log, run_from_error_diagnostic, run_regex_apply_undo,
};
use smoke_and_literal::{
    run_dot_literal_shorthand, run_literal_lifecycle, run_recursive_apply_all, run_replace_files,
    run_smoke_commands,
};

#[test]
fn dogfood_executable_covers_public_commands() {
    let temp = TempDir::new().expect("temp dir");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/dogfood");
    setup_fixtures(&temp, &fixture);
    run_smoke_commands(&temp);
    run_literal_lifecycle(&temp);
    run_dot_literal_shorthand(&temp);
    run_recursive_apply_all(&temp);
    run_replace_files(&temp);
    run_regex_apply_undo(&temp);
    run_ast_discard(&temp);
    run_from_error_diagnostic(&temp);
    run_config_apply_log(&temp);
    cleanup_and_assert(&temp, &fixture);
}
