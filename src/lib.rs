//! IntentLayer — a prompt-only compiler for coding agents.
//!
//! Transforms messy user prompts into compact, context-preserving,
//! execution-grade prompts for downstream coding agents.

pub mod classifier;
pub mod compiler;
pub mod guard;
pub mod rules;

pub use compiler::{compile, CompileInput, CompileOutput, Compiler};

use std::path::Path;

/// Build a [`Compiler`] loaded with rules from `research/transformation_rules.json`.
///
/// The path is relative to the crate root or an absolute path.
pub fn from_rules_file(path: &Path) -> Result<Compiler, String> {
    let rules = rules::RuleSet::load(path)?;
    Ok(Compiler::new(rules))
}

/// Convenience: build a [`Compiler`] using the default rules file shipped with
/// the repo (`research/transformation_rules.json`).
pub fn default_compiler() -> Result<Compiler, String> {
    let candidates = [
        Path::new("research/transformation_rules.json"),
        Path::new("../research/transformation_rules.json"),
        Path::new("../../research/transformation_rules.json"),
    ];
    for p in &candidates {
        if p.exists() {
            return from_rules_file(p);
        }
    }
    Err("Could not locate research/transformation_rules.json".into())
}
