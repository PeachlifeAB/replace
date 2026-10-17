//! Mixed-language tree inference for injected regions.
//!
//! This module detects injected language regions in host documents (e.g., JavaScript
//! inside `<script>` blocks in HTML, CSS inside `<style>` blocks) and applies AST
//! rewrites only to those regions using the appropriate language grammar.
//!
//! # Supported injections
//!
//! - HTML → JavaScript in `<script>` blocks
//! - HTML → TypeScript in `<script lang="ts">` blocks
//! - HTML → CSS in `<style>` blocks
//! - HTML → SCSS in `<style lang="scss">` blocks
//!
//! # Design
//!
//! The module works in three phases:
//! 1. **Detect** — scan the host document for injected region boundaries
//! 2. **Rewrite** — apply the pattern/rewrite to each region using its language
//! 3. **Reconstruct** — merge rewritten regions back into the host document

mod detect;
mod region;
mod replace;
mod rewrite;
mod tag_language;
mod tag_scan;
#[cfg(test)]
mod tests;

pub use detect::detect_injected_regions;
pub use region::InjectedRegion;
pub use replace::replace_injected_regions;
pub use rewrite::rewrite_region;
