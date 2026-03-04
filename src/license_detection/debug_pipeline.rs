//! Debug-only wrapper functions for pipeline introspection.
//!
//! These functions are only compiled with the "debug-pipeline" feature and
//! expose internal filter functions for debugging purposes. They are not
//! part of the public API and may change without notice.
//!
//! Usage: `cargo build --features debug-pipeline`

use crate::license_detection::index::LicenseIndex;
use crate::license_detection::match_refine::filter_low_quality::{
    filter_below_rule_minimum_coverage, filter_false_positive_matches,
    filter_invalid_matches_to_single_word_gibberish, filter_matches_missing_required_phrases,
    filter_matches_to_spurious_single_token, filter_short_matches_scattered_on_too_many_lines,
    filter_spurious_matches, filter_too_short_matches,
};
use crate::license_detection::match_refine::{
    filter_contained_matches, filter_overlapping_matches,
};
use crate::license_detection::models::LicenseMatch;
use crate::license_detection::query::Query;

pub fn filter_contained_matches_debug_only(
    matches: &[LicenseMatch],
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    filter_contained_matches(matches)
}

pub fn filter_too_short_matches_debug_only(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    filter_too_short_matches(index, matches)
}

pub fn filter_false_positive_matches_debug_only(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    filter_false_positive_matches(index, matches)
}

pub fn filter_spurious_matches_debug_only(
    matches: &[LicenseMatch],
    query: &Query,
) -> Vec<LicenseMatch> {
    filter_spurious_matches(matches, query)
}

pub fn filter_below_rule_minimum_coverage_debug_only(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    filter_below_rule_minimum_coverage(index, matches)
}

pub fn filter_short_matches_scattered_on_too_many_lines_debug_only(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    filter_short_matches_scattered_on_too_many_lines(index, matches)
}

pub fn filter_matches_missing_required_phrases_debug_only(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
    query: &Query,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    filter_matches_missing_required_phrases(index, matches, query)
}

pub fn filter_matches_to_spurious_single_token_debug_only(
    matches: &[LicenseMatch],
    query: &Query,
    unknown_count: usize,
) -> Vec<LicenseMatch> {
    filter_matches_to_spurious_single_token(matches, query, unknown_count)
}

pub fn filter_invalid_matches_to_single_word_gibberish_debug_only(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
    query: &Query,
) -> Vec<LicenseMatch> {
    filter_invalid_matches_to_single_word_gibberish(index, matches, query)
}

pub fn filter_overlapping_matches_debug_only(
    matches: Vec<LicenseMatch>,
    index: &LicenseIndex,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    filter_overlapping_matches(matches, index)
}
