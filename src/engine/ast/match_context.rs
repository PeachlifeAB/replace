//! What a stage records about the first match of an AST rewrite: its captures and enclosing scopes.
use ast_grep_core::meta_var::MetaVariable;
use ast_grep_core::{Doc, NodeMatch};
use serde_json::{Map, Value};

const METAVARIABLE_PREFIX: &str = "$";
const NAME_FIELD: &str = "name";
const FUNCTION_KIND_HINTS: [&str; 2] = ["function", "method"];
const CLASS_KIND_HINTS: [&str; 1] = ["class"];

/// Captured metavariables and the named function and class that contain a match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchContext {
    pub captures: Option<Value>,
    pub parent_function: Option<String>,
    pub parent_class: Option<String>,
}

pub(super) fn describe<D: Doc>(found: &NodeMatch<'_, D>) -> MatchContext {
    MatchContext {
        captures: captures_of(found),
        parent_function: enclosing_name(found, &FUNCTION_KIND_HINTS),
        parent_class: enclosing_name(found, &CLASS_KIND_HINTS),
    }
}

fn captures_of<D: Doc>(found: &NodeMatch<'_, D>) -> Option<Value> {
    let env = found.get_env();
    let captures: Map<String, Value> = env
        .get_matched_variables()
        .filter_map(|variable| match variable {
            MetaVariable::Capture(name, _) => env.get_match(&name).map(|node| {
                (
                    format!("{METAVARIABLE_PREFIX}{name}"),
                    Value::String(node.text().into_owned()),
                )
            }),
            _ => None,
        })
        .collect();
    (!captures.is_empty()).then_some(Value::Object(captures))
}

/// Name of the nearest enclosing node whose kind mentions one of `kind_hints`.
fn enclosing_name<D: Doc>(found: &NodeMatch<'_, D>, kind_hints: &[&str]) -> Option<String> {
    found.ancestors().find_map(|ancestor| {
        let kind = ancestor.kind();
        if !kind_hints.iter().any(|hint| kind.contains(hint)) {
            return None;
        }
        ancestor
            .field(NAME_FIELD)
            .map(|name| name.text().into_owned())
    })
}
