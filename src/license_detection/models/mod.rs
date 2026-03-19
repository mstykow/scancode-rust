//! Core data structures for license detection.

pub mod license;
pub mod license_match;
pub mod rule;

pub use license::License;
pub use license_match::{LicenseMatch, MatcherKind};
pub use rule::{Rule, RuleKind};

#[cfg(test)]
mod mod_tests;
