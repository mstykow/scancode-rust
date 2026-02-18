//! License detection assembly and grouping logic.
//!
//! This module implements Phase 6 of the license detection pipeline:
//! grouping raw matches into LicenseDetection objects based on proximity
//! and applying heuristics.

use crate::license_detection::expression::{CombineRelation, combine_expressions};
use crate::license_detection::models::LicenseMatch;
use crate::license_detection::spdx_mapping::SpdxMapping;

/// Line gap threshold for grouping matches.
/// Matches with line gap > this are considered separate groups.
/// Corresponds to Python's LINES_THRESHOLD = 4 (query.py:108)
const LINES_THRESHOLD: usize = 4;

/// Coverage value below which detections are not perfect.
/// Any value < 100 means detection is imperfect.
const IMPERFECT_MATCH_COVERAGE_THR: f32 = 100.0;

/// Coverage values below this are reported as license clues.
const CLUES_MATCH_COVERAGE_THR: f32 = 60.0;

/// False positive threshold for rule length (in tokens).
/// Rules with length <= this are potential false positives.
const FALSE_POSITIVE_RULE_LENGTH_THRESHOLD: usize = 3;

/// False positive threshold for start line.
/// Matches after this line with short rules are potential false positives.
const FALSE_POSITIVE_START_LINE_THRESHOLD: usize = 1000;

// ============================================================================
// Detection Log Categories (Python parity: DetectionRule enum)
// ============================================================================

/// Perfect detection - all matches are exact with 100% coverage.
pub const DETECTION_LOG_PERFECT_DETECTION: &str = "perfect-detection";

/// Possible false positive detection.
pub const DETECTION_LOG_FALSE_POSITIVE: &str = "possible-false-positive";

/// License clues - low quality matches.
pub const DETECTION_LOG_LICENSE_CLUES: &str = "license-clues";

/// Low quality match fragments - similar to license clues but distinct category.
pub const DETECTION_LOG_LOW_QUALITY_MATCHES: &str = "low-quality-matches";

/// Imperfect match coverage - at least one match has coverage < 100%.
pub const DETECTION_LOG_IMPERFECT_COVERAGE: &str = "imperfect-match-coverage";

/// Unknown match - matches with unknown license identifiers.
pub const DETECTION_LOG_UNKNOWN_MATCH: &str = "unknown-match";

/// Extra words - match contains extra text beyond the matched rule.
pub const DETECTION_LOG_EXTRA_WORDS: &str = "extra-words";

/// Undetected license - single undetected match (no license found).
pub const DETECTION_LOG_UNDETECTED_LICENSE: &str = "undetected-license";

/// Unknown intro followed by match - license intro followed by proper detection.
pub const DETECTION_LOG_UNKNOWN_INTRO_FOLLOWED_BY_MATCH: &str = "unknown-intro-followed-by-match";

/// Unknown reference to local file - match references another file (e.g., "see LICENSE").
pub const DETECTION_LOG_UNKNOWN_REFERENCE_TO_LOCAL_FILE: &str = "unknown-reference-to-local-file";

/// A group of license matches that are nearby each other in the file.
#[derive(Debug, Clone)]
pub struct DetectionGroup {
    /// The matches in this group
    pub matches: Vec<LicenseMatch>,
    /// Start line of the group (1-indexed)
    pub start_line: usize,
    /// End line of the group (1-indexed)
    pub end_line: usize,
}

impl DetectionGroup {
    fn new(matches: Vec<LicenseMatch>) -> Self {
        if matches.is_empty() {
            return Self {
                matches,
                start_line: 0,
                end_line: 0,
            };
        }

        let start_line = matches.iter().map(|m| m.start_line).min().unwrap_or(0);
        let end_line = matches.iter().map(|m| m.end_line).max().unwrap_or(0);

        Self {
            matches,
            start_line,
            end_line,
        }
    }
}

/// A LicenseDetection combines one or more LicenseMatch objects using
/// various rules and heuristics.
#[derive(Debug, Clone)]
pub struct LicenseDetection {
    /// A license expression string using SPDX license expression syntax
    /// and ScanCode license keys - the effective license expression for this detection.
    pub license_expression: Option<String>,

    /// SPDX license expression string with SPDX ids only.
    pub license_expression_spdx: Option<String>,

    /// List of license matches combined in this detection.
    pub matches: Vec<LicenseMatch>,

    /// A list of detection log entries explaining how this detection was created.
    pub detection_log: Vec<String>,

    /// An identifier unique for a license detection, containing the license
    /// expression and a UUID crafted from the match contents.
    pub identifier: Option<String>,

    /// File path and start/end lines to locate the detection.
    pub file_region: Option<FileRegion>,
}

/// A file has one or more file-regions, which are separate regions of the file
/// containing some license information.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct FileRegion {
    /// File path
    pub path: String,
    /// Start line number (1-indexed)
    pub start_line: usize,
    /// End line number (1-indexed)
    pub end_line: usize,
}

/// Group matches by file region using proximity threshold.
///
/// This function groups license matches that are near each other in the file
/// (within LINES_THRESHOLD lines) and handles special cases for license intros
/// and license clues.
///
/// # Arguments
///
/// * `matches` - List of license matches to group, should be sorted by start_line
/// * `proximity_threshold` - Maximum line gap between matches to be in the same group (default: LINES_THRESHOLD)
///
/// # Returns
///
/// A vector of DetectionGroup objects, each containing matches that form a region
pub fn group_matches_by_region(matches: &[LicenseMatch]) -> Vec<DetectionGroup> {
    group_matches_by_region_with_threshold(matches, LINES_THRESHOLD)
}

/// Group matches by file region with a custom proximity threshold.
///
/// # Arguments
///
/// * `matches` - List of license matches to group, should be sorted by start_line
/// * `_proximity_threshold` - Maximum line gap between matches to be in the same group (kept for API compatibility, not used)
///
/// # Returns
///
/// A vector of DetectionGroup objects, each containing matches that form a region
fn group_matches_by_region_with_threshold(
    matches: &[LicenseMatch],
    _proximity_threshold: usize,
) -> Vec<DetectionGroup> {
    let mut groups = Vec::new();
    let mut current_group: Vec<LicenseMatch> = Vec::new();

    for match_item in matches {
        if current_group.is_empty() {
            current_group.push(match_item.clone());
            continue;
        }

        let previous_match = current_group.last().unwrap();

        if previous_match.is_license_intro {
            current_group.push(match_item.clone());
        } else if match_item.is_license_intro {
            if !current_group.is_empty() {
                groups.push(DetectionGroup::new(current_group.clone()));
            }
            current_group = vec![match_item.clone()];
        } else if match_item.is_license_clue {
            if !current_group.is_empty() {
                groups.push(DetectionGroup::new(current_group.clone()));
            }
            groups.push(DetectionGroup::new(vec![match_item.clone()]));
            current_group = Vec::new();
        } else if should_group_together(previous_match, match_item) {
            current_group.push(match_item.clone());
        } else {
            if !current_group.is_empty() {
                groups.push(DetectionGroup::new(current_group.clone()));
            }
            current_group = vec![match_item.clone()];
        }
    }

    if !current_group.is_empty() {
        groups.push(DetectionGroup::new(current_group));
    }

    groups
}

/// Check if two matches should be in the same group based on line proximity.
///
/// Matches are grouped together when line gap is within threshold.
///
/// Based on Python's group_matches() at detection.py:1820-1868:
/// ```python
/// is_in_group_by_threshold = license_match.start_line <= previous_match.end_line + lines_threshold
/// ```
///
/// This means: GROUP if start_line <= prev_end_line + 4 (equivalent to line_gap <= 4)
fn should_group_together(prev: &LicenseMatch, cur: &LicenseMatch) -> bool {
    let line_gap = cur.start_line.saturating_sub(prev.end_line);
    line_gap <= LINES_THRESHOLD
}

/// Sort matches by start line for grouping.
pub fn sort_matches_by_line(matches: &mut [LicenseMatch]) {
    matches.sort_by(|a, b| {
        a.start_line
            .cmp(&b.start_line)
            .then_with(|| a.end_line.cmp(&b.end_line))
    });
}

/// Check if matches are correct detection (perfect matches).
///
/// A detection is correct if:
/// - All matchers are "1-hash", "1-spdx-id", or "2-aho" (exact matchers)
/// - All match coverages are 100%
///
/// Based on Python: is_correct_detection() at detection.py:1078
fn is_correct_detection(matches: &[LicenseMatch]) -> bool {
    if matches.is_empty() {
        return false;
    }

    let all_valid_matchers = matches
        .iter()
        .all(|m| m.matcher == "1-hash" || m.matcher == "1-spdx-id" || m.matcher == "2-aho");

    let all_perfect_coverage = matches.iter().all(|m| m.match_coverage >= 100.0 - 0.01);

    all_valid_matchers && all_perfect_coverage
}

/// Check if matches are correct detection (perfect matches).
///
/// A detection is correct if:
/// - All matchers are "1-hash", "1-spdx-id", or "2-aho" (exact matchers)
/// - All match coverages are 100%
///
/// Based on Python: is_correct_detection() at detection.py:1078
#[allow(non_snake_case)]
pub fn is_correctDetection(matches: &[LicenseMatch]) -> bool {
    is_correct_detection(matches)
}

/// Check if match coverage is below threshold.
///
/// Based on Python: is_match_coverage_less_than_threshold() at detection.py:1095
///
/// - If any_matches is True (default), returns True if ANY match has coverage < threshold
/// - If any_matches is False, returns True if NONE of the matches have coverage > threshold
fn is_match_coverage_below_threshold(
    matches: &[LicenseMatch],
    threshold: f32,
    any_matches: bool,
) -> bool {
    if any_matches {
        return matches.iter().any(|m| m.match_coverage < threshold - 0.01);
    }
    !matches.iter().any(|m| m.match_coverage > threshold)
}

/// Check if all matches have unknown license identifiers.
fn has_unknown_matches(matches: &[LicenseMatch]) -> bool {
    matches
        .iter()
        .any(|m| m.rule_identifier.contains("unknown") || m.license_expression.contains("unknown"))
}

/// Check if matches have extra words.
///
/// Extra words are present when score < (coverage * relevance) / 100.
/// Based on Python: calculate_query_coverage_coefficient() at detection.py:1124
/// and has_extra_words() at detection.py:1139
fn has_extra_words(matches: &[LicenseMatch]) -> bool {
    matches.iter().any(|m| {
        let score_coverage_relevance = m.match_coverage * m.rule_relevance as f32 / 100.0;
        score_coverage_relevance - m.score > 0.01
    })
}

/// Check if detection is a false positive.
///
/// False positives are identified based on:
/// - Single matches with bare identifiers and low relevance
/// - GPL matches with short length
/// - Late matches with short rules and low relevance
/// - Tag matches with short length
///
/// Based on Python: is_false_positive() at detection.py:1162
fn is_false_positive(matches: &[LicenseMatch]) -> bool {
    if matches.is_empty() {
        return false;
    }

    // Early return if all matches have full relevance (100)
    let has_full_relevance = matches.iter().all(|m| m.rule_relevance == 100);
    if has_full_relevance {
        return false;
    }

    let start_line = matches.iter().map(|m| m.start_line).min().unwrap_or(0);

    let bare_rules = ["gpl_bare", "freeware_bare", "public-domain_bare"];
    let is_bare_rule = matches.iter().all(|m| {
        bare_rules
            .iter()
            .any(|bare| m.rule_identifier.to_lowercase().contains(bare))
    });

    let is_gpl = matches
        .iter()
        .all(|m| m.rule_identifier.to_lowercase().contains("gpl"));

    // Use rule_length (token count) instead of matched_length (character count)
    let rule_length_values: Vec<usize> = matches.iter().map(|m| m.rule_length).collect();

    let all_rule_length_one = rule_length_values.iter().all(|&l| l == 1);

    let all_low_relevance = matches.iter().all(|m| m.rule_relevance < 60);

    let is_single = matches.len() == 1;

    // Check if all matches are license tags with length == 1
    let all_is_license_tag = matches.iter().all(|m| m.is_license_tag);

    // Check 1: Single bare rule with low relevance
    if is_single && is_bare_rule && all_low_relevance {
        return true;
    }

    // Check 2: GPL with all rules having length == 1 (token count)
    if is_gpl && all_rule_length_one {
        return true;
    }

    // Check 3: Late match with low relevance and any short rule
    if all_low_relevance
        && start_line > FALSE_POSITIVE_START_LINE_THRESHOLD
        && rule_length_values
            .iter()
            .any(|&l| l <= FALSE_POSITIVE_RULE_LENGTH_THRESHOLD)
    {
        return true;
    }

    // Check 4: License tag matches with length == 1
    if all_is_license_tag && all_rule_length_one {
        return true;
    }

    // Check 5: Single is_license_reference match with short rule length
    // This filters false positives like "borceux" matching the word "GPL"
    if is_single
        && matches[0].is_license_reference
        && matches[0].rule_length <= FALSE_POSITIVE_RULE_LENGTH_THRESHOLD
    {
        return true;
    }

    false
}

/// Check if matches are low quality (below clue threshold).
///
/// Based on Python: is_low_quality_matches() at detection.py:1275
fn is_low_quality_matches(matches: &[LicenseMatch]) -> bool {
    if matches.is_empty() {
        return true;
    }

    !is_correctDetection(matches)
        && is_match_coverage_below_threshold(matches, CLUES_MATCH_COVERAGE_THR, false)
}

/// Check if matches represent an undetected license.
///
/// Returns true if there is exactly one match and its matcher is "5-undetected".
/// This indicates no license was found in the analyzed text.
///
/// Based on Python: is_undetected_license_matches() at detection.py:1054
fn is_undetected_license_matches(matches: &[LicenseMatch]) -> bool {
    if matches.len() != 1 {
        return false;
    }
    matches[0].matcher == "5-undetected"
}

/// Check if there's an unknown license intro followed by a proper detection.
///
/// This detects cases where a license introduction statement (like "Licensed under")
/// is immediately followed by a proper license match. The intro can be discarded
/// as it's describing the license that follows.
///
/// Based on Python: has_unknown_intro_before_detection() at detection.py:1289
fn has_unknown_intro_before_detection(matches: &[LicenseMatch]) -> bool {
    if matches.len() == 1 {
        return false;
    }

    let all_unknown_intro = matches.iter().all(is_unknown_intro);
    if all_unknown_intro {
        return false;
    }

    let mut has_unknown_intro = false;

    for m in matches {
        if is_unknown_intro(m) {
            has_unknown_intro = true;
            continue;
        }

        if has_unknown_intro {
            let coverage_ok = m.match_coverage >= IMPERFECT_MATCH_COVERAGE_THR - 0.01;
            let not_unknown =
                !m.rule_identifier.contains("unknown") && !m.license_expression.contains("unknown");
            if coverage_ok && not_unknown {
                return true;
            }
        }
    }

    if has_unknown_intro {
        let filtered_matches = filter_license_intros(matches);
        if filtered_matches.len() != matches.len()
            && is_match_coverage_below_threshold(
                &filtered_matches,
                IMPERFECT_MATCH_COVERAGE_THR,
                false,
            )
        {
            return true;
        }
    }

    false
}

/// Check if a match is an unknown license intro.
///
/// Based on Python: is_unknown_intro() at detection.py:1250-1262
fn is_unknown_intro(m: &LicenseMatch) -> bool {
    let has_unknown = m.license_expression.contains("unknown");
    has_unknown
        && (m.is_license_intro || m.is_license_clue || m.license_expression == "free-unknown")
}

/// Check if a match should be considered a license intro for filtering.
///
/// A match is considered a license intro if it has is_license_intro or
/// is_license_clue flag set OR its license_expression is "free-unknown",
/// AND it was matched by the "2-aho" matcher OR has 100% match coverage.
///
/// Based on Python: is_license_intro() at detection.py:1349-1365
fn is_license_intro(match_item: &LicenseMatch) -> bool {
    (match_item.is_license_intro
        || match_item.is_license_clue
        || match_item.license_expression == "free-unknown")
        && (match_item.matcher == "2-aho" || match_item.match_coverage >= 99.99)
}

/// Filter out license intro matches from a list of matches.
///
/// Returns matches with intro matches removed. If filtering would result in
/// an empty list, returns the original matches unchanged.
///
/// Based on Python: filter_license_intros() at detection.py:1336-1347
fn filter_license_intros(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    let filtered: Vec<_> = matches
        .iter()
        .filter(|m| !is_license_intro(m))
        .cloned()
        .collect();

    if filtered.is_empty() {
        matches.to_vec()
    } else {
        filtered
    }
}

/// Check if a match has a reference to a local file.
///
/// Returns true if the match has non-empty `referenced_filenames`,
/// indicating it references another file (e.g., "See LICENSE file").
///
/// Based on Python: is_license_reference_local_file() at detection.py:1368-1374
fn is_license_reference_local_file(m: &LicenseMatch) -> bool {
    m.referenced_filenames
        .as_ref()
        .is_some_and(|f| !f.is_empty())
}

/// Filter out matches that reference local files.
///
/// Returns matches with license reference matches removed. If filtering would
/// result in an empty list, returns the original matches unchanged.
///
/// Based on Python: filter_license_references() at detection.py:1377-1389
fn filter_license_references(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    let filtered: Vec<_> = matches
        .iter()
        .filter(|m| !is_license_reference_local_file(m))
        .cloned()
        .collect();

    if filtered.is_empty() {
        matches.to_vec()
    } else {
        filtered
    }
}

/// Filter out both license intro matches and license reference matches.
///
/// Applies filter_license_intros first, then filter_license_references.
///
/// Based on Python: filter_license_intros_and_references() at detection.py:1392-1399
fn filter_license_intros_and_references(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    let filtered = filter_license_intros(matches);
    filter_license_references(&filtered)
}

/// Check if matches have references to local files.
///
/// This is detected when a rule has `referenced_filenames` populated,
/// indicating the match references another file (e.g., "See LICENSE file").
///
/// Based on Python: has_references_to_local_files() at detection.py:1402
fn has_references_to_local_files(matches: &[LicenseMatch]) -> bool {
    !has_extra_words(matches)
        && matches.iter().any(|m| {
            m.referenced_filenames
                .as_ref()
                .is_some_and(|f| !f.is_empty())
        })
}

/// Analyze detection and return the appropriate detection log category.
///
/// This implements the full detection analysis logic from Python's analyze_detection()
/// function, determining what category a group of matches falls into.
///
/// Based on Python: analyze_detection() at detection.py:1760
fn analyze_detection(matches: &[LicenseMatch], package_license: bool) -> &'static str {
    if is_undetected_license_matches(matches) {
        return DETECTION_LOG_UNDETECTED_LICENSE;
    }

    if has_unknown_intro_before_detection(matches) {
        return DETECTION_LOG_UNKNOWN_INTRO_FOLLOWED_BY_MATCH;
    }

    if has_references_to_local_files(matches) {
        return DETECTION_LOG_UNKNOWN_REFERENCE_TO_LOCAL_FILE;
    }

    if !package_license && is_false_positive(matches) {
        return DETECTION_LOG_FALSE_POSITIVE;
    }

    if !package_license && is_low_quality_matches(matches) {
        return DETECTION_LOG_LICENSE_CLUES;
    }

    if is_correctDetection(matches) && !has_unknown_matches(matches) && !has_extra_words(matches) {
        return DETECTION_LOG_PERFECT_DETECTION;
    }

    if has_unknown_matches(matches) {
        return DETECTION_LOG_UNKNOWN_MATCH;
    }

    if !package_license && is_low_quality_matches(matches) {
        return DETECTION_LOG_LOW_QUALITY_MATCHES;
    }

    if is_match_coverage_below_threshold(matches, IMPERFECT_MATCH_COVERAGE_THR, true) {
        return DETECTION_LOG_IMPERFECT_COVERAGE;
    }

    if has_extra_words(matches) {
        return DETECTION_LOG_EXTRA_WORDS;
    }

    DETECTION_LOG_PERFECT_DETECTION
}

/// Compute detection score from grouped matches.
///
/// The score is the weighted average of match scores, weighted by match length.
/// This ensures longer matches have more influence on the overall score.
///
/// Score formula: sum(match_score * (match_length / total_length))
/// Cap at 100.0 as the maximum.
///
/// Based on Python: LicenseDetection.score() at detection.py:398
pub fn compute_detection_score(matches: &[LicenseMatch]) -> f32 {
    if matches.is_empty() {
        return 0.0;
    }

    if matches.len() == 1 {
        return matches[0].score.min(100.0);
    }

    let total_length: f32 = matches.iter().map(|m| m.matched_length as f32).sum();

    if total_length < 0.01 {
        return matches.iter().map(|m| m.score).sum::<f32>() / matches.len() as f32;
    }

    let weighted_score: f32 = matches
        .iter()
        .map(|m| {
            let weight = m.matched_length as f32 / total_length;
            m.score * weight
        })
        .sum();

    weighted_score.min(100.0)
}

/// Determine license expression from matches.
///
/// Combines license expressions from all matches using AND relation.
///
/// Returns error if expression combination fails.
///
/// Based on Python: get_detected_license_expression() at detection.py:1468
pub fn determine_license_expression(matches: &[LicenseMatch]) -> Result<String, String> {
    if matches.is_empty() {
        return Err("No matches to determine expression".to_string());
    }

    let expressions: Vec<&str> = matches
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();

    combine_expressions(&expressions, CombineRelation::And, true)
        .map_err(|e| format!("Failed to combine expressions: {:?}", e))
}

/// Determine SPDX license expression from matches.
///
/// Combines SPDX license expressions from all matches using AND relation.
///
/// Returns error if expression combination fails.
///
/// Based on Python: detection.spdx_license_expression() at detection.py:269
pub fn determine_spdx_expression(matches: &[LicenseMatch]) -> Result<String, String> {
    if matches.is_empty() {
        return Err("No matches to determine expression".to_string());
    }

    let expressions: Vec<&str> = matches
        .iter()
        .map(|m| m.license_expression_spdx.as_str())
        .collect();

    combine_expressions(&expressions, CombineRelation::And, true)
        .map_err(|e| format!("Failed to combine SPDX expressions: {:?}", e))
}

/// Determine SPDX license expression from ScanCode expression using SpdxMapping.
///
/// Converts ScanCode license keys to SPDX license keys in the expression.
///
/// Based on Python: build_spdx_license_expression() in cache.py
///
/// # Arguments
///
/// * `scancode_expression` - License expression with ScanCode keys
/// * `spdx_mapping` - SPDX mapping for conversion
///
/// # Returns
///
/// SPDX license expression string, or error if conversion fails
pub fn determine_spdx_expression_from_scancode(
    scancode_expression: &str,
    spdx_mapping: &SpdxMapping,
) -> Result<String, String> {
    if scancode_expression.is_empty() {
        return Ok(String::new());
    }

    spdx_mapping
        .expression_scancode_to_spdx(scancode_expression)
        .map_err(|e| e.to_string())
}

/// Classify detection as valid (true positive) or invalid.
///
/// A detection is valid if:
/// - Score meets minimum threshold
/// - Not identified as low quality matches
/// - Not identified as false positive
///
/// Based on Python: is_correct_detection_non_unknown() at detection.py:1066
pub fn classify_detection(detection: &LicenseDetection, min_score: f32) -> bool {
    if detection.matches.is_empty() {
        return false;
    }

    let score = compute_detection_score(&detection.matches);
    let meets_score_threshold = score >= min_score - 0.01;
    let not_low_quality = !is_low_quality_matches(&detection.matches);
    let not_false_positive = !is_false_positive(&detection.matches);

    meets_score_threshold && not_low_quality && not_false_positive
}

/// Populate LicenseDetection from a DetectionGroup.
///
/// This function:
/// 1. Computes the detection score
/// 2. Determines the license expression
/// 3. Adds appropriate detection log entries
/// 4. Creates the identifier
///
/// Parameter `index` is reserved for future use (e.g., spdx conversion).
pub fn populate_detection_from_group(detection: &mut LicenseDetection, group: &DetectionGroup) {
    if group.matches.is_empty() {
        return;
    }

    detection.matches = group.matches.clone();

    let _score = compute_detection_score(&detection.matches);

    if let Ok(expr) = determine_license_expression(&detection.matches) {
        detection.license_expression = Some(expr.clone());

        if let Ok(spdx_expr) = determine_spdx_expression(&detection.matches) {
            detection.license_expression_spdx = Some(spdx_expr);
        }
    }

    let log_category = analyze_detection(&detection.matches, false);
    detection.detection_log.push(log_category.to_string());

    detection.identifier = None;

    if group.start_line > 0 {
        detection.file_region = Some(FileRegion {
            path: String::new(),
            start_line: group.start_line,
            end_line: group.end_line,
        });
    }
}

/// Populate LicenseDetection from a DetectionGroup with SPDX mapping.
///
/// This function:
/// 1. Computes the detection score
/// 2. Determines the ScanCode license expression
/// 3. Determines the SPDX license expression using the mapping
/// 4. Adds appropriate detection log entries
/// 5. Creates the identifier
///
/// # Arguments
///
/// * `detection` - LicenseDetection to populate
/// * `group` - DetectionGroup containing the matches
/// * `spdx_mapping` - SpdxMapping for SPDX conversion
pub fn populate_detection_from_group_with_spdx(
    detection: &mut LicenseDetection,
    group: &DetectionGroup,
    spdx_mapping: &SpdxMapping,
) {
    populate_detection_from_group(detection, group);

    if let Some(ref scancode_expr) = detection.license_expression
        && let Ok(spdx_expr) = determine_spdx_expression_from_scancode(scancode_expr, spdx_mapping)
    {
        detection.license_expression_spdx = Some(spdx_expr);
    }
}

/// Create a basic LicenseDetection from a DetectionGroup.
///
/// This function properly populates all detection fields using the
/// analysis logic defined in populate_detection_from_group.
///
/// # Arguments
///
/// * `group` - DetectionGroup containing the matches
///
/// # Returns
///
/// A fully populated LicenseDetection
pub fn create_detection_from_group(group: &DetectionGroup) -> LicenseDetection {
    let mut detection = LicenseDetection {
        license_expression: None,
        license_expression_spdx: None,
        matches: Vec::new(),
        detection_log: Vec::new(),
        identifier: None,
        file_region: None,
    };

    if group.matches.is_empty() {
        return detection;
    }

    let log_category = analyze_detection(&group.matches, false);

    let matches_for_expression = if log_category == DETECTION_LOG_UNKNOWN_INTRO_FOLLOWED_BY_MATCH {
        filter_license_intros(&group.matches)
    } else if log_category == DETECTION_LOG_UNKNOWN_REFERENCE_TO_LOCAL_FILE {
        filter_license_intros_and_references(&group.matches)
    } else {
        group.matches.clone()
    };

    detection.matches = matches_for_expression.clone();

    let _score = compute_detection_score(&detection.matches);

    if let Ok(expr) = determine_license_expression(&detection.matches) {
        detection.license_expression = Some(expr.clone());

        if let Ok(spdx_expr) = determine_spdx_expression(&detection.matches) {
            detection.license_expression_spdx = Some(spdx_expr);
        }
    }

    detection.detection_log.push(log_category.to_string());

    detection.identifier = None;

    if group.start_line > 0 {
        detection.file_region = Some(FileRegion {
            path: String::new(),
            start_line: group.start_line,
            end_line: group.end_line,
        });
    }

    detection
}

/// Filter detections by minimum score threshold.
///
/// Returns only detections with score >= min_score threshold.
///
/// Based on Python minimum score filtering in detection pipeline.
pub fn filter_detections_by_score(
    detections: Vec<LicenseDetection>,
    min_score: f32,
) -> Vec<LicenseDetection> {
    detections
        .into_iter()
        .filter(|detection| classify_detection(detection, min_score))
        .collect()
}

/// Remove duplicate detections (same identifier).
///
/// Groups detections by their identifier (license expression + content hash).
/// Detections with the same identifier represent the same license at the same
/// location. Detections with the same expression but different identifiers
/// represent the same license at DIFFERENT locations and should be kept separate.
///
/// Based on Python get_detections_by_id behavior in detection.py.
pub fn remove_duplicate_detections(detections: Vec<LicenseDetection>) -> Vec<LicenseDetection> {
    let mut detections_by_id: std::collections::HashMap<String, LicenseDetection> =
        std::collections::HashMap::new();

    for detection in detections {
        let identifier = detection
            .identifier
            .clone()
            .unwrap_or_else(|| compute_detection_identifier(&detection));

        let entry = detections_by_id.entry(identifier.clone());
        if let std::collections::hash_map::Entry::Vacant(e) = entry {
            let mut detection = detection;
            detection.identifier = Some(identifier);
            e.insert(detection);
        }
    }

    detections_by_id.into_values().collect()
}

/// Rank detections by score and coverage.
///
/// Sorts detections in descending order by:
/// 1. Detection score (higher is better)
/// 2. Detection coverage (higher is better)
///
/// Based on Python: sort_unique_detections() at detection.py:1003
pub fn rank_detections(mut detections: Vec<LicenseDetection>) -> Vec<LicenseDetection> {
    detections.sort_by(|a, b| {
        let score_a = compute_detection_score(&a.matches);
        let score_b = compute_detection_score(&b.matches);
        let coverage_a = compute_detection_coverage(&a.matches);
        let coverage_b = compute_detection_coverage(&b.matches);

        score_b
            .partial_cmp(&score_a)
            .unwrap()
            .then_with(|| coverage_b.partial_cmp(&coverage_a).unwrap())
    });

    detections
}

/// Sort detections by minimum line number (earliest match first).
///
/// This matches Python's qstart ordering, ensuring detections
/// earlier in the file come first in the results.
pub fn sort_detections_by_line(mut detections: Vec<LicenseDetection>) -> Vec<LicenseDetection> {
    detections.sort_by(|a, b| {
        let min_line_a = a.matches.iter().map(|m| m.start_line).min().unwrap_or(0);
        let min_line_b = b.matches.iter().map(|m| m.start_line).min().unwrap_or(0);
        min_line_a.cmp(&min_line_b)
    });
    detections
}

fn python_safe_name(s: &str) -> String {
    let mut result = String::new();
    let mut prev_underscore = false;

    for c in s.chars() {
        if c.is_alphanumeric() {
            result.push(c);
            prev_underscore = false;
        } else if !prev_underscore {
            result.push('_');
            prev_underscore = true;
        }
    }

    let trimmed = result.trim_matches('_');
    if trimmed.is_empty() {
        String::new()
    } else {
        trimmed.to_string()
    }
}

fn get_uuid_on_content(content: &[(&str, f32, &str)]) -> String {
    let content_tuple: Vec<(&str, f32, &str)> = content.to_vec();

    let repr_str = format!("{:?}", content_tuple);

    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(repr_str.as_bytes());
    let hash = hasher.finalize();
    let hex_str = hex::encode(hash);

    let uuid_hex = &hex_str[..32];

    uuid::Uuid::parse_str(uuid_hex)
        .map(|u| u.to_string())
        .unwrap_or_else(|_| uuid_hex.to_string())
}

fn compute_content_identifier(matches: &[LicenseMatch]) -> String {
    let content: Vec<(&str, f32, &str)> = matches
        .iter()
        .map(|m| {
            let matched_text = m.matched_text.as_deref().unwrap_or("");
            (m.rule_identifier.as_str(), m.score, matched_text)
        })
        .collect();

    get_uuid_on_content(&content)
}

pub fn compute_detection_identifier(detection: &LicenseDetection) -> String {
    let expression = detection
        .license_expression
        .as_ref()
        .map(|s| python_safe_name(s))
        .unwrap_or_default();

    let content_uuid = compute_content_identifier(&detection.matches);
    format!("{}-{}", expression, content_uuid)
}

/// Compute detection coverage from matches.
///
/// Average of match_coverage weighted by matched_length.
/// Capped at 100.0 as the maximum.
///
/// Based on Python: LicenseDetection.coverage() at detection.py:373
fn compute_detection_coverage(matches: &[LicenseMatch]) -> f32 {
    if matches.is_empty() {
        return 0.0;
    }

    if matches.len() == 1 {
        return matches[0].match_coverage.min(100.0);
    }

    let total_length: f32 = matches.iter().map(|m| m.matched_length as f32).sum();

    if total_length < 0.01 {
        return matches.iter().map(|m| m.match_coverage).sum::<f32>() / matches.len() as f32;
    }

    let weighted_coverage: f32 = matches
        .iter()
        .map(|m| {
            let weight = m.matched_length as f32 / total_length;
            m.match_coverage * weight
        })
        .sum();

    weighted_coverage.min(100.0)
}

/// Get matcher priority for detection preference.
///
/// Returns a priority score where lower values mean higher preference.
/// Preference order: SPDX-LID (1) > hash (2) > Aho (3) > seq (4) > unknown (5)
///
/// Based on ScanCode matcher strategy ordering.
fn get_matcher_priority(matcher: &str) -> u8 {
    if matcher == "1-spdx-id" {
        1
    } else if matcher == "1-hash" {
        2
    } else if matcher == "2-aho" {
        3
    } else if matcher.starts_with("3-seq") {
        4
    } else {
        5
    }
}

/// Apply detection preferences based on matcher type.
///
/// When multiple detections have similar scores, prefer:
/// 1. SPDX-LID matches over hash matches
/// 2. Hash matches over Aho matches
/// 3. Aho matches over sequence matches
/// 4. Sequence matches over unknown matches
///
/// Based on ScanCode matcher strategy preferences.
pub fn apply_detection_preferences(detections: Vec<LicenseDetection>) -> Vec<LicenseDetection> {
    let mut processed: std::collections::HashMap<String, (f32, u8, LicenseDetection)> =
        std::collections::HashMap::new();

    for detection in detections {
        let expr = detection
            .license_expression
            .clone()
            .unwrap_or_else(String::new);
        let score = compute_detection_score(&detection.matches);

        let best_matcher_priority = detection
            .matches
            .iter()
            .map(|m| get_matcher_priority(&m.matcher))
            .min()
            .unwrap_or(5);

        let should_keep = processed
            .get(&expr)
            .map(|(existing_score, existing_priority, _)| {
                if (score - existing_score).abs() < 0.01 {
                    best_matcher_priority < *existing_priority
                } else {
                    score > *existing_score
                }
            })
            .unwrap_or(true);

        if should_keep {
            processed.insert(expr, (score, best_matcher_priority, detection));
        }
    }

    processed
        .into_values()
        .map(|(_, _, detection)| detection)
        .collect()
}

/// Main post-processing function for detections.
///
/// Applies the following steps in order:
/// 1. Filter detections by minimum score threshold
/// 2. Remove duplicate detections (same license expression)
/// 3. Apply detection preferences (based on matcher type)
/// 4. Rank detections by score and coverage
///
/// This is the main entry point for post-processing after all detections
/// have been created.
///
/// # Arguments
///
/// * `detections` - Raw detections from detection grouping
/// * `min_score` - Minimum score threshold (default: 90.0)
///
/// # Returns
///
/// Post-processed and ranked list of detections
pub fn post_process_detections(
    detections: Vec<LicenseDetection>,
    min_score: f32,
) -> Vec<LicenseDetection> {
    let filtered = filter_detections_by_score(detections, min_score);
    let deduplicated = remove_duplicate_detections(filtered);
    let preferred = apply_detection_preferences(deduplicated);
    let ranked = rank_detections(preferred);
    sort_detections_by_line(ranked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::license_detection::models::License;
    use crate::license_detection::spdx_mapping::build_spdx_mapping;

    fn create_test_match(
        start_line: usize,
        end_line: usize,
        matcher: &str,
        rule_identifier: &str,
    ) -> LicenseMatch {
        LicenseMatch {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: Some("test.txt".to_string()),
            start_line,
            end_line,
            start_token: 0,
            end_token: 0,
            matcher: matcher.to_string(),
            score: 0.95,
            matched_length: 100,
            match_coverage: 95.0,
            rule_relevance: 100,
            rule_identifier: rule_identifier.to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: Some("MIT License".to_string()),
            referenced_filenames: None,
            is_license_intro: false,
            is_license_clue: false,
            is_license_reference: false,
            is_license_tag: false,
            rule_length: 100,
            matched_token_positions: None,
            hilen: 50,
        }
    }

    #[test]
    fn test_group_matches_empty() {
        let matches = Vec::new();
        let groups = group_matches_by_region(&matches);

        assert_eq!(groups.len(), 0);
    }

    #[test]
    fn test_group_matches_single() {
        let match1 = create_test_match(1, 5, "1-hash", "mit.LICENSE");
        let matches = vec![match1];
        let groups = group_matches_by_region(&matches);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].matches.len(), 1);
        assert_eq!(groups[0].start_line, 1);
        assert_eq!(groups[0].end_line, 5);
    }

    #[test]
    fn test_group_matches_within_threshold() {
        let match1 = create_test_match(1, 5, "1-hash", "mit.LICENSE");
        let match2 = create_test_match(6, 10, "2-aho", "mit.LICENSE");
        let matches = vec![match1, match2];
        let groups = group_matches_by_region(&matches);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].matches.len(), 2);
        assert_eq!(groups[0].start_line, 1);
        assert_eq!(groups[0].end_line, 10);
    }

    #[test]
    fn test_group_matches_separate_by_threshold() {
        let match1 = create_test_match(1, 5, "1-hash", "mit.LICENSE");
        let match2 = create_test_match(10, 15, "1-hash", "apache-2.0.LICENSE");
        let matches = vec![match1, match2];
        let groups = group_matches_by_region(&matches);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].matches.len(), 1);
        assert_eq!(groups[1].matches.len(), 1);
        assert_eq!(groups[0].start_line, 1);
        assert_eq!(groups[0].end_line, 5);
        assert_eq!(groups[1].start_line, 10);
        assert_eq!(groups[1].end_line, 15);
    }

    #[test]
    fn test_group_matches_exactly_at_line_gap_threshold() {
        let match1 = create_test_match(1, 5, "1-hash", "mit.LICENSE");
        let match2 = create_test_match(8, 12, "2-aho", "mit.LICENSE");
        let matches = vec![match1, match2];
        let groups = group_matches_by_region(&matches);

        assert_eq!(groups.len(), 1, "Line gap 3 (8-5=3) should be grouped");
        assert_eq!(groups[0].matches.len(), 2);
    }

    #[test]
    fn test_group_matches_just_past_line_gap_threshold() {
        let match1 = create_test_match(1, 5, "1-hash", "mit.LICENSE");
        let match2 = create_test_match(9, 13, "2-aho", "mit.LICENSE");
        let matches = vec![match1, match2];
        let groups = group_matches_by_region(&matches);

        assert_eq!(
            groups.len(),
            2,
            "Line gap 4 (9-5=4) exceeds threshold 3, should separate"
        );
    }

    #[test]
    fn test_group_matches_far_apart() {
        let match1 = create_test_match(1, 5, "1-hash", "mit.LICENSE");
        let match2 = create_test_match(20, 25, "1-hash", "apache-2.0.LICENSE");
        let matches = vec![match1, match2];
        let groups = group_matches_by_region(&matches);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].matches.len(), 1);
        assert_eq!(groups[1].matches.len(), 1);
        assert_eq!(groups[0].start_line, 1);
        assert_eq!(groups[0].end_line, 5);
        assert_eq!(groups[1].start_line, 20);
        assert_eq!(groups[1].end_line, 25);
    }

    #[test]
    fn test_sort_matches_by_line() {
        let mut matches = vec![
            create_test_match(10, 15, "1-hash", "mit.LICENSE"),
            create_test_match(1, 5, "2-aho", "apache-2.0.LICENSE"),
            create_test_match(6, 9, "1-spdx-id", "gpl.LICENSE"),
        ];

        sort_matches_by_line(&mut matches);

        assert_eq!(matches[0].start_line, 1);
        assert_eq!(matches[1].start_line, 6);
        assert_eq!(matches[2].start_line, 10);
    }

    #[test]
    fn test_detection_group_new_empty() {
        let group = DetectionGroup::new(Vec::new());

        assert!(group.matches.is_empty());
        assert_eq!(group.start_line, 0);
        assert_eq!(group.end_line, 0);
    }

    #[test]
    fn test_detection_group_new_with_matches() {
        let match1 = create_test_match(1, 5, "1-hash", "mit.LICENSE");
        let match2 = create_test_match(10, 15, "2-aho", "mit.LICENSE");
        let group = DetectionGroup::new(vec![match1, match2]);

        assert_eq!(group.matches.len(), 2);
        assert_eq!(group.start_line, 1);
        assert_eq!(group.end_line, 15);
    }

    #[test]
    fn test_create_detection_from_group_empty() {
        let group = DetectionGroup::new(Vec::new());
        let detection = create_detection_from_group(&group);

        assert!(detection.license_expression.is_none());
        assert!(detection.matches.is_empty());
        assert!(detection.identifier.is_none());
    }

    #[test]
    fn test_create_detection_from_group_with_matches() {
        let match1 = create_test_match(1, 5, "1-hash", "mit.LICENSE");
        let group = DetectionGroup::new(vec![match1]);
        let detection = create_detection_from_group(&group);

        assert_eq!(detection.matches.len(), 1);
        assert!(!detection.detection_log.is_empty());
    }

    #[test]
    fn test_lines_threshold_constant() {
        assert_eq!(LINES_THRESHOLD, 4);
    }

    fn create_test_match_with_tokens(
        start_line: usize,
        end_line: usize,
        start_token: usize,
        end_token: usize,
    ) -> LicenseMatch {
        LicenseMatch {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: Some("test.txt".to_string()),
            start_line,
            end_line,
            start_token,
            end_token,
            matcher: "1-hash".to_string(),
            score: 0.95,
            matched_length: 100,
            match_coverage: 95.0,
            rule_relevance: 100,
            rule_identifier: "mit.LICENSE".to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: Some("MIT License".to_string()),
            referenced_filenames: None,
            is_license_intro: false,
            is_license_clue: false,
            is_license_reference: false,
            is_license_tag: false,
            rule_length: 100,
            matched_token_positions: None,
            hilen: 50,
        }
    }

    #[test]
    fn test_grouping_within_both_thresholds() {
        let m1 = create_test_match_with_tokens(1, 10, 0, 50);
        let m2 = create_test_match_with_tokens(12, 20, 55, 100);
        let groups = group_matches_by_region(&[m1, m2]);
        assert_eq!(
            groups.len(),
            1,
            "Should group when both line gap (2) and token gap (5) within thresholds"
        );
    }

    #[test]
    fn test_grouping_separates_by_line_threshold() {
        let m1 = create_test_match_with_tokens(1, 10, 0, 50);
        let m2 = create_test_match_with_tokens(15, 25, 55, 100);
        let groups = group_matches_by_region(&[m1, m2]);
        assert_eq!(
            groups.len(),
            2,
            "Should separate when line gap (5) exceeds threshold (3)"
        );
    }

    #[test]
    fn test_grouping_separates_by_token_threshold() {
        let m1 = create_test_match_with_tokens(1, 10, 0, 50);
        let m2 = create_test_match_with_tokens(12, 20, 65, 100);
        let groups = group_matches_by_region(&[m1, m2]);
        assert_eq!(
            groups.len(),
            2,
            "Should separate when token gap (15) exceeds threshold (10)"
        );
    }

    #[test]
    fn test_grouping_at_exact_line_threshold() {
        let m1 = create_test_match_with_tokens(1, 10, 0, 50);
        let m2 = create_test_match_with_tokens(13, 20, 55, 100);
        let groups = group_matches_by_region(&[m1, m2]);
        assert_eq!(
            groups.len(),
            1,
            "Should group at exact line gap (3) within threshold"
        );
    }

    #[test]
    fn test_grouping_at_exact_token_threshold() {
        let m1 = create_test_match_with_tokens(1, 10, 0, 50);
        let m2 = create_test_match_with_tokens(11, 20, 60, 100);
        let groups = group_matches_by_region(&[m1, m2]);
        assert_eq!(
            groups.len(),
            1,
            "Should group at exact token gap (10) within threshold"
        );
    }

    #[test]
    fn test_grouping_requires_both_thresholds() {
        let m1 = create_test_match_with_tokens(1, 10, 0, 50);
        let m2 = create_test_match_with_tokens(15, 25, 65, 100);
        let groups = group_matches_by_region(&[m1, m2]);
        assert_eq!(
            groups.len(),
            2,
            "Should separate when both line gap (5) and token gap (15) exceed thresholds"
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn create_test_match_with_params(
        license_expression: &str,
        matcher: &str,
        start_line: usize,
        end_line: usize,
        score: f32,
        matched_length: usize,
        rule_length: usize,
        match_coverage: f32,
        rule_relevance: u8,
        rule_identifier: &str,
    ) -> LicenseMatch {
        LicenseMatch {
            license_expression: license_expression.to_string(),
            license_expression_spdx: license_expression.to_string(),
            from_file: Some("test.txt".to_string()),
            start_line,
            end_line,
            start_token: 0,
            end_token: 0,
            matcher: matcher.to_string(),
            score,
            matched_length,
            rule_length,
            match_coverage,
            rule_relevance,
            rule_identifier: rule_identifier.to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: Some("License text".to_string()),
            referenced_filenames: None,
            is_license_intro: false,
            is_license_clue: false,
            is_license_reference: false,
            is_license_tag: false,
            matched_token_positions: None,
            hilen: matched_length / 2,
        }
    }

    #[test]
    fn test_is_correct_detection_perfect_hash() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "1-hash",
            1,
            10,
            95.0,
            100,
            100,
            100.0,
            100,
            "mit.LICENSE",
        )];

        assert!(is_correctDetection(&matches));
    }

    #[test]
    fn test_is_correct_detection_perfect_spdx() {
        let matches = vec![create_test_match_with_params(
            "apache-2.0",
            "1-spdx-id",
            1,
            10,
            95.0,
            100,
            100,
            100.0,
            100,
            "apache-2.0.LICENSE",
        )];

        assert!(is_correctDetection(&matches));
    }

    #[test]
    fn test_is_correct_detection_perfect_aho() {
        let matches = vec![create_test_match_with_params(
            "gpl-2.0",
            "2-aho",
            1,
            10,
            95.0,
            100,
            100,
            100.0,
            100,
            "gpl-2.0.LICENSE",
        )];

        assert!(is_correctDetection(&matches));
    }

    #[test]
    fn test_is_correct_detection_multiple_perfect() {
        let matches = vec![
            create_test_match_with_params("mit", "1-hash", 1, 10, 95.0, 100, 100, 100.0, 100, "#1"),
            create_test_match_with_params(
                "apache-2.0",
                "1-spdx-id",
                11,
                20,
                95.0,
                100,
                100,
                100.0,
                100,
                "#2",
            ),
        ];

        assert!(is_correctDetection(&matches));
    }

    #[test]
    fn test_is_correct_detection_imperfect_coverage() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "1-hash",
            1,
            10,
            85.0,
            100,
            100,
            95.0,
            100,
            "mit.LICENSE",
        )];

        assert!(!is_correctDetection(&matches));
    }

    #[test]
    fn test_is_correct_detection_unknown_matcher() {
        let matches = vec![create_test_match_with_params(
            "unknown",
            "5-unknown",
            1,
            10,
            50.0,
            50,
            50,
            50.0,
            50,
            "unknown.LICENSE",
        )];

        assert!(!is_correctDetection(&matches));
    }

    #[test]
    fn test_is_correct_detection_empty() {
        let matches: Vec<LicenseMatch> = vec![];
        assert!(!is_correctDetection(&matches));
    }

    #[test]
    fn test_is_correct_detection_snake_case() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "1-hash",
            1,
            10,
            95.0,
            100,
            100,
            100.0,
            100,
            "mit.LICENSE",
        )];

        assert!(is_correct_detection(&matches));
    }

    #[test]
    fn test_is_match_coverage_below_threshold_above() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "1-hash",
            1,
            10,
            95.0,
            100,
            100,
            80.0,
            100,
            "mit.LICENSE",
        )];

        assert!(!is_match_coverage_below_threshold(&matches, 70.0, true));
    }

    #[test]
    fn test_is_match_coverage_below_threshold_below() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "1-hash",
            1,
            10,
            65.0,
            100,
            100,
            65.0,
            100,
            "mit.LICENSE",
        )];

        assert!(is_match_coverage_below_threshold(&matches, 70.0, true));
    }

    #[test]
    fn test_is_match_coverage_below_threshold_exact() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "1-hash",
            1,
            10,
            60.0,
            100,
            100,
            60.0,
            100,
            "mit.LICENSE",
        )];

        assert!(!is_match_coverage_below_threshold(&matches, 60.0, true));
    }

    #[test]
    fn test_is_match_coverage_below_threshold_empty() {
        let matches: Vec<LicenseMatch> = vec![];
        assert!(!is_match_coverage_below_threshold(&matches, 60.0, true));
    }

    #[test]
    fn test_has_unknown_matches_false() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "1-hash",
            1,
            10,
            95.0,
            100,
            100,
            100.0,
            100,
            "mit.LICENSE",
        )];

        assert!(!has_unknown_matches(&matches));
    }

    #[test]
    fn test_has_unknown_matches_true_in_identifier() {
        let matches = vec![create_test_match_with_params(
            "unknown",
            "5-unknown",
            1,
            10,
            50.0,
            50,
            50,
            50.0,
            50,
            "unknown.LICENSE",
        )];

        assert!(has_unknown_matches(&matches));
    }

    #[test]
    fn test_has_unknown_matches_true_in_expression() {
        let matches = vec![create_test_match_with_params(
            "free-unknown",
            "2-aho",
            1,
            10,
            75.0,
            75,
            75,
            75.0,
            75,
            "#42",
        )];

        assert!(has_unknown_matches(&matches));
    }

    #[test]
    fn test_has_extra_words_false() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "1-hash",
            1,
            10,
            100.0,
            100,
            100,
            100.0,
            100,
            "mit.LICENSE",
        )];

        assert!(!has_extra_words(&matches));
    }

    #[test]
    fn test_has_extra_words_true() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "2-aho",
            1,
            10,
            50.0,
            100,
            100,
            60.0,
            100,
            "mit.LICENSE",
        )];

        assert!(has_extra_words(&matches));
    }

    #[test]
    fn test_is_false_positive_bare_single() {
        let matches = vec![create_test_match_with_params(
            "gpl",
            "2-aho",
            2000,
            2005,
            30.0,
            3,
            3,
            30.0,
            50,
            "gpl_bare.LICENSE",
        )];

        assert!(is_false_positive(&matches));
    }

    #[test]
    fn test_is_false_positive_gpl_short() {
        // GPL with rule_length == 1 and low relevance should be filtered
        let matches = vec![create_test_match_with_params(
            "gpl-2.0",
            "2-aho",
            1,
            10,
            50.0,
            2,
            1,
            50.0,
            50,
            "gpl-2.0.LICENSE",
        )];

        assert!(is_false_positive(&matches));
    }

    #[test]
    fn test_is_false_positive_late_short_low_relevance() {
        // Late match with low relevance and short rule_length should be filtered
        let matches = vec![create_test_match_with_params(
            "mit",
            "2-aho",
            1500,
            1505,
            30.0,
            3,
            3,
            30.0,
            50,
            "mit.LICENSE",
        )];

        assert!(is_false_positive(&matches));
    }

    #[test]
    fn test_is_false_positive_perfect_match() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "1-hash",
            1,
            10,
            95.0,
            100,
            100,
            100.0,
            100,
            "mit.LICENSE",
        )];

        assert!(!is_false_positive(&matches));
    }

    #[test]
    fn test_is_false_positive_empty() {
        let matches: Vec<LicenseMatch> = vec![];
        assert!(!is_false_positive(&matches));
    }

    #[test]
    fn test_is_false_positive_single_license_reference_short() {
        let mut m = create_test_match_with_params(
            "borceux",
            "2-aho",
            1,
            10,
            100.0,
            1,
            1,
            100.0,
            80,
            "borceux.LICENSE",
        );
        m.is_license_reference = true;
        m.rule_length = 1;
        let matches = vec![m];
        assert!(is_false_positive(&matches));
    }

    #[test]
    fn test_is_false_positive_single_license_reference_long_rule() {
        let mut m = create_test_match_with_params(
            "some-license",
            "2-aho",
            1,
            10,
            100.0,
            10,
            10,
            100.0,
            80,
            "some-license.LICENSE",
        );
        m.is_license_reference = true;
        m.rule_length = 10;
        let matches = vec![m];
        assert!(
            !is_false_positive(&matches),
            "Long rule_length should not be filtered"
        );
    }

    #[test]
    fn test_is_false_positive_single_license_reference_full_relevance() {
        let mut m = create_test_match_with_params(
            "some-license",
            "2-aho",
            1,
            10,
            100.0,
            1,
            1,
            100.0,
            100,
            "some-license.LICENSE",
        );
        m.is_license_reference = true;
        m.rule_length = 1;
        let matches = vec![m];
        assert!(
            !is_false_positive(&matches),
            "Full relevance should not be filtered"
        );
    }

    #[test]
    fn test_is_low_quality_matches_low_coverage() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "2-aho",
            1,
            10,
            40.0,
            20,
            20,
            40.0,
            80,
            "mit.LICENSE",
        )];

        assert!(is_low_quality_matches(&matches));
    }

    #[test]
    fn test_is_low_quality_matches_false_perfect() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "1-hash",
            1,
            10,
            95.0,
            100,
            100,
            100.0,
            100,
            "mit.LICENSE",
        )];

        assert!(!is_low_quality_matches(&matches));
    }

    #[test]
    fn test_is_low_quality_matches_empty() {
        let matches: Vec<LicenseMatch> = vec![];
        assert!(is_low_quality_matches(&matches));
    }

    #[test]
    fn test_compute_detection_score_single() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "1-hash",
            1,
            10,
            95.0,
            100,
            100,
            95.0,
            100,
            "mit.LICENSE",
        )];

        let score = compute_detection_score(&matches);
        assert!((score - 95.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_detection_score_multiple_equal() {
        let matches = vec![
            create_test_match_with_params("mit", "1-hash", 1, 10, 95.0, 100, 100, 95.0, 100, "#1"),
            create_test_match_with_params(
                "apache-2.0",
                "1-spdx-id",
                11,
                20,
                85.0,
                100,
                100,
                85.0,
                100,
                "#2",
            ),
        ];

        let score = compute_detection_score(&matches);
        assert!((score - 90.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_detection_score_weighted() {
        let matches = vec![
            create_test_match_with_params("mit", "1-hash", 1, 10, 95.0, 200, 200, 95.0, 100, "#1"),
            create_test_match_with_params(
                "apache-2.0",
                "1-spdx-id",
                11,
                15,
                85.0,
                50,
                50,
                85.0,
                100,
                "#2",
            ),
        ];

        let score = compute_detection_score(&matches);
        assert!((score - 93.0).abs() < 0.1);
    }

    #[test]
    fn test_compute_detection_score_empty() {
        let matches: Vec<LicenseMatch> = vec![];
        let score = compute_detection_score(&matches);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_compute_detection_score_capped_at_100() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "1-hash",
            1,
            10,
            150.0,
            100,
            100,
            100.0,
            100,
            "mit.LICENSE",
        )];

        let score = compute_detection_score(&matches);
        assert_eq!(score, 100.0);
    }

    #[test]
    fn test_determine_license_expression_single() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "1-hash",
            1,
            10,
            95.0,
            100,
            100,
            95.0,
            100,
            "mit.LICENSE",
        )];

        let expr = determine_license_expression(&matches);
        assert!(expr.is_ok());
        assert_eq!(expr.unwrap(), "mit");
    }

    #[test]
    fn test_determine_license_expression_multiple() {
        let matches = vec![
            create_test_match_with_params("mit", "1-hash", 1, 10, 95.0, 100, 100, 95.0, 100, "#1"),
            create_test_match_with_params(
                "apache-2.0",
                "1-spdx-id",
                11,
                20,
                85.0,
                100,
                100,
                85.0,
                100,
                "#2",
            ),
        ];

        let expr = determine_license_expression(&matches);
        assert!(expr.is_ok());
        let expr_value = expr.unwrap();
        assert!(expr_value.contains("mit"));
        assert!(expr_value.contains("apache-2.0"));
    }

    #[test]
    fn test_determine_license_expression_empty() {
        let matches: Vec<LicenseMatch> = vec![];
        let expr = determine_license_expression(&matches);
        assert!(expr.is_err());
    }

    #[test]
    fn test_classify_detection_valid_perfect() {
        let detection = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: None,
            matches: vec![create_test_match_with_params(
                "mit",
                "1-hash",
                1,
                10,
                95.0,
                100,
                100,
                100.0,
                100,
                "mit.LICENSE",
            )],
            detection_log: Vec::new(),
            identifier: None,
            file_region: None,
        };

        assert!(classify_detection(&detection, 90.0));
    }

    #[test]
    fn test_classify_detection_invalid_low_score() {
        let detection = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: None,
            matches: vec![create_test_match_with_params(
                "mit",
                "2-aho",
                1,
                10,
                30.0,
                50,
                50,
                30.0,
                50,
                "mit.LICENSE",
            )],
            detection_log: Vec::new(),
            identifier: None,
            file_region: None,
        };

        assert!(!classify_detection(&detection, 90.0));
    }

    #[test]
    fn test_classify_detection_invalid_false_positive() {
        let detection = LicenseDetection {
            license_expression: Some("gpl".to_string()),
            license_expression_spdx: None,
            matches: vec![create_test_match_with_params(
                "gpl",
                "2-aho",
                2000,
                2005,
                30.0,
                3,
                3,
                30.0,
                50,
                "gpl_bare.LICENSE",
            )],
            detection_log: Vec::new(),
            identifier: None,
            file_region: None,
        };

        assert!(!classify_detection(&detection, 0.0));
    }

    #[test]
    fn test_classify_detection_invalid_empty() {
        let detection = LicenseDetection {
            license_expression: None,
            license_expression_spdx: None,
            matches: Vec::new(),
            detection_log: Vec::new(),
            identifier: None,
            file_region: None,
        };

        assert!(!classify_detection(&detection, 0.0));
    }

    #[test]
    fn test_classify_detection_score_threshold() {
        let detection = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: None,
            matches: vec![create_test_match_with_params(
                "mit",
                "1-hash",
                1,
                10,
                85.0,
                100,
                100,
                100.0,
                100,
                "mit.LICENSE",
            )],
            detection_log: Vec::new(),
            identifier: None,
            file_region: None,
        };

        assert!(!classify_detection(&detection, 90.0));
        assert!(classify_detection(&detection, 80.0));
    }

    #[test]
    fn test_classify_detection_perfect_matches() {
        let detection = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: None,
            matches: vec![create_test_match_with_params(
                "mit",
                "1-hash",
                1,
                10,
                95.0,
                100,
                100,
                100.0,
                100,
                "mit.LICENSE",
            )],
            detection_log: Vec::new(),
            identifier: None,
            file_region: None,
        };

        assert!(classify_detection(&detection, 90.0));
    }

    #[test]
    fn test_populate_detection_from_group_perfect() {
        let group = DetectionGroup {
            matches: vec![create_test_match_with_params(
                "mit",
                "1-hash",
                1,
                10,
                100.0,
                100,
                100,
                100.0,
                100,
                "mit.LICENSE",
            )],
            start_line: 1,
            end_line: 10,
        };

        let mut detection = LicenseDetection {
            license_expression: None,
            license_expression_spdx: None,
            matches: Vec::new(),
            detection_log: Vec::new(),
            identifier: None,
            file_region: None,
        };

        populate_detection_from_group(&mut detection, &group);

        assert_eq!(detection.matches.len(), 1);
        assert!(detection.license_expression.is_some());
        assert_eq!(detection.license_expression.unwrap(), "mit");
        assert!(
            detection
                .detection_log
                .contains(&"perfect-detection".to_string())
        );
    }

    #[test]
    fn test_populate_detection_from_group_empty() {
        let group = DetectionGroup {
            matches: Vec::new(),
            start_line: 0,
            end_line: 0,
        };

        let mut detection = LicenseDetection {
            license_expression: None,
            license_expression_spdx: None,
            matches: Vec::new(),
            detection_log: Vec::new(),
            identifier: None,
            file_region: None,
        };

        populate_detection_from_group(&mut detection, &group);

        assert!(detection.matches.is_empty());
        assert!(detection.license_expression.is_none());
    }

    #[test]
    fn test_populate_detection_from_group_false_positive() {
        let group = DetectionGroup {
            matches: vec![create_test_match_with_params(
                "gpl",
                "2-aho",
                2000,
                2005,
                30.0,
                3,
                3,
                30.0,
                50,
                "gpl_bare.LICENSE",
            )],
            start_line: 2000,
            end_line: 2005,
        };

        let mut detection = LicenseDetection {
            license_expression: None,
            license_expression_spdx: None,
            matches: Vec::new(),
            detection_log: Vec::new(),
            identifier: None,
            file_region: None,
        };

        populate_detection_from_group(&mut detection, &group);

        assert_eq!(detection.matches.len(), 1);
        assert!(
            detection
                .detection_log
                .contains(&"possible-false-positive".to_string())
        );
        assert!(!classify_detection(&detection, 0.0));
    }

    #[test]
    fn test_imperfect_match_coverage_threshold_constant() {
        assert_eq!(IMPERFECT_MATCH_COVERAGE_THR, 100.0);
    }

    #[test]
    fn test_clues_match_coverage_threshold_constant() {
        assert_eq!(CLUES_MATCH_COVERAGE_THR, 60.0);
    }

    #[test]
    fn test_false_positive_rule_length_threshold_constant() {
        assert_eq!(FALSE_POSITIVE_RULE_LENGTH_THRESHOLD, 3);
    }

    #[test]
    fn test_false_positive_start_line_threshold_constant() {
        assert_eq!(FALSE_POSITIVE_START_LINE_THRESHOLD, 1000);
    }

    #[test]
    fn test_determine_spdx_expression_single() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "1-hash",
            1,
            10,
            95.0,
            100,
            100,
            95.0,
            100,
            "mit.LICENSE",
        )];

        let result = determine_spdx_expression(&matches);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "mit");
    }

    #[test]
    fn test_determine_spdx_expression_multiple() {
        let matches = vec![
            create_test_match_with_params("mit", "1-hash", 1, 10, 95.0, 100, 100, 95.0, 100, "#1"),
            create_test_match_with_params(
                "apache-2.0",
                "1-spdx-id",
                11,
                20,
                85.0,
                100,
                100,
                85.0,
                100,
                "#2",
            ),
        ];

        let result = determine_spdx_expression(&matches);
        assert!(result.is_ok());
        let expr_value = result.unwrap();
        assert!(expr_value.contains("mit"));
        assert!(expr_value.contains("apache-2.0"));
    }

    #[test]
    fn test_determine_spdx_expression_empty() {
        let matches: Vec<LicenseMatch> = vec![];
        let result = determine_spdx_expression(&matches);
        assert!(result.is_err());
    }

    #[test]
    fn test_determine_spdx_expression_from_scancode_single() {
        let licenses = vec![License {
            key: "mit".to_string(),
            name: "MIT License".to_string(),
            spdx_license_key: Some("MIT".to_string()),
            other_spdx_license_keys: vec![],
            category: Some("Permissive".to_string()),
            text: "MIT License text...".to_string(),
            reference_urls: vec![],
            notes: None,
            is_deprecated: false,
            replaced_by: vec![],
            minimum_coverage: None,
            ignorable_copyrights: None,
            ignorable_holders: None,
            ignorable_authors: None,
            ignorable_urls: None,
            ignorable_emails: None,
        }];
        let mapping = build_spdx_mapping(&licenses);

        let result = determine_spdx_expression_from_scancode("mit", &mapping);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "MIT");
    }

    #[test]
    fn test_determine_spdx_expression_from_scancode_multiple() {
        let licenses = vec![
            License {
                key: "mit".to_string(),
                name: "MIT License".to_string(),
                spdx_license_key: Some("MIT".to_string()),
                other_spdx_license_keys: vec![],
                category: Some("Permissive".to_string()),
                text: "MIT License text...".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
            },
            License {
                key: "apache-2.0".to_string(),
                name: "Apache License 2.0".to_string(),
                spdx_license_key: Some("Apache-2.0".to_string()),
                other_spdx_license_keys: vec![],
                category: Some("Permissive".to_string()),
                text: "Apache License text...".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
            },
        ];
        let mapping = build_spdx_mapping(&licenses);

        let result = determine_spdx_expression_from_scancode("mit AND apache-2.0", &mapping);
        assert!(result.is_ok());
        let expr = result.unwrap();
        assert!(expr.contains("MIT"));
        assert!(expr.contains("Apache-2.0"));
    }

    #[test]
    fn test_determine_spdx_expression_from_scancode_empty() {
        let licenses = vec![];
        let mapping = build_spdx_mapping(&licenses);

        let result = determine_spdx_expression_from_scancode("", &mapping);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn test_determine_spdx_expression_from_scancode_custom_license() {
        let licenses = vec![License {
            key: "custom-1".to_string(),
            name: "Custom License 1".to_string(),
            spdx_license_key: None,
            other_spdx_license_keys: vec![],
            category: Some("Unstated License".to_string()),
            text: "Custom license text...".to_string(),
            reference_urls: vec![],
            notes: None,
            is_deprecated: false,
            replaced_by: vec![],
            minimum_coverage: None,
            ignorable_copyrights: None,
            ignorable_holders: None,
            ignorable_authors: None,
            ignorable_urls: None,
            ignorable_emails: None,
        }];
        let mapping = build_spdx_mapping(&licenses);

        let result = determine_spdx_expression_from_scancode("custom-1", &mapping);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "LicenseRef-scancode-custom-1");
    }

    #[test]
    fn test_populate_detection_from_group_with_spdx_perfect() {
        let licenses = vec![License {
            key: "mit".to_string(),
            name: "MIT License".to_string(),
            spdx_license_key: Some("MIT".to_string()),
            other_spdx_license_keys: vec![],
            category: Some("Permissive".to_string()),
            text: "MIT License text...".to_string(),
            reference_urls: vec![],
            notes: None,
            is_deprecated: false,
            replaced_by: vec![],
            minimum_coverage: None,
            ignorable_copyrights: None,
            ignorable_holders: None,
            ignorable_authors: None,
            ignorable_urls: None,
            ignorable_emails: None,
        }];
        let mapping = build_spdx_mapping(&licenses);

        let group = DetectionGroup {
            matches: vec![create_test_match_with_params(
                "mit",
                "1-hash",
                1,
                10,
                100.0,
                100,
                100,
                100.0,
                100,
                "mit.LICENSE",
            )],
            start_line: 1,
            end_line: 10,
        };

        let mut detection = LicenseDetection {
            license_expression: None,
            license_expression_spdx: None,
            matches: Vec::new(),
            detection_log: Vec::new(),
            identifier: None,
            file_region: None,
        };

        populate_detection_from_group_with_spdx(&mut detection, &group, &mapping);

        assert_eq!(detection.matches.len(), 1);
        assert!(detection.license_expression.is_some());
        assert_eq!(detection.license_expression.unwrap(), "mit");
        assert!(detection.license_expression_spdx.is_some());
        assert_eq!(detection.license_expression_spdx.unwrap(), "MIT");
        assert!(
            detection
                .detection_log
                .contains(&"perfect-detection".to_string())
        );
    }

    #[test]
    fn test_populate_detection_from_group_with_spdx_empty() {
        let licenses = vec![];
        let mapping = build_spdx_mapping(&licenses);

        let group = DetectionGroup {
            matches: Vec::new(),
            start_line: 0,
            end_line: 0,
        };

        let mut detection = LicenseDetection {
            license_expression: None,
            license_expression_spdx: None,
            matches: Vec::new(),
            detection_log: Vec::new(),
            identifier: None,
            file_region: None,
        };

        populate_detection_from_group_with_spdx(&mut detection, &group, &mapping);

        assert!(detection.matches.is_empty());
        assert!(detection.license_expression.is_none());
        assert!(detection.license_expression_spdx.is_none());
    }

    #[test]
    fn test_populate_detection_from_group_with_spdx_multiple() {
        let licenses = vec![
            License {
                key: "mit".to_string(),
                name: "MIT License".to_string(),
                spdx_license_key: Some("MIT".to_string()),
                other_spdx_license_keys: vec![],
                category: Some("Permissive".to_string()),
                text: "MIT License text...".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
            },
            License {
                key: "apache-2.0".to_string(),
                name: "Apache License 2.0".to_string(),
                spdx_license_key: Some("Apache-2.0".to_string()),
                other_spdx_license_keys: vec![],
                category: Some("Permissive".to_string()),
                text: "Apache License text...".to_string(),
                reference_urls: vec![],
                notes: None,
                is_deprecated: false,
                replaced_by: vec![],
                minimum_coverage: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                ignorable_urls: None,
                ignorable_emails: None,
            },
        ];
        let mapping = build_spdx_mapping(&licenses);

        let group = DetectionGroup {
            matches: vec![
                create_test_match_with_params(
                    "mit", "1-hash", 1, 10, 100.0, 100, 100, 100.0, 100, "#1",
                ),
                create_test_match_with_params(
                    "apache-2.0",
                    "1-spdx-id",
                    11,
                    20,
                    100.0,
                    100,
                    100,
                    100.0,
                    100,
                    "#2",
                ),
            ],
            start_line: 1,
            end_line: 20,
        };

        let mut detection = LicenseDetection {
            license_expression: None,
            license_expression_spdx: None,
            matches: Vec::new(),
            detection_log: Vec::new(),
            identifier: None,
            file_region: None,
        };

        populate_detection_from_group_with_spdx(&mut detection, &group, &mapping);

        assert_eq!(detection.matches.len(), 2);
        assert!(detection.license_expression.is_some());
        let scancode_expr = detection.license_expression.unwrap();
        assert!(scancode_expr.contains("mit"));
        assert!(scancode_expr.contains("apache-2.0"));
        assert!(detection.license_expression_spdx.is_some());
        let spdx_expr = detection.license_expression_spdx.unwrap();
        assert!(spdx_expr.contains("MIT"));
        assert!(spdx_expr.contains("Apache-2.0"));
        assert!(
            detection
                .detection_log
                .contains(&"perfect-detection".to_string())
        );
    }

    #[test]
    fn test_populate_detection_from_group_with_spdx_custom_license() {
        let licenses = vec![License {
            key: "custom-1".to_string(),
            name: "Custom License 1".to_string(),
            spdx_license_key: None,
            other_spdx_license_keys: vec![],
            category: Some("Unstated License".to_string()),
            text: "Custom license text...".to_string(),
            reference_urls: vec![],
            notes: None,
            is_deprecated: false,
            replaced_by: vec![],
            minimum_coverage: None,
            ignorable_copyrights: None,
            ignorable_holders: None,
            ignorable_authors: None,
            ignorable_urls: None,
            ignorable_emails: None,
        }];
        let mapping = build_spdx_mapping(&licenses);

        let group = DetectionGroup {
            matches: vec![create_test_match_with_params(
                "custom-1",
                "2-aho",
                1,
                10,
                100.0,
                100,
                100,
                100.0,
                100,
                "custom-1.LICENSE",
            )],
            start_line: 1,
            end_line: 10,
        };

        let mut detection = LicenseDetection {
            license_expression: None,
            license_expression_spdx: None,
            matches: Vec::new(),
            detection_log: Vec::new(),
            identifier: None,
            file_region: None,
        };

        populate_detection_from_group_with_spdx(&mut detection, &group, &mapping);

        assert_eq!(detection.matches.len(), 1);
        assert!(detection.license_expression.is_some());
        assert_eq!(detection.license_expression.unwrap(), "custom-1");
        assert!(detection.license_expression_spdx.is_some());
        assert_eq!(
            detection.license_expression_spdx.unwrap(),
            "LicenseRef-scancode-custom-1"
        );
    }

    #[test]
    fn test_populate_detection_from_group_generates_spdx_expression() {
        let group = DetectionGroup {
            matches: vec![create_test_match_with_params(
                "mit",
                "1-hash",
                1,
                10,
                95.0,
                100,
                100,
                100.0,
                100,
                "custom-1.LICENSE",
            )],
            start_line: 1,
            end_line: 10,
        };

        let mut detection = LicenseDetection {
            license_expression: None,
            license_expression_spdx: None,
            matches: Vec::new(),
            detection_log: Vec::new(),
            identifier: None,
            file_region: None,
        };

        populate_detection_from_group(&mut detection, &group);

        assert_eq!(detection.matches.len(), 1);
        assert!(detection.license_expression.is_some());
        assert_eq!(detection.license_expression.unwrap(), "mit");
        assert!(detection.license_expression_spdx.is_some());
        assert_eq!(detection.license_expression_spdx.unwrap(), "mit");
    }

    #[test]
    fn test_filter_detections_by_score_all_pass() {
        let detections = vec![
            LicenseDetection {
                license_expression: Some("mit".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "mit",
                    "1-hash",
                    1,
                    10,
                    95.0,
                    100,
                    100,
                    100.0,
                    100,
                    "mit.LICENSE",
                )],
                detection_log: Vec::new(),
                identifier: None,
                file_region: None,
            },
            LicenseDetection {
                license_expression: Some("apache-2.0".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "apache-2.0",
                    "1-spdx-id",
                    1,
                    10,
                    90.0,
                    100,
                    100,
                    100.0,
                    100,
                    "#1",
                )],
                detection_log: Vec::new(),
                identifier: None,
                file_region: None,
            },
        ];

        let filtered = filter_detections_by_score(detections, 90.0);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_detections_by_score_some_filtered() {
        let detections = vec![
            LicenseDetection {
                license_expression: Some("mit".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "mit",
                    "1-hash",
                    1,
                    10,
                    95.0,
                    100,
                    100,
                    100.0,
                    100,
                    "mit.LICENSE",
                )],
                detection_log: Vec::new(),
                identifier: None,
                file_region: None,
            },
            LicenseDetection {
                license_expression: Some("mit".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "mit",
                    "2-aho",
                    1,
                    10,
                    30.0,
                    100,
                    100,
                    30.0,
                    50,
                    "mit.LICENSE",
                )],
                detection_log: Vec::new(),
                identifier: None,
                file_region: None,
            },
        ];

        let filtered = filter_detections_by_score(detections, 90.0);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].license_expression, Some("mit".to_string()));
    }

    #[test]
    fn test_filter_detections_by_score_all_filtered() {
        let detections = vec![LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: None,
            matches: vec![create_test_match_with_params(
                "mit",
                "2-aho",
                1,
                10,
                30.0,
                100,
                100,
                30.0,
                50,
                "mit.LICENSE",
            )],
            detection_log: Vec::new(),
            identifier: None,
            file_region: None,
        }];

        let filtered = filter_detections_by_score(detections, 90.0);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_detections_by_score_empty() {
        let detections = Vec::new();
        let filtered = filter_detections_by_score(detections, 90.0);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_remove_duplicate_detections_different_expressions() {
        let detections = vec![
            LicenseDetection {
                license_expression: Some("mit".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "mit",
                    "1-hash",
                    1,
                    10,
                    95.0,
                    100,
                    100,
                    100.0,
                    100,
                    "mit.LICENSE",
                )],
                detection_log: Vec::new(),
                identifier: None,
                file_region: None,
            },
            LicenseDetection {
                license_expression: Some("apache-2.0".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "apache-2.0",
                    "1-spdx-id",
                    1,
                    10,
                    90.0,
                    100,
                    100,
                    100.0,
                    100,
                    "#1",
                )],
                detection_log: Vec::new(),
                identifier: None,
                file_region: None,
            },
        ];

        let result = remove_duplicate_detections(detections);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_remove_duplicate_detections_same_identifier_removed() {
        let identifier = "mit-abc123".to_string();
        let detections = vec![
            LicenseDetection {
                license_expression: Some("mit".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "mit",
                    "1-hash",
                    1,
                    10,
                    95.0,
                    100,
                    100,
                    100.0,
                    100,
                    "mit.LICENSE",
                )],
                detection_log: Vec::new(),
                identifier: Some(identifier.clone()),
                file_region: None,
            },
            LicenseDetection {
                license_expression: Some("mit".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "mit",
                    "2-aho",
                    1,
                    10,
                    85.0,
                    100,
                    100,
                    85.0,
                    100,
                    "mit.LICENSE",
                )],
                detection_log: Vec::new(),
                identifier: Some(identifier.clone()),
                file_region: None,
            },
        ];

        let result = remove_duplicate_detections(detections);
        assert_eq!(result.len(), 1, "Same identifier should dedupe");
        assert_eq!(result[0].identifier, Some(identifier));
    }

    #[test]
    fn test_remove_duplicate_detections_same_expression_different_identifier() {
        let detections = vec![
            LicenseDetection {
                license_expression: Some("mit".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "mit",
                    "1-hash",
                    1,
                    10,
                    95.0,
                    100,
                    100,
                    100.0,
                    100,
                    "mit.LICENSE",
                )],
                detection_log: Vec::new(),
                identifier: Some("id1".to_string()),
                file_region: None,
            },
            LicenseDetection {
                license_expression: Some("mit".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "mit",
                    "2-aho",
                    100,
                    110,
                    85.0,
                    100,
                    100,
                    85.0,
                    100,
                    "mit.LICENSE",
                )],
                detection_log: Vec::new(),
                identifier: Some("id2".to_string()),
                file_region: None,
            },
        ];

        let result = remove_duplicate_detections(detections);
        assert_eq!(
            result.len(),
            2,
            "Different identifiers should be kept separate"
        );
    }

    #[test]
    fn test_remove_duplicate_detections_empty() {
        let detections: Vec<LicenseDetection> = vec![];
        let result = remove_duplicate_detections(detections);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_rank_detections_by_score() {
        let detections = vec![
            LicenseDetection {
                license_expression: Some("mit".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "mit",
                    "2-aho",
                    1,
                    10,
                    85.0,
                    100,
                    100,
                    85.0,
                    100,
                    "mit.LICENSE",
                )],
                detection_log: Vec::new(),
                identifier: Some("id2".to_string()),
                file_region: None,
            },
            LicenseDetection {
                license_expression: Some("apache-2.0".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "apache-2.0",
                    "1-spdx-id",
                    1,
                    10,
                    90.0,
                    100,
                    100,
                    100.0,
                    100,
                    "#1",
                )],
                detection_log: Vec::new(),
                identifier: Some("id1".to_string()),
                file_region: None,
            },
        ];

        let result = rank_detections(detections);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].license_expression, Some("apache-2.0".to_string()));
        assert_eq!(result[1].license_expression, Some("mit".to_string()));
    }

    #[test]
    fn test_rank_detections_by_coverage_when_scores_equal() {
        let detections = vec![
            LicenseDetection {
                license_expression: Some("mit".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "mit",
                    "2-aho",
                    1,
                    10,
                    90.0,
                    100,
                    100,
                    85.0,
                    100,
                    "mit.LICENSE",
                )],
                detection_log: Vec::new(),
                identifier: Some("id2".to_string()),
                file_region: None,
            },
            LicenseDetection {
                license_expression: Some("apache-2.0".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "apache-2.0",
                    "1-spdx-id",
                    1,
                    10,
                    90.0,
                    100,
                    100,
                    90.0,
                    100,
                    "#1",
                )],
                detection_log: Vec::new(),
                identifier: Some("id1".to_string()),
                file_region: None,
            },
        ];

        let result = rank_detections(detections);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].license_expression, Some("apache-2.0".to_string()));
    }

    #[test]
    fn test_rank_detections_empty() {
        let detections: Vec<LicenseDetection> = vec![];
        let result = rank_detections(detections);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_compute_detection_identifier_deterministic() {
        let detection = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: None,
            matches: vec![create_test_match_with_params(
                "mit",
                "1-hash",
                1,
                10,
                95.0,
                100,
                100,
                100.0,
                100,
                "mit.LICENSE",
            )],
            detection_log: Vec::new(),
            identifier: None,
            file_region: None,
        };

        let id1 = compute_detection_identifier(&detection);
        let id2 = compute_detection_identifier(&detection);
        assert_eq!(id1, id2, "Identifier should be deterministic");
        assert!(
            id1.starts_with("mit-"),
            "Identifier should start with expression"
        );
    }

    #[test]
    fn test_compute_detection_identifier_different_content() {
        let detection1 = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: None,
            matches: vec![create_test_match_with_params(
                "mit",
                "1-hash",
                1,
                10,
                95.0,
                100,
                100,
                100.0,
                100,
                "mit.LICENSE",
            )],
            detection_log: Vec::new(),
            identifier: None,
            file_region: None,
        };

        let detection2 = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: None,
            matches: vec![create_test_match_with_params(
                "mit",
                "2-aho",
                100,
                110,
                85.0,
                100,
                100,
                85.0,
                100,
                "mit.LICENSE",
            )],
            detection_log: Vec::new(),
            identifier: None,
            file_region: None,
        };

        let id1 = compute_detection_identifier(&detection1);
        let id2 = compute_detection_identifier(&detection2);
        assert_ne!(
            id1, id2,
            "Different content should produce different identifiers"
        );
    }

    #[test]
    fn test_python_safe_name() {
        assert_eq!(python_safe_name("mit"), "mit");
        assert_eq!(python_safe_name("gpl-2.0"), "gpl_2_0");
        assert_eq!(python_safe_name("apache-2.0"), "apache_2_0");
        assert_eq!(python_safe_name(""), "");
        assert_eq!(python_safe_name("---"), "");
    }

    #[test]
    fn test_get_matcher_priority() {
        assert_eq!(get_matcher_priority("1-spdx-id"), 1);
        assert_eq!(get_matcher_priority("1-hash"), 2);
        assert_eq!(get_matcher_priority("2-aho"), 3);
        assert_eq!(get_matcher_priority("3-seq-1"), 4);
        assert_eq!(get_matcher_priority("3-seq-2"), 4);
        assert_eq!(get_matcher_priority("5-unknown"), 5);
    }

    #[test]
    fn test_apply_detection_preferences_prefers_spdx_over_hash() {
        let detections = vec![
            LicenseDetection {
                license_expression: Some("mit".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "mit",
                    "1-hash",
                    1,
                    10,
                    90.0,
                    100,
                    100,
                    100.0,
                    100,
                    "mit.LICENSE",
                )],
                detection_log: Vec::new(),
                identifier: Some("hash".to_string()),
                file_region: None,
            },
            LicenseDetection {
                license_expression: Some("mit".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "mit",
                    "1-spdx-id",
                    1,
                    10,
                    90.0,
                    100,
                    100,
                    100.0,
                    100,
                    "#42",
                )],
                detection_log: Vec::new(),
                identifier: Some("spdx".to_string()),
                file_region: None,
            },
        ];

        let result = apply_detection_preferences(detections);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].identifier, Some("spdx".to_string()));
    }

    #[test]
    fn test_apply_detection_preferences_score_dominates() {
        let detections = vec![
            LicenseDetection {
                license_expression: Some("mit".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "mit",
                    "1-spdx-id",
                    1,
                    10,
                    85.0,
                    100,
                    100,
                    85.0,
                    100,
                    "#42",
                )],
                detection_log: Vec::new(),
                identifier: Some("spdx".to_string()),
                file_region: None,
            },
            LicenseDetection {
                license_expression: Some("mit".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "mit",
                    "1-hash",
                    1,
                    10,
                    95.0,
                    100,
                    100,
                    100.0,
                    100,
                    "mit.LICENSE",
                )],
                detection_log: Vec::new(),
                identifier: Some("hash".to_string()),
                file_region: None,
            },
        ];

        let result = apply_detection_preferences(detections);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].identifier, Some("hash".to_string()));
    }

    #[test]
    fn test_apply_detection_preferences_different_expressions() {
        let detections = vec![
            LicenseDetection {
                license_expression: Some("mit".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "mit",
                    "1-hash",
                    1,
                    10,
                    90.0,
                    100,
                    100,
                    100.0,
                    100,
                    "mit.LICENSE",
                )],
                detection_log: Vec::new(),
                identifier: Some("mit".to_string()),
                file_region: None,
            },
            LicenseDetection {
                license_expression: Some("apache-2.0".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "apache-2.0",
                    "1-spdx-id",
                    1,
                    10,
                    90.0,
                    100,
                    100,
                    100.0,
                    100,
                    "#1",
                )],
                detection_log: Vec::new(),
                identifier: Some("apache".to_string()),
                file_region: None,
            },
        ];

        let result = apply_detection_preferences(detections);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_apply_detection_preferences_empty() {
        let detections: Vec<LicenseDetection> = vec![];
        let result = apply_detection_preferences(detections);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_post_process_detections_full_pipeline() {
        let detections = vec![
            LicenseDetection {
                license_expression: Some("mit".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "mit",
                    "1-hash",
                    1,
                    10,
                    95.0,
                    100,
                    100,
                    100.0,
                    100,
                    "mit.LICENSE",
                )],
                detection_log: Vec::new(),
                identifier: Some("id1".to_string()),
                file_region: None,
            },
            LicenseDetection {
                license_expression: Some("mit".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "mit",
                    "2-aho",
                    1,
                    10,
                    85.0,
                    100,
                    100,
                    85.0,
                    100,
                    "mit.LICENSE",
                )],
                detection_log: Vec::new(),
                identifier: Some("id2".to_string()),
                file_region: None,
            },
            LicenseDetection {
                license_expression: Some("mit".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "mit",
                    "5-unknown",
                    1,
                    10,
                    30.0,
                    100,
                    100,
                    30.0,
                    50,
                    "unknown.LICENSE",
                )],
                detection_log: Vec::new(),
                identifier: Some("id3".to_string()),
                file_region: None,
            },
            LicenseDetection {
                license_expression: Some("apache-2.0".to_string()),
                license_expression_spdx: None,
                matches: vec![create_test_match_with_params(
                    "apache-2.0",
                    "1-spdx-id",
                    1,
                    10,
                    92.0,
                    100,
                    100,
                    100.0,
                    100,
                    "#1",
                )],
                detection_log: Vec::new(),
                identifier: Some("id4".to_string()),
                file_region: None,
            },
        ];

        let result = post_process_detections(detections, 90.0);
        assert_eq!(result.len(), 2);
        assert!(result[0].matches[0].matcher.starts_with("1-"));
        assert!(result[1].matches[0].matcher.starts_with("1-"));
    }

    #[test]
    fn test_post_process_detections_all_filtered() {
        let detections = vec![LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: None,
            matches: vec![create_test_match_with_params(
                "mit",
                "2-aho",
                1,
                10,
                30.0,
                100,
                100,
                30.0,
                50,
                "mit.LICENSE",
            )],
            detection_log: Vec::new(),
            identifier: Some("id1".to_string()),
            file_region: None,
        }];

        let result = post_process_detections(detections, 90.0);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_post_process_detections_empty() {
        let detections: Vec<LicenseDetection> = vec![];
        let result = post_process_detections(detections, 90.0);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_compute_detection_coverage_single() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "1-hash",
            1,
            10,
            95.0,
            100,
            100,
            95.0,
            100,
            "mit.LICENSE",
        )];

        let coverage = compute_detection_coverage(&matches);
        assert!((coverage - 95.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_detection_coverage_multiple_equal() {
        let matches = vec![
            create_test_match_with_params("mit", "1-hash", 1, 10, 95.0, 100, 100, 95.0, 100, "#1"),
            create_test_match_with_params(
                "apache-2.0",
                "1-spdx-id",
                11,
                20,
                85.0,
                100,
                100,
                85.0,
                100,
                "#2",
            ),
        ];

        let coverage = compute_detection_coverage(&matches);
        assert!((coverage - 90.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_detection_coverage_weighted() {
        let matches = vec![
            create_test_match_with_params("mit", "1-hash", 1, 10, 95.0, 200, 200, 95.0, 100, "#1"),
            create_test_match_with_params(
                "apache-2.0",
                "1-spdx-id",
                11,
                15,
                85.0,
                50,
                50,
                85.0,
                100,
                "#2",
            ),
        ];

        let coverage = compute_detection_coverage(&matches);
        assert!((coverage - 93.0).abs() < 0.1);
    }

    #[test]
    fn test_compute_detection_coverage_empty() {
        let matches: Vec<LicenseMatch> = vec![];
        let coverage = compute_detection_coverage(&matches);
        assert_eq!(coverage, 0.0);
    }

    #[test]
    fn test_compute_detection_coverage_capped_at_100() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "1-hash",
            1,
            10,
            100.0,
            100,
            100,
            100.0,
            100,
            "mit.LICENSE",
        )];

        let coverage = compute_detection_coverage(&matches);
        assert_eq!(coverage, 100.0);
    }

    #[test]
    fn test_detection_log_constants_match_python() {
        assert_eq!(DETECTION_LOG_PERFECT_DETECTION, "perfect-detection");
        assert_eq!(DETECTION_LOG_FALSE_POSITIVE, "possible-false-positive");
        assert_eq!(DETECTION_LOG_LICENSE_CLUES, "license-clues");
        assert_eq!(DETECTION_LOG_LOW_QUALITY_MATCHES, "low-quality-matches");
        assert_eq!(DETECTION_LOG_IMPERFECT_COVERAGE, "imperfect-match-coverage");
        assert_eq!(DETECTION_LOG_UNKNOWN_MATCH, "unknown-match");
        assert_eq!(DETECTION_LOG_EXTRA_WORDS, "extra-words");
        assert_eq!(DETECTION_LOG_UNDETECTED_LICENSE, "undetected-license");
        assert_eq!(
            DETECTION_LOG_UNKNOWN_INTRO_FOLLOWED_BY_MATCH,
            "unknown-intro-followed-by-match"
        );
        assert_eq!(
            DETECTION_LOG_UNKNOWN_REFERENCE_TO_LOCAL_FILE,
            "unknown-reference-to-local-file"
        );
    }

    #[test]
    fn test_is_undetected_license_matches_single_undetected() {
        let matches = vec![create_test_match_with_params(
            "unknown",
            "5-undetected",
            1,
            10,
            0.0,
            0,
            0,
            0.0,
            0,
            "undetected.LICENSE",
        )];

        assert!(is_undetected_license_matches(&matches));
    }

    #[test]
    fn test_is_undetected_license_matches_wrong_matcher() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "1-hash",
            1,
            10,
            95.0,
            100,
            100,
            100.0,
            100,
            "mit.LICENSE",
        )];

        assert!(!is_undetected_license_matches(&matches));
    }

    #[test]
    fn test_is_undetected_license_matches_multiple() {
        let matches = vec![
            create_test_match_with_params("mit", "1-hash", 1, 10, 95.0, 100, 100, 100.0, 100, "#1"),
            create_test_match_with_params(
                "apache-2.0",
                "1-spdx-id",
                11,
                20,
                85.0,
                100,
                100,
                85.0,
                100,
                "#2",
            ),
        ];

        assert!(!is_undetected_license_matches(&matches));
    }

    #[test]
    fn test_is_undetected_license_matches_empty() {
        let matches: Vec<LicenseMatch> = vec![];
        assert!(!is_undetected_license_matches(&matches));
    }

    #[test]
    fn test_analyze_detection_undetected() {
        let matches = vec![create_test_match_with_params(
            "unknown",
            "5-undetected",
            1,
            10,
            0.0,
            0,
            0,
            0.0,
            0,
            "undetected.LICENSE",
        )];

        assert_eq!(
            analyze_detection(&matches, false),
            DETECTION_LOG_UNDETECTED_LICENSE
        );
    }

    #[test]
    fn test_analyze_detection_perfect() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "1-hash",
            1,
            10,
            100.0,
            100,
            100,
            100.0,
            100,
            "mit.LICENSE",
        )];

        assert_eq!(
            analyze_detection(&matches, false),
            DETECTION_LOG_PERFECT_DETECTION
        );
    }

    #[test]
    fn test_analyze_detection_false_positive() {
        let matches = vec![create_test_match_with_params(
            "gpl",
            "2-aho",
            2000,
            2005,
            30.0,
            3,
            3,
            30.0,
            50,
            "gpl_bare.LICENSE",
        )];

        assert_eq!(
            analyze_detection(&matches, false),
            DETECTION_LOG_FALSE_POSITIVE
        );
    }

    #[test]
    fn test_analyze_detection_false_positive_ignored_for_package() {
        let matches = vec![create_test_match_with_params(
            "gpl",
            "2-aho",
            2000,
            2005,
            30.0,
            3,
            3,
            30.0,
            50,
            "gpl_bare.LICENSE",
        )];

        assert_ne!(
            analyze_detection(&matches, true),
            DETECTION_LOG_FALSE_POSITIVE
        );
    }

    #[test]
    fn test_analyze_detection_license_clues() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "2-aho",
            1,
            10,
            40.0,
            20,
            20,
            40.0,
            80,
            "mit.LICENSE",
        )];

        assert_eq!(
            analyze_detection(&matches, false),
            DETECTION_LOG_LICENSE_CLUES
        );
    }

    #[test]
    fn test_analyze_detection_unknown_match() {
        let matches = vec![create_test_match_with_params(
            "unknown",
            "5-unknown",
            1,
            10,
            80.0,
            50,
            50,
            80.0,
            100,
            "unknown.LICENSE",
        )];

        assert_eq!(
            analyze_detection(&matches, false),
            DETECTION_LOG_UNKNOWN_MATCH
        );
    }

    #[test]
    fn test_analyze_detection_imperfect_coverage() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "2-aho",
            1,
            10,
            85.0,
            100,
            100,
            85.0,
            100,
            "mit.LICENSE",
        )];

        assert_eq!(
            analyze_detection(&matches, false),
            DETECTION_LOG_IMPERFECT_COVERAGE
        );
    }

    #[test]
    fn test_analyze_detection_extra_words() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "2-aho",
            1,
            10,
            90.0,
            100,
            100,
            100.0,
            100,
            "mit.LICENSE",
        )];

        assert_eq!(
            analyze_detection(&matches, false),
            DETECTION_LOG_EXTRA_WORDS
        );
    }

    #[test]
    fn test_has_unknown_intro_before_detection_single_match_returns_false() {
        let intro = LicenseMatch {
            license_expression: "unknown".to_string(),
            license_expression_spdx: "unknown".to_string(),
            from_file: Some("test.txt".to_string()),
            start_line: 1,
            end_line: 2,
            start_token: 0,
            end_token: 0,
            matcher: "2-aho".to_string(),
            score: 100.0,
            matched_length: 5,
            match_coverage: 100.0,
            rule_relevance: 100,
            rule_identifier: "license-intro.LICENSE".to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: Some("Licensed under".to_string()),
            referenced_filenames: None,
            is_license_intro: true,
            is_license_clue: false,
            is_license_reference: false,
            is_license_tag: false,
            rule_length: 5,
            matched_token_positions: None,
            hilen: 2,
        };

        let matches = vec![intro];

        let result = has_unknown_intro_before_detection(&matches);
        assert!(!result, "Single match should return false");
    }

    #[test]
    fn test_has_unknown_intro_before_detection_post_loop_returns_true() {
        let intro = LicenseMatch {
            license_expression: "unknown".to_string(),
            license_expression_spdx: "unknown".to_string(),
            from_file: Some("test.txt".to_string()),
            start_line: 1,
            end_line: 2,
            start_token: 0,
            end_token: 0,
            matcher: "2-aho".to_string(),
            score: 100.0,
            matched_length: 5,
            match_coverage: 100.0,
            rule_relevance: 100,
            rule_identifier: "license-intro.LICENSE".to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: Some("Licensed under".to_string()),
            referenced_filenames: None,
            is_license_intro: true,
            is_license_clue: false,
            is_license_reference: false,
            is_license_tag: false,
            rule_length: 5,
            matched_token_positions: None,
            hilen: 2,
        };

        let low_coverage_match = LicenseMatch {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: Some("test.txt".to_string()),
            start_line: 3,
            end_line: 10,
            start_token: 5,
            end_token: 50,
            matcher: "2-aho".to_string(),
            score: 50.0,
            matched_length: 10,
            match_coverage: 50.0,
            rule_relevance: 100,
            rule_identifier: "mit.LICENSE".to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: Some("MIT License".to_string()),
            referenced_filenames: None,
            is_license_intro: false,
            is_license_clue: false,
            is_license_reference: false,
            is_license_tag: false,
            rule_length: 20,
            matched_token_positions: None,
            hilen: 5,
        };

        let matches = vec![intro, low_coverage_match];

        let result = has_unknown_intro_before_detection(&matches);
        assert!(
            result,
            "Unknown intro + low coverage match should return true"
        );
    }

    #[test]
    fn test_sort_detections_by_line() {
        fn create_detection(
            license_expr: &str,
            start_line: usize,
            end_line: usize,
        ) -> LicenseDetection {
            LicenseDetection {
                license_expression: Some(license_expr.to_string()),
                license_expression_spdx: Some(license_expr.to_uppercase()),
                matches: vec![LicenseMatch {
                    license_expression: license_expr.to_string(),
                    license_expression_spdx: license_expr.to_uppercase(),
                    from_file: None,
                    start_line,
                    end_line,
                    start_token: 0,
                    end_token: 0,
                    matcher: "1-hash".to_string(),
                    score: 0.95,
                    matched_length: 100,
                    match_coverage: 95.0,
                    rule_relevance: 100,
                    rule_identifier: format!("{}.LICENSE", license_expr),
                    rule_url: String::new(),
                    matched_text: None,
                    referenced_filenames: None,
                    is_license_intro: false,
                    is_license_clue: false,
                    is_license_reference: false,
                    is_license_tag: false,
                    rule_length: 100,
                    matched_token_positions: None,
                    hilen: 50,
                }],
                detection_log: vec![],
                identifier: None,
                file_region: None,
            }
        }

        let detection1 = create_detection("mit", 50, 60);
        let detection2 = create_detection("apache-2.0", 10, 20);
        let detection3 = create_detection("bsd-3-clause", 30, 40);

        let detections = vec![detection1.clone(), detection2.clone(), detection3.clone()];
        let sorted = sort_detections_by_line(detections);

        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].license_expression, Some("apache-2.0".to_string()));
        assert_eq!(
            sorted[1].license_expression,
            Some("bsd-3-clause".to_string())
        );
        assert_eq!(sorted[2].license_expression, Some("mit".to_string()));
    }

    #[test]
    fn test_sort_detections_by_line_empty_matches() {
        let detection = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: None,
            matches: vec![],
            detection_log: vec![],
            identifier: None,
            file_region: None,
        };

        let sorted = sort_detections_by_line(vec![detection.clone()]);
        assert_eq!(sorted.len(), 1);
    }

    #[test]
    fn test_is_unknown_intro_true_with_is_license_intro_flag() {
        let m = LicenseMatch {
            license_expression: "unknown".to_string(),
            license_expression_spdx: "unknown".to_string(),
            from_file: Some("test.txt".to_string()),
            start_line: 1,
            end_line: 5,
            start_token: 0,
            end_token: 0,
            matcher: "2-aho".to_string(),
            score: 100.0,
            matched_length: 10,
            match_coverage: 100.0,
            rule_relevance: 100,
            rule_identifier: "license-intro.LICENSE".to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: Some("Licensed under".to_string()),
            referenced_filenames: None,
            is_license_intro: true,
            is_license_clue: false,
            is_license_reference: false,
            is_license_tag: false,
            rule_length: 10,
            matched_token_positions: None,
            hilen: 5,
        };
        assert!(is_unknown_intro(&m));
    }

    #[test]
    fn test_is_unknown_intro_true_with_is_license_clue_flag() {
        let m = LicenseMatch {
            license_expression: "unknown".to_string(),
            license_expression_spdx: "unknown".to_string(),
            from_file: Some("test.txt".to_string()),
            start_line: 1,
            end_line: 5,
            start_token: 0,
            end_token: 0,
            matcher: "2-aho".to_string(),
            score: 100.0,
            matched_length: 10,
            match_coverage: 100.0,
            rule_relevance: 100,
            rule_identifier: "license-clue.LICENSE".to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: Some("Licensed under".to_string()),
            referenced_filenames: None,
            is_license_intro: false,
            is_license_clue: true,
            is_license_reference: false,
            is_license_tag: false,
            rule_length: 10,
            matched_token_positions: None,
            hilen: 5,
        };
        assert!(is_unknown_intro(&m));
    }

    #[test]
    fn test_is_unknown_intro_true_with_free_unknown_expression() {
        let m = LicenseMatch {
            license_expression: "free-unknown".to_string(),
            license_expression_spdx: "free-unknown".to_string(),
            from_file: Some("test.txt".to_string()),
            start_line: 1,
            end_line: 5,
            start_token: 0,
            end_token: 0,
            matcher: "2-aho".to_string(),
            score: 100.0,
            matched_length: 10,
            match_coverage: 100.0,
            rule_relevance: 100,
            rule_identifier: "free-unknown.LICENSE".to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: Some("Licensed under".to_string()),
            referenced_filenames: None,
            is_license_intro: false,
            is_license_clue: false,
            is_license_reference: false,
            is_license_tag: false,
            rule_length: 10,
            matched_token_positions: None,
            hilen: 5,
        };
        assert!(is_unknown_intro(&m));
    }

    #[test]
    fn test_is_unknown_intro_false_no_unknown_in_expression() {
        let m = LicenseMatch {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: Some("test.txt".to_string()),
            start_line: 1,
            end_line: 5,
            start_token: 0,
            end_token: 0,
            matcher: "1-hash".to_string(),
            score: 100.0,
            matched_length: 10,
            match_coverage: 100.0,
            rule_relevance: 100,
            rule_identifier: "mit.LICENSE".to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: Some("MIT License".to_string()),
            referenced_filenames: None,
            is_license_intro: true,
            is_license_clue: false,
            is_license_reference: false,
            is_license_tag: false,
            rule_length: 10,
            matched_token_positions: None,
            hilen: 5,
        };
        assert!(!is_unknown_intro(&m));
    }

    #[test]
    fn test_is_unknown_intro_false_no_flags_or_free_unknown() {
        let m = LicenseMatch {
            license_expression: "unknown".to_string(),
            license_expression_spdx: "unknown".to_string(),
            from_file: Some("test.txt".to_string()),
            start_line: 1,
            end_line: 5,
            start_token: 0,
            end_token: 0,
            matcher: "5-unknown".to_string(),
            score: 50.0,
            matched_length: 10,
            match_coverage: 50.0,
            rule_relevance: 50,
            rule_identifier: "unknown.LICENSE".to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: Some("Some text".to_string()),
            referenced_filenames: None,
            is_license_intro: false,
            is_license_clue: false,
            is_license_reference: false,
            is_license_tag: false,
            rule_length: 10,
            matched_token_positions: None,
            hilen: 5,
        };
        assert!(!is_unknown_intro(&m));
    }

    fn create_test_match_with_reference(
        start_line: usize,
        end_line: usize,
        referenced_filenames: Option<Vec<String>>,
    ) -> LicenseMatch {
        LicenseMatch {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: Some("test.txt".to_string()),
            start_line,
            end_line,
            start_token: 0,
            end_token: 0,
            matcher: "1-hash".to_string(),
            score: 95.0,
            matched_length: 100,
            match_coverage: 95.0,
            rule_relevance: 100,
            rule_identifier: "mit.LICENSE".to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: Some("MIT License".to_string()),
            referenced_filenames,
            is_license_intro: false,
            is_license_clue: false,
            is_license_reference: false,
            is_license_tag: false,
            rule_length: 100,
            matched_token_positions: None,
            hilen: 50,
        }
    }

    #[test]
    fn test_is_license_reference_local_file_true() {
        let m = create_test_match_with_reference(1, 5, Some(vec!["LICENSE".to_string()]));
        assert!(is_license_reference_local_file(&m));
    }

    #[test]
    fn test_is_license_reference_local_file_true_multiple() {
        let m = create_test_match_with_reference(
            1,
            5,
            Some(vec!["LICENSE".to_string(), "COPYING".to_string()]),
        );
        assert!(is_license_reference_local_file(&m));
    }

    #[test]
    fn test_is_license_reference_local_file_false_empty() {
        let m = create_test_match_with_reference(1, 5, Some(vec![]));
        assert!(!is_license_reference_local_file(&m));
    }

    #[test]
    fn test_is_license_reference_local_file_false_none() {
        let m = create_test_match_with_reference(1, 5, None);
        assert!(!is_license_reference_local_file(&m));
    }

    #[test]
    fn test_filter_license_references_filters_matches() {
        let m1 = create_test_match_with_reference(1, 5, None);
        let m2 = create_test_match_with_reference(6, 10, Some(vec!["LICENSE".to_string()]));
        let m3 = create_test_match_with_reference(11, 15, None);
        let matches = vec![m1.clone(), m2, m3.clone()];

        let filtered = filter_license_references(&matches);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].start_line, 1);
        assert_eq!(filtered[1].start_line, 11);
    }

    #[test]
    fn test_filter_license_references_returns_original_when_empty() {
        let m1 = create_test_match_with_reference(1, 5, Some(vec!["LICENSE".to_string()]));
        let m2 = create_test_match_with_reference(6, 10, Some(vec!["COPYING".to_string()]));
        let matches = vec![m1, m2];

        let filtered = filter_license_references(&matches);

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_license_references_no_filtering_needed() {
        let m1 = create_test_match_with_reference(1, 5, None);
        let m2 = create_test_match_with_reference(6, 10, None);
        let matches = vec![m1.clone(), m2.clone()];

        let filtered = filter_license_references(&matches);

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_license_intros_and_references_filters_both() {
        let mut m1 = create_test_match_with_reference(1, 5, None);
        m1.is_license_intro = true;
        m1.matcher = "2-aho".to_string();
        m1.match_coverage = 100.0;

        let m2 = create_test_match_with_reference(6, 10, Some(vec!["LICENSE".to_string()]));
        let m3 = create_test_match_with_reference(11, 15, None);
        let matches = vec![m1, m2, m3.clone()];

        let filtered = filter_license_intros_and_references(&matches);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].start_line, 11);
    }

    #[test]
    fn test_create_detection_from_group_unknown_reference_filters() {
        let m1 = create_test_match_with_reference(1, 5, None);
        let mut m2 = create_test_match_with_reference(6, 10, Some(vec!["LICENSE".to_string()]));
        m2.match_coverage = 100.0;
        m2.matcher = "1-hash".to_string();
        m2.score = 100.0;

        let group = DetectionGroup {
            matches: vec![m1.clone(), m2],
            start_line: 1,
            end_line: 10,
        };

        let detection = create_detection_from_group(&group);

        assert_eq!(
            detection.detection_log,
            vec!["unknown-reference-to-local-file"]
        );
        assert_eq!(detection.matches.len(), 1);
        assert_eq!(detection.matches[0].start_line, 1);
    }
}
