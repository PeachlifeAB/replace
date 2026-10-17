use ast_grep_language::SupportLang;

/// Extract the language from a `<script>` tag.
pub(super) fn extract_script_lang(input: &str, tag_start: usize, tag_end: usize) -> SupportLang {
    let tag_content = &input[tag_start..tag_end];

    // Check for lang="ts" or lang="typescript"
    if tag_content.to_lowercase().contains("lang=\"ts\"")
        || tag_content.to_lowercase().contains("lang='ts'")
    {
        return SupportLang::TypeScript;
    }
    if tag_content.to_lowercase().contains("lang=\"typescript\"")
        || tag_content.to_lowercase().contains("lang='typescript'")
    {
        return SupportLang::TypeScript;
    }

    // Default to JavaScript
    SupportLang::JavaScript
}

/// Extract the language from a `<style>` tag.
pub(super) fn extract_style_lang(input: &str, tag_start: usize, tag_end: usize) -> SupportLang {
    let tag_content = &input[tag_start..tag_end];

    // Check for lang="scss"
    if tag_content.to_lowercase().contains("lang=\"scss\"")
        || tag_content.to_lowercase().contains("lang='scss'")
    {
        return SupportLang::Css; // ast-grep uses Css for SCSS too
    }

    // Default to CSS
    SupportLang::Css
}
