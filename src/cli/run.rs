use camino::Utf8PathBuf;

use super::commands::AstFlags;
use super::hooks::{HookContext, run_hooks};
use super::rewrite::{
    AstInputs, RewriteOutcome, TextRewrite, execute_ast, execute_literal, execute_regex,
};
use super::util::write_stdout_line;
use super::{OutputOptions, RewriteOptions, StagePolicy};
use crate::config::{ReplaceConfig, Rule};
use crate::error::ReplaceError;

pub fn execute_run(
    config: &ReplaceConfig,
    output: OutputOptions,
    walk_options: &crate::walk::WalkOptions,
    strict: bool,
    rename_paths: bool,
) -> Result<(), ReplaceError> {
    if config.rules.is_empty() {
        return Err(ReplaceError::InvalidArgs(
            "no rules configured: add [[rules]] to .replace/config.toml or pass --config-path"
                .to_owned(),
        ));
    }

    let repo_root = config.root.as_ref().map(std::string::ToString::to_string);
    if config.apply_immediately {
        run_hooks(
            &config.hooks.pre_apply,
            &HookContext::pre(None, None, vec![], repo_root.clone()),
        )?;
    }

    let rules_result = run_rules(config, output, walk_options, rename_paths);

    if config.apply_immediately {
        let changed = rules_result.as_ref().map(|counts| &counts.changed).ok();
        run_hooks(
            &config.hooks.post_apply,
            &HookContext::post(
                changed.map(stage_ids_for_hook),
                None,
                changed.map(files_for_hook).unwrap_or_default(),
                repo_root,
            ),
        )?;
    }

    let counts = rules_result?;
    let warned = counts.changed.fail_if_strict(strict);
    print_run_summary(config, &counts)?;
    warned
}

fn stage_ids_for_hook(changed: &RewriteOutcome) -> String {
    changed.stage_ids.join(",")
}

fn files_for_hook(changed: &RewriteOutcome) -> Vec<String> {
    let mut files: Vec<String> = changed.files.iter().map(ToString::to_string).collect();
    files.dedup();
    files
}

/// How many rules staged something, and how many of those were applied right away.
struct RunCounts {
    staged: usize,
    applied: usize,
    changed: RewriteOutcome,
}

fn run_rules(
    config: &ReplaceConfig,
    output: OutputOptions,
    walk_options: &crate::walk::WalkOptions,
    rename_paths: bool,
) -> Result<RunCounts, ReplaceError> {
    let root = config
        .root
        .as_deref()
        .map_or_else(|| Utf8PathBuf::from("."), Utf8PathBuf::from);
    let paths = [root];
    let options = RewriteOptions {
        stage_policy: if config.apply_immediately {
            StagePolicy::Apply
        } else {
            StagePolicy::Stage
        },
        rename_files: rename_paths,
        output,
        walk_options: walk_options.clone(),
        backup: false,
        ast_options: None,
    };

    let mut counts = RunCounts {
        staged: 0,
        applied: 0,
        changed: RewriteOutcome::default(),
    };
    for rule in &config.rules {
        let outcome = run_rule(config, rule, &paths, options.clone())?;
        if !outcome.is_empty() {
            counts.staged += 1;
            if config.apply_immediately {
                counts.applied += 1;
            }
        }
        counts.changed.merge(outcome);
    }
    Ok(counts)
}

/// Run one rule; rules with an empty pattern are skipped.
fn run_rule(
    config: &ReplaceConfig,
    rule: &Rule,
    paths: &[Utf8PathBuf],
    options: RewriteOptions,
) -> Result<RewriteOutcome, ReplaceError> {
    match rule {
        Rule::Literal { find, replace } if !find.is_empty() => execute_literal(
            &TextRewrite {
                ignore_case: false,
                pattern: find,
                replacement: replace,
                paths,
            },
            options,
        ),
        Rule::Regex {
            pattern,
            replacement,
        } if !pattern.is_empty() => execute_regex(
            &TextRewrite {
                ignore_case: false,
                pattern,
                replacement,
                paths,
            },
            options,
        ),
        Rule::Ast {
            language,
            pattern,
            replacement,
        } if !pattern.is_empty() => {
            let flags = AstFlags {
                lang: language.clone(),
                ..AstFlags::default()
            };
            let inputs = AstInputs {
                pattern: Some(pattern.clone()),
                rewrite: Some(replacement.clone()),
                paths: paths.to_vec(),
            };
            let options = RewriteOptions {
                rename_files: false,
                ..options
            };
            execute_ast(config, &flags, &inputs, options)
        }
        _ => Ok(RewriteOutcome::default()),
    }
}

fn print_run_summary(config: &ReplaceConfig, counts: &RunCounts) -> Result<(), ReplaceError> {
    if counts.staged == 0 {
        write_stdout_line(format_args!("No matches found"))
    } else {
        write_stdout_line(format_args!(
            "Run complete: {} rules processed, {} staged, {} applied",
            config.rules.len(),
            counts.staged,
            counts.applied
        ))
    }
}
