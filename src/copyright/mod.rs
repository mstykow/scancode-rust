// SPDX-FileCopyrightText: Provenant contributors
// SPDX-License-Identifier: Apache-2.0

//! Copyright detection module.
//!
//! Detects copyright statements, holder names, and author information
//! from source code files using a four-stage pipeline:
//! 1. Text preparation (normalization)
//! 2. Candidate line selection
//! 3. Lexing (POS tagging) and parsing (grammar rules)
//! 4. Refinement and junk filtering

use crate::cancel::CancelToken;
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

pub use credits::{detect_credits_authors, is_credits_file};
pub(crate) use detector::is_pod_author_heading;
pub(crate) use prepare::prepare_text_line;
pub(crate) use refiner::has_copyright_year;
pub(crate) use refiner::looks_like_source_code;
pub(crate) use refiner::refine_author;
pub use refiner::refine_copyright;
pub use types::{AuthorDetection, CopyrightDetection, HolderDetection};

pub fn detect_copyrights(
    content: &str,
    max_runtime: Option<Duration>,
) -> (
    Vec<CopyrightDetection>,
    Vec<HolderDetection>,
    Vec<AuthorDetection>,
) {
    let token = max_runtime.map(CancelToken::with_timeout);
    detect_copyrights_with_cancel(content, token.as_ref())
}

/// Like [`detect_copyrights`], but stops early once `cancel` trips.
pub fn detect_copyrights_with_cancel(
    content: &str,
    cancel: Option<&CancelToken>,
) -> (
    Vec<CopyrightDetection>,
    Vec<HolderDetection>,
    Vec<AuthorDetection>,
) {
    detector::detect_copyrights_from_text_with_cancel(content, cancel)
}
