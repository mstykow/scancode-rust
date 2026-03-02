//! Core data structures for license detection.

pub mod license;
pub mod license_match;
pub mod rule;

pub use license::License;
pub use license_match::LicenseMatch;
pub use rule::Rule;

#[cfg(test)]
mod mod_tests;
