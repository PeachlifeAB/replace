//! AST rewrites: the `ast` flags become one plan, then each file is rewritten by ast-grep.
use camino::Utf8PathBuf;

use super::super::commands::AstFlags;
use super::super::util::{infer_ast_language, parse_ast_language};
use super::super::{AstStageOptions, RewriteOptions};
use super::executor::{FileRewrite, RewriteJob, RewriteOutcome, execute_rewrite};
use super::inputs::AstInputs;
use crate::config::ReplaceConfig;
use crate::engine::ast::{self, AstRule};
use crate::error::ReplaceError;
use crate::stage::StageMode;

/// A parsed ast-grep YAML rule.
struct AstgrepYamlRule {
    source: String,
    language: ast_grep_language::SupportLang,
}

/// Load an ast-grep YAML rule file.
///
/// # Errors
///
/// Returns an error if the file cannot be read or is not valid ast-grep YAML.
fn load_astgrep_yaml_rule(path: &Utf8PathBuf) -> Result<AstgrepYamlRule, ReplaceError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| ReplaceError::InvalidArgs(format!("failed to read rule file {path}: {e}")))?;

    let language = ast::yaml_rule_language(&content)
        .map_err(|error| ReplaceError::InvalidArgs(format!("invalid rule file {path}: {error}")))?;

    Ok(AstgrepYamlRule {
        source: content,
        language,
    })
}

const YAML_RULE_PATTERN_LABEL: &str = "<native ast-grep YAML rule>";
const YAML_RULE_REWRITE_LABEL: &str = "<native ast-grep fix>";

/// Everything `execute_ast` decided from the flags before touching any file.
struct AstPlan {
    pattern: String,
    rewrite: String,
    selector: Option<String>,
    strictness: Option<String>,
    yaml_rule: Option<String>,
    yaml_language: Option<ast_grep_language::SupportLang>,
    rule_path: Option<Utf8PathBuf>,
}

/// Validate the AST flags and turn them into a plan.
fn plan_ast_rewrite(flags: &AstFlags, inputs: &AstInputs) -> Result<AstPlan, ReplaceError> {
    if !flags.sg_args.is_empty() {
        return Err(ReplaceError::InvalidArgs(
            "--sg-arg is unsupported by embedded ast-grep; use --selector, --kind, or --strictness"
                .to_owned(),
        ));
    }
    let selector = merge_selector_aliases(flags.selector.as_deref(), flags.kind.as_deref())?;
    let strictness = flags.strictness.clone();

    match (&flags.rule, &inputs.pattern, &inputs.rewrite) {
        (Some(rule_file), _, _) => plan_from_rule_file(flags, rule_file, selector, strictness),
        (None, Some(pattern), Some(rewrite)) => Ok(AstPlan {
            pattern: pattern.clone(),
            rewrite: rewrite.clone(),
            selector,
            strictness,
            yaml_rule: None,
            yaml_language: None,
            rule_path: None,
        }),
        _ => Err(ReplaceError::InvalidArgs(
            "either PATTERN and REWRITE or --rule must be provided".to_owned(),
        )),
    }
}

fn merge_selector_aliases(
    selector: Option<&str>,
    kind: Option<&str>,
) -> Result<Option<String>, ReplaceError> {
    match (selector, kind) {
        (Some(selector), Some(kind)) if selector != kind => {
            Err(ReplaceError::InvalidArgs(format!(
                "--selector ({selector}) conflicts with --kind ({kind}); supply one value or matching aliases"
            )))
        }
        (Some(value), _) | (_, Some(value)) => Ok(Some(value.to_owned())),
        (None, None) => Ok(None),
    }
}

fn plan_from_rule_file(
    flags: &AstFlags,
    rule_file: &Utf8PathBuf,
    selector: Option<String>,
    strictness: Option<String>,
) -> Result<AstPlan, ReplaceError> {
    if selector.is_some() || strictness.is_some() {
        return Err(ReplaceError::InvalidArgs(
            "--selector, --kind, and --strictness are defined by --rule YAML; remove the CLI flags"
                .to_owned(),
        ));
    }
    let rule = load_astgrep_yaml_rule(rule_file)?;
    if let Some(lang) = &flags.lang
        && parse_ast_language(lang)? != rule.language
    {
        return Err(ReplaceError::InvalidArgs(format!(
            "--lang ({lang}) conflicts with the language declared by --rule"
        )));
    }
    let rule_path = rule_file.canonicalize_utf8().map_err(|error| {
        ReplaceError::InvalidArgs(format!("failed to resolve rule file {rule_file}: {error}"))
    })?;
    Ok(AstPlan {
        pattern: YAML_RULE_PATTERN_LABEL.to_owned(),
        rewrite: YAML_RULE_REWRITE_LABEL.to_owned(),
        selector,
        strictness,
        yaml_rule: Some(rule.source),
        yaml_language: Some(rule.language),
        rule_path: Some(rule_path),
    })
}

/// The parser language for a file, and its name when it had to be inferred.
struct TargetLanguage {
    language: ast_grep_language::SupportLang,
    inferred_name: Option<String>,
}

fn resolve_language(
    plan: &AstPlan,
    flags: &AstFlags,
    config: &ReplaceConfig,
    path: &Utf8PathBuf,
) -> Result<TargetLanguage, ReplaceError> {
    if let Some(language) = plan.yaml_language {
        return Ok(TargetLanguage {
            language,
            inferred_name: None,
        });
    }
    let name = flags
        .lang
        .clone()
        .or_else(|| infer_ast_language(path, config))
        .ok_or_else(|| {
            ReplaceError::InvalidArgs(format!("no language specified for AST rewrite: {path}"))
        })?;
    Ok(TargetLanguage {
        language: parse_ast_language(&name)?,
        inferred_name: Some(name),
    })
}

/// Rewrite one file's text according to the plan.
fn rewrite_ast_file(
    plan: &AstPlan,
    flags: &AstFlags,
    config: &ReplaceConfig,
    path: &Utf8PathBuf,
    original: &str,
) -> Result<FileRewrite, ReplaceError> {
    let is_rule_file = plan
        .rule_path
        .as_ref()
        .zip(path.canonicalize_utf8().ok())
        .is_some_and(|(rule_path, target_path)| *rule_path == target_path);
    if is_rule_file {
        return Ok(FileRewrite {
            output: original.to_owned(),
            replacements: 0,
            language: None,
            first_match: None,
        });
    }

    let target = resolve_language(plan, flags, config, path)?;

    let rule = AstRule {
        language: target.language,
        pattern: plan.pattern.clone(),
        replacement: plan.rewrite.clone(),
        selector: plan.selector.clone(),
        strictness: plan.strictness.clone(),
        yaml_rule: plan.yaml_rule.clone(),
    };
    let rewritten = ast::rewrite_with_context(original, &rule)?;
    let result = rewritten.result;

    // When requested, refuse rewrites whose output still matches the
    // pattern: a cascading (potentially infinite) rewrite.
    if flags.check_after && ast::replace_all(&result.output, &rule)?.replacements > 0 {
        return Err(ReplaceError::InvalidArgs(format!(
            "rewrite still matches after applying (cascading rewrite): {path}"
        )));
    }

    Ok(FileRewrite {
        output: result.output,
        replacements: result.replacements,
        language: target.inferred_name,
        first_match: rewritten.first_match,
    })
}

pub fn execute_ast(
    config: &ReplaceConfig,
    flags: &AstFlags,
    inputs: &AstInputs,
    options: RewriteOptions,
) -> Result<RewriteOutcome, ReplaceError> {
    let plan = plan_ast_rewrite(flags, inputs)?;
    let job = RewriteJob {
        ignore_case: false,
        mode: StageMode::Ast,
        pattern: &plan.pattern,
        rewrite: &plan.rewrite,
        paths: &inputs.paths,
    };
    let options = RewriteOptions {
        ast_options: Some(AstStageOptions {
            selector: plan.selector.clone(),
            strictness: plan.strictness.clone(),
            rule_yaml: plan.yaml_rule.clone(),
        }),
        ..options
    };
    execute_rewrite(&job, options, |path, original| {
        rewrite_ast_file(&plan, flags, config, path, original)
    })
}
