use ast_grep_core::matcher::Pattern;
use ast_grep_language::SupportLang;

use super::{detect_injected_regions, replace_injected_regions, rewrite_region};

#[test]
fn detect_script_regions_should_find_basic_script() {
    let html = r"<!DOCTYPE html>
<html>
<head><title>Test</title></head>
<body>
<script>
var a = 1;
</script>
</body>
</html>";

    let regions = detect_injected_regions(html).unwrap();
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].language, SupportLang::JavaScript);
    assert!(regions[0].start > 0);
    assert!(regions[0].end > regions[0].start);
}

#[test]
fn detect_script_regions_should_detect_typescript() {
    let html = r#"<!DOCTYPE html>
<html>
<body>
<script lang="ts">
const a: number = 1;
</script>
</body>
</html>"#;

    let regions = detect_injected_regions(html).unwrap();
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].language, SupportLang::TypeScript);
}

#[test]
fn detect_style_regions_should_find_css() {
    let html = r"<!DOCTYPE html>
<html>
<head>
<style>
body { color: red; }
</style>
</head>
<body></body>
</html>";

    let regions = detect_injected_regions(html).unwrap();
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].language, SupportLang::Css);
}

#[test]
fn detect_multiple_regions() {
    let html = r#"<!DOCTYPE html>
<html>
<head>
<style>
h1 { color: blue; }
</style>
</head>
<body>
<script>
var a = 1;
</script>
<script lang="ts">
const b: number = 2;
</script>
</body>
</html>"#;

    let regions = detect_injected_regions(html).unwrap();
    assert_eq!(regions.len(), 3);
    assert_eq!(regions[0].language, SupportLang::Css);
    assert_eq!(regions[1].language, SupportLang::JavaScript);
    assert_eq!(regions[2].language, SupportLang::TypeScript);
}

#[test]
fn rewrite_region_should_apply_pattern() {
    let html = r"<!DOCTYPE html>
<html>
<body>
<script>
var a = 1;
var b = 2;
</script>
</body>
</html>";

    let regions = detect_injected_regions(html).unwrap();
    let pattern = Pattern::try_new("var $A = $B;", SupportLang::JavaScript).unwrap();
    let result = rewrite_region(html, &regions[0], &pattern, "let $A = $B;").unwrap();
    assert!(result.is_some());
    let (rewritten, count) = result.unwrap();
    assert_eq!(count, 2);
    assert!(rewritten.contains("let a = 1;"));
    assert!(rewritten.contains("let b = 2;"));
}

#[test]
fn replace_injected_regions_should_rewrite_matching_regions() {
    let html = r"<!DOCTYPE html>
<html>
<body>
<style>
h1 { color: blue; }
</style>
<script>
var a = 1;
var b = 2;
</script>
</body>
</html>";

    let result = replace_injected_regions(html, "var $A = $B;", "let $A = $B;").unwrap();
    assert_eq!(result.replacements, 2);
    assert!(result.output.contains("let a = 1;"));
    assert!(result.output.contains("let b = 2;"));
    // Style should be unchanged
    assert!(result.output.contains("h1 { color: blue; }"));
}

#[test]
fn replace_injected_regions_should_return_unchanged_if_no_matches() {
    let html = r"<!DOCTYPE html>
<html>
<body>
<script>
const a = 1;
</script>
</body>
</html>";

    let result = replace_injected_regions(html, "var $A = $B;", "let $A = $B;").unwrap();
    assert_eq!(result.replacements, 0);
    assert_eq!(result.output, html);
}
