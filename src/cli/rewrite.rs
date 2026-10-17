mod ast_rewrite;
mod direct;
mod emit;
mod executor;
mod inputs;
mod targets;
mod text_rewrite;

pub use ast_rewrite::execute_ast;
pub use emit::print_next_steps;
pub use emit::{JsonRecords, RunTotals, print_summary};
pub use executor::RewriteOutcome;
pub use inputs::{AstInputs, PatternSource, resolve_ast_inputs};
pub use text_rewrite::{TextRewrite, execute_literal, execute_regex};
