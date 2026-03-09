//! Copyright detection module.
//!
//! Detects copyright statements, holder names, and author information
//! from source code files using a four-stage pipeline:
//! 1. Text preparation (normalization)
//! 2. Candidate line selection
//! 3. Lexing (POS tagging) and parsing (grammar rules)
//! 4. Refinement and junk filtering

use std::time::Duration;

mod candidates;
mod credits;
mod detector;
mod detector_input_normalization;
pub mod golden_utils;
mod grammar;
mod hints;
mod lexer;
mod line_tracking;
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

#[derive(Debug, Clone)]
pub struct CopyrightDetectionOptions {
    pub include_copyrights: bool,
    pub include_holders: bool,
    pub include_authors: bool,
    pub max_runtime: Option<Duration>,
}

impl Default for CopyrightDetectionOptions {
    fn default() -> Self {
        Self {
            include_copyrights: true,
            include_holders: true,
            include_authors: true,
            max_runtime: None,
        }
    }
}

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
    detect_copyrights_with_options(content, &CopyrightDetectionOptions::default())
}

pub fn detect_copyrights_with_options(
    content: &str,
    options: &CopyrightDetectionOptions,
) -> (
    Vec<CopyrightDetection>,
    Vec<HolderDetection>,
    Vec<AuthorDetection>,
) {
    let (mut copyrights, mut holders, mut authors) = if let Some(max_runtime) = options.max_runtime
    {
        detector::detect_copyrights_from_text_with_deadline(content, Some(max_runtime))
    } else {
        detector::detect_copyrights_from_text(content)
    };

    if !options.include_copyrights {
        copyrights.clear();
    }
    if !options.include_holders {
        holders.clear();
    }
    if !options.include_authors {
        authors.clear();
    }

    (copyrights, holders, authors)
}

#[cfg(test)]
mod tests {
    use super::{CopyrightDetectionOptions, detect_copyrights_with_options};

    #[test]
    fn test_options_can_disable_all_outputs() {
        let content = "Copyright (c) 2024 Acme Inc.\nWritten by John Doe";
        let options = CopyrightDetectionOptions {
            include_copyrights: false,
            include_holders: false,
            include_authors: false,
            ..CopyrightDetectionOptions::default()
        };

        let (copyrights, holders, authors) = detect_copyrights_with_options(content, &options);
        assert!(copyrights.is_empty());
        assert!(holders.is_empty());
        assert!(authors.is_empty());
    }

    #[test]
    fn test_options_can_keep_only_authors() {
        let content = "Written by John Doe";
        let options = CopyrightDetectionOptions {
            include_copyrights: false,
            include_holders: false,
            include_authors: true,
            ..CopyrightDetectionOptions::default()
        };

        let (copyrights, holders, authors) = detect_copyrights_with_options(content, &options);
        assert!(copyrights.is_empty());
        assert!(holders.is_empty());
        assert!(!authors.is_empty());
    }
}
