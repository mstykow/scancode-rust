//! Grammar facade for copyright parse tree construction.
//!
//! Types and rule data are split into dedicated submodules to keep this module
//! small and focused while preserving existing import paths.

#[path = "grammar_rules.rs"]
mod grammar_rules;
#[path = "grammar_types.rs"]
mod grammar_types;

pub(crate) use grammar_rules::GRAMMAR_RULES;
pub(crate) use grammar_types::{GrammarRule, TagMatcher};

#[cfg(test)]
#[path = "grammar_test.rs"]
mod tests;
