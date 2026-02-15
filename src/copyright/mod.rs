//! Copyright detection module.
//!
//! Detects copyright statements, holder names, and author information
//! from source code files using a four-stage pipeline:
//! 1. Text preparation (normalization)
//! 2. Candidate line selection
//! 3. Lexing (POS tagging) and parsing (grammar rules)
//! 4. Refinement and junk filtering

mod candidates;
mod credits;
mod detector;
mod grammar;
mod hints;
mod lexer;
mod parser;
mod patterns;
mod prepare;
mod refiner;
mod types;

#[cfg(all(test, feature = "golden-tests"))]
mod golden_test;

pub use candidates::strip_balanced_edge_parens;
pub use credits::{detect_credits_authors, is_credits_file};
pub use types::{AuthorDetection, CopyrightDetection, HolderDetection};

/// Detect copyrights, holders, and authors in the given text content.
///
/// Returns a tuple of (copyrights, holders, authors).
pub fn detect_copyrights(
    content: &str,
) -> (
    Vec<CopyrightDetection>,
    Vec<HolderDetection>,
    Vec<AuthorDetection>,
) {
    detector::detect_copyrights_from_text(content)
}
