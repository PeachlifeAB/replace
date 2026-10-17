//! Rewrites inside host documents (HTML, Vue, Svelte, Astro) touch only the injected code.
use std::fs;

use assert_fs::TempDir;
use serde_json::Value;

#[allow(dead_code, reason = "each test crate uses a subset of the harness")]
mod common;
use common::replace_cmd;

const VUE_LITERAL_AND_COMMENT: &str = "<script setup lang=\"ts\">\nconst text = \"foo(name)\"\n// foo(name) - do not rewrite this\nconst value = foo(name)\n</script>\n";

#[test]
#[ignore = "bug: .vue is not an inferred language; 'no language specified for AST rewrite'"]
fn vue_script_block_is_rewritten_without_an_explicit_language() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(
        temp.path().join("Component.vue"),
        "<script setup lang=\"ts\">\nconst value = foo(name)\n</script>\n",
    )
    .expect("fixture");

    replace_cmd(&temp)
        .args(["ast", "foo($A)", "bar($A)", "--apply", "Component.vue"])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(temp.path().join("Component.vue")).expect("read"),
        "<script setup lang=\"ts\">\nconst value = bar(name)\n</script>\n"
    );
}

#[test]
#[ignore = "bug: vue host language unsupported (REPLACE_E_AST unsupported language: vue)"]
fn vue_rewrite_leaves_string_literals_alone() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("Component.vue"), VUE_LITERAL_AND_COMMENT).expect("fixture");

    replace_cmd(&temp)
        .args([
            "ast",
            "--lang",
            "vue",
            "foo($A)",
            "bar($A)",
            "--apply",
            "Component.vue",
        ])
        .assert()
        .success();

    let content = fs::read_to_string(temp.path().join("Component.vue")).expect("read");
    assert!(content.contains("const text = \"foo(name)\""));
    assert!(content.contains("const value = bar(name)"));
}

#[test]
#[ignore = "bug: vue host language unsupported (REPLACE_E_AST unsupported language: vue)"]
fn vue_rewrite_leaves_comments_alone() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join("Component.vue"), VUE_LITERAL_AND_COMMENT).expect("fixture");

    replace_cmd(&temp)
        .args([
            "ast",
            "--lang",
            "vue",
            "foo($A)",
            "bar($A)",
            "--apply",
            "Component.vue",
        ])
        .assert()
        .success();

    let content = fs::read_to_string(temp.path().join("Component.vue")).expect("read");
    assert!(content.contains("// foo(name) - do not rewrite this"));
}

#[test]
#[ignore = "bug: vue host language unsupported; injected_language never populated"]
fn json_reports_typescript_as_the_injected_language_of_a_vue_host() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(
        temp.path().join("Component.vue"),
        "<script setup lang=\"ts\">\nconst value = foo(name)\n</script>\n",
    )
    .expect("fixture");

    let output = replace_cmd(&temp)
        .args([
            "--json",
            "ast",
            "--lang",
            "vue",
            "foo($A)",
            "bar($A)",
            "Component.vue",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).expect("json");
    let stage = &json["stages"][0];

    assert_eq!(stage["language"], "vue");
    assert_eq!(stage["injected_language"], "typescript");
}

#[test]
#[ignore = "bug: HTML <script> regions are not rewritten; replace_injected_regions has no caller (src/engine/inject.rs:239)"]
fn html_script_block_javascript_is_rewritten() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(
        temp.path().join("index.html"),
        "<script>\nfoo(name)\n</script>\n",
    )
    .expect("fixture");

    replace_cmd(&temp)
        .args(["ast", "foo($A)", "bar($A)", "--apply", "index.html"])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(temp.path().join("index.html")).expect("read"),
        "<script>\nbar(name)\n</script>\n"
    );
}

#[test]
#[ignore = "bug: HTML <style> regions are not rewritten; replace_injected_regions has no caller (src/engine/inject.rs:239)"]
fn html_style_block_css_is_rewritten() {
    let temp = TempDir::new().expect("temp dir");
    fs::write(
        temp.path().join("index.html"),
        "<style>\n.old { color: red; }\n</style>\n",
    )
    .expect("fixture");

    replace_cmd(&temp)
        .args([
            "ast",
            ".old { $$$BODY }",
            ".new { $$$BODY }",
            "--apply",
            "index.html",
        ])
        .assert()
        .success();

    assert!(
        fs::read_to_string(temp.path().join("index.html"))
            .expect("read")
            .contains(".new")
    );
}

/// `file` holding `source` after `replace ast --lang LANG PATTERN REWRITE --apply file`.
fn rewritten_host(
    file: &str,
    source: &str,
    language: &str,
    pattern: &str,
    rewrite: &str,
) -> String {
    let temp = TempDir::new().expect("temp dir");
    fs::write(temp.path().join(file), source).expect("fixture");
    replace_cmd(&temp)
        .args(["ast", "--lang", language, pattern, rewrite, "--apply", file])
        .assert()
        .success();
    fs::read_to_string(temp.path().join(file)).expect("read")
}

#[test]
#[ignore = "bug: svelte host language unsupported (REPLACE_E_AST unsupported language: svelte)"]
fn svelte_script_block_is_rewritten() {
    let source = "<script>\nconst value = foo(name)\n</script>\n";

    let written = rewritten_host("App.svelte", source, "svelte", "foo($A)", "bar($A)");

    assert!(written.contains("bar(name)"));
}

#[test]
#[ignore = "bug: astro host language unsupported (REPLACE_E_AST unsupported language: astro)"]
fn astro_frontmatter_is_rewritten() {
    let source = "---\nconst value = foo(name)\n---\n";

    let written = rewritten_host("Page.astro", source, "astro", "foo($A)", "bar($A)");

    assert!(written.contains("bar(name)"));
}

#[test]
#[ignore = "bug: vue host language unsupported; <style lang=\"scss\"> regions not rewritten"]
fn vue_scss_style_block_is_rewritten() {
    let source = "<style lang=\"scss\">\n.old { color: red; }\n</style>\n";

    let written = rewritten_host(
        "Component.vue",
        source,
        "vue",
        ".old { $$$BODY }",
        ".new { $$$BODY }",
    );

    assert!(written.contains(".new"));
}
