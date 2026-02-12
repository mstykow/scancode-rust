//! License detection assembly and grouping logic.
//!
//! This module implements Phase 6 of the license detection pipeline:
//! grouping raw matches into LicenseDetection objects based on proximity
//! and applying heuristics.

use crate::license_detection::expression::{CombineRelation, combine_expressions};
use crate::license_detection::models::LicenseMatch;
use crate::license_detection::spdx_mapping::SpdxMapping;

/// Proximity threshold for grouping matches in lines.
/// Matches more than this many lines apart are considered separate regions.
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
/// * `proximity_threshold` - Maximum line gap between matches to be in the same group
///
/// # Returns
///
/// A vector of DetectionGroup objects, each containing matches that form a region
fn group_matches_by_region_with_threshold(
    matches: &[LicenseMatch],
    proximity_threshold: usize,
) -> Vec<DetectionGroup> {
    let mut groups = Vec::new();
    let mut current_group: Vec<LicenseMatch> = Vec::new();

    for match_item in matches {
        if current_group.is_empty() {
            current_group.push(match_item.clone());
            continue;
        }

        let previous_match = current_group.last().unwrap();
        let is_in_group_by_threshold =
            match_item.start_line <= previous_match.end_line + proximity_threshold;

        if previous_match.matcher.starts_with("5-unknown") && is_license_intro_match(previous_match)
        {
            current_group.push(match_item.clone());
        } else if is_license_intro_match(match_item) {
            if !current_group.is_empty() {
                groups.push(DetectionGroup::new(current_group.clone()));
            }
            current_group = vec![match_item.clone()];
        } else if is_license_clue_match(match_item) {
            if !current_group.is_empty() {
                groups.push(DetectionGroup::new(current_group.clone()));
            }
            groups.push(DetectionGroup::new(vec![match_item.clone()]));
            current_group = Vec::new();
        } else if is_in_group_by_threshold {
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

/// Sort matches by start line for grouping.
pub fn sort_matches_by_line(matches: &mut [LicenseMatch]) {
    matches.sort_by(|a, b| {
        a.start_line
            .cmp(&b.start_line)
            .then_with(|| a.end_line.cmp(&b.end_line))
    });
}

/// Check if a match is a license intro.
fn is_license_intro_match(match_item: &LicenseMatch) -> bool {
    match_item.matcher.starts_with("5-unknown") || match_item.rule_identifier.contains("intro")
}

/// Check if a match is a license clue.
fn is_license_clue_match(match_item: &LicenseMatch) -> bool {
    match_item.matcher == "5-unknown" || match_item.rule_identifier.contains("clue")
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

    let all_short = matches
        .iter()
        .all(|m| m.matched_length <= FALSE_POSITIVE_RULE_LENGTH_THRESHOLD);

    let all_low_relevance = matches.iter().all(|m| m.rule_relevance < 60);

    let is_single = matches.len() == 1;

    if is_single && is_bare_rule && all_low_relevance {
        return true;
    }

    if is_gpl && all_short {
        return true;
    }

    if all_low_relevance && start_line > FALSE_POSITIVE_START_LINE_THRESHOLD && all_short {
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

    false
}

/// Check if a match is an unknown license intro.
///
/// A license intro is typically a short statement introducing a license,
/// often matched by the unknown matcher.
fn is_unknown_intro(m: &LicenseMatch) -> bool {
    m.matcher.starts_with("5-unknown") && m.rule_identifier.contains("intro")
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
    if group.matches.is_empty() {
        return;
    }

    detection.matches = group.matches.clone();

    let _score = compute_detection_score(&detection.matches);

    if let Ok(scancode_expr) = determine_license_expression(&detection.matches) {
        detection.license_expression = Some(scancode_expr.clone());

        if let Ok(spdx_expr) = determine_spdx_expression_from_scancode(&scancode_expr, spdx_mapping)
        {
            detection.license_expression_spdx = Some(spdx_expr);
        }
    }

    let log_category = analyze_detection(&detection.matches, false);
    detection.detection_log.push(log_category.to_string());

    detection.identifier = None;
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
    if group.matches.is_empty() {
        return LicenseDetection {
            license_expression: None,
            license_expression_spdx: None,
            matches: Vec::new(),
            detection_log: Vec::new(),
            identifier: None,
            file_region: None,
        };
    }

    let mut detection = LicenseDetection {
        license_expression: None,
        license_expression_spdx: None,
        matches: Vec::new(),
        detection_log: Vec::new(),
        identifier: None,
        file_region: None,
    };

    populate_detection_from_group(&mut detection, group);

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

/// Remove duplicate detections (same license expression).
///
/// When multiple detections have the same license_expression, keeps only
/// the one with the highest score. If scores are equal, keeps the first one.
///
/// Based on Python deduplication logic in detection.py.
pub fn remove_duplicate_detections(detections: Vec<LicenseDetection>) -> Vec<LicenseDetection> {
    let mut unique_detections: std::collections::HashMap<String, LicenseDetection> =
        std::collections::HashMap::new();

    for detection in detections {
        let expr = detection
            .license_expression
            .clone()
            .unwrap_or_else(String::new);

        let score = compute_detection_score(&detection.matches);
        let should_keep = unique_detections
            .get(&expr)
            .map(|existing| score >= compute_detection_score(&existing.matches))
            .unwrap_or(true);

        if should_keep {
            unique_detections.insert(expr, detection);
        }
    }

    unique_detections.into_values().collect()
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
    rank_detections(preferred)
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
            matcher: matcher.to_string(),
            score: 0.95,
            matched_length: 100,
            match_coverage: 95.0,
            rule_relevance: 100,
            rule_identifier: rule_identifier.to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: Some("MIT License".to_string()),
            referenced_filenames: None,
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
    fn test_group_matches_exactly_at_threshold() {
        let match1 = create_test_match(1, 5, "1-hash", "mit.LICENSE");
        let match2 = create_test_match(9, 13, "2-aho", "mit.LICENSE");
        let matches = vec![match1, match2];
        let groups = group_matches_by_region(&matches);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].matches.len(), 2);
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

    #[allow(clippy::too_many_arguments)]
    fn create_test_match_with_params(
        license_expression: &str,
        matcher: &str,
        start_line: usize,
        end_line: usize,
        score: f32,
        matched_length: usize,
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
            matcher: matcher.to_string(),
            score,
            matched_length,
            match_coverage,
            rule_relevance,
            rule_identifier: rule_identifier.to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: Some("License text".to_string()),
            referenced_filenames: None,
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
            100.0,
            100,
            "gpl-2.0.LICENSE",
        )];

        assert!(is_correctDetection(&matches));
    }

    #[test]
    fn test_is_correct_detection_multiple_perfect() {
        let matches = vec![
            create_test_match_with_params("mit", "1-hash", 1, 10, 95.0, 100, 100.0, 100, "#1"),
            create_test_match_with_params(
                "apache-2.0",
                "1-spdx-id",
                11,
                20,
                95.0,
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
            30.0,
            50,
            "gpl_bare.LICENSE",
        )];

        assert!(is_false_positive(&matches));
    }

    #[test]
    fn test_is_false_positive_gpl_short() {
        let matches = vec![create_test_match_with_params(
            "gpl-2.0",
            "2-aho",
            1,
            10,
            50.0,
            2,
            50.0,
            100,
            "gpl-2.0.LICENSE",
        )];

        assert!(is_false_positive(&matches));
    }

    #[test]
    fn test_is_false_positive_late_short_low_relevance() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "2-aho",
            1500,
            1505,
            30.0,
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
    fn test_is_low_quality_matches_low_coverage() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "2-aho",
            1,
            10,
            40.0,
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
            create_test_match_with_params("mit", "1-hash", 1, 10, 95.0, 100, 95.0, 100, "#1"),
            create_test_match_with_params(
                "apache-2.0",
                "1-spdx-id",
                11,
                20,
                85.0,
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
            create_test_match_with_params("mit", "1-hash", 1, 10, 95.0, 200, 95.0, 100, "#1"),
            create_test_match_with_params(
                "apache-2.0",
                "1-spdx-id",
                11,
                15,
                85.0,
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
            create_test_match_with_params("mit", "1-hash", 1, 10, 95.0, 100, 95.0, 100, "#1"),
            create_test_match_with_params(
                "apache-2.0",
                "1-spdx-id",
                11,
                20,
                85.0,
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
            create_test_match_with_params("mit", "1-hash", 1, 10, 95.0, 100, 95.0, 100, "#1"),
            create_test_match_with_params(
                "apache-2.0",
                "1-spdx-id",
                11,
                20,
                85.0,
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
            category: Some("Permissive".to_string()),
            text: "MIT License text...".to_string(),
            reference_urls: vec![],
            notes: None,
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
                category: Some("Permissive".to_string()),
                text: "MIT License text...".to_string(),
                reference_urls: vec![],
                notes: None,
            },
            License {
                key: "apache-2.0".to_string(),
                name: "Apache License 2.0".to_string(),
                spdx_license_key: Some("Apache-2.0".to_string()),
                category: Some("Permissive".to_string()),
                text: "Apache License text...".to_string(),
                reference_urls: vec![],
                notes: None,
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
            category: Some("Unstated License".to_string()),
            text: "Custom license text...".to_string(),
            reference_urls: vec![],
            notes: None,
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
            category: Some("Permissive".to_string()),
            text: "MIT License text...".to_string(),
            reference_urls: vec![],
            notes: None,
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
                category: Some("Permissive".to_string()),
                text: "MIT License text...".to_string(),
                reference_urls: vec![],
                notes: None,
            },
            License {
                key: "apache-2.0".to_string(),
                name: "Apache License 2.0".to_string(),
                spdx_license_key: Some("Apache-2.0".to_string()),
                category: Some("Permissive".to_string()),
                text: "Apache License text...".to_string(),
                reference_urls: vec![],
                notes: None,
            },
        ];
        let mapping = build_spdx_mapping(&licenses);

        let group = DetectionGroup {
            matches: vec![
                create_test_match_with_params("mit", "1-hash", 1, 10, 100.0, 100, 100.0, 100, "#1"),
                create_test_match_with_params(
                    "apache-2.0",
                    "1-spdx-id",
                    11,
                    20,
                    100.0,
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
            category: Some("Unstated License".to_string()),
            text: "Custom license text...".to_string(),
            reference_urls: vec![],
            notes: None,
        }];
        let mapping = build_spdx_mapping(&licenses);

        let group = DetectionGroup {
            matches: vec![create_test_match_with_params(
                "custom-1",
                "2-aho",
                1,
                10,
                95.0,
                100,
                95.0,
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
    fn test_remove_duplicate_detections_same_expression_keeps_best() {
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
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].license_expression, Some("mit".to_string()));
        assert_eq!(result[0].identifier, Some("id1".to_string()));
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
            create_test_match_with_params("mit", "1-hash", 1, 10, 95.0, 100, 95.0, 100, "#1"),
            create_test_match_with_params(
                "apache-2.0",
                "1-spdx-id",
                11,
                20,
                85.0,
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
            create_test_match_with_params("mit", "1-hash", 1, 10, 95.0, 200, 95.0, 100, "#1"),
            create_test_match_with_params(
                "apache-2.0",
                "1-spdx-id",
                11,
                15,
                85.0,
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
            100.0,
            100,
            "mit.LICENSE",
        )];

        assert!(!is_undetected_license_matches(&matches));
    }

    #[test]
    fn test_is_undetected_license_matches_multiple() {
        let matches = vec![
            create_test_match_with_params("mit", "1-hash", 1, 10, 95.0, 100, 100.0, 100, "#1"),
            create_test_match_with_params(
                "apache-2.0",
                "1-spdx-id",
                11,
                20,
                85.0,
                100,
                100.0,
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
    fn test_has_unknown_intro_before_detection_true() {
        let intro = LicenseMatch {
            license_expression: "unknown".to_string(),
            license_expression_spdx: "unknown".to_string(),
            from_file: Some("test.txt".to_string()),
            start_line: 1,
            end_line: 2,
            matcher: "5-unknown".to_string(),
            score: 50.0,
            matched_length: 5,
            match_coverage: 100.0,
            rule_relevance: 100,
            rule_identifier: "license-intro.LICENSE".to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: Some("Licensed under".to_string()),
            referenced_filenames: None,
        };
        let license_match = create_test_match_with_params(
            "mit",
            "1-hash",
            3,
            10,
            100.0,
            100,
            100.0,
            100,
            "mit.LICENSE",
        );

        let matches = vec![intro, license_match];
        assert!(has_unknown_intro_before_detection(&matches));
    }

    #[test]
    fn test_has_unknown_intro_before_detection_false_single_match() {
        let matches = vec![create_test_match_with_params(
            "mit",
            "1-hash",
            1,
            10,
            100.0,
            100,
            100.0,
            100,
            "mit.LICENSE",
        )];

        assert!(!has_unknown_intro_before_detection(&matches));
    }

    #[test]
    fn test_has_unknown_intro_before_detection_false_all_intros() {
        let intro1 = LicenseMatch {
            license_expression: "unknown".to_string(),
            license_expression_spdx: "unknown".to_string(),
            from_file: Some("test.txt".to_string()),
            start_line: 1,
            end_line: 2,
            matcher: "5-unknown".to_string(),
            score: 50.0,
            matched_length: 5,
            match_coverage: 100.0,
            rule_relevance: 100,
            rule_identifier: "license-intro.LICENSE".to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: Some("Licensed under".to_string()),
            referenced_filenames: None,
        };
        let intro2 = LicenseMatch {
            license_expression: "unknown".to_string(),
            license_expression_spdx: "unknown".to_string(),
            from_file: Some("test.txt".to_string()),
            start_line: 3,
            end_line: 4,
            matcher: "5-unknown".to_string(),
            score: 50.0,
            matched_length: 5,
            match_coverage: 100.0,
            rule_relevance: 100,
            rule_identifier: "license-intro-2.LICENSE".to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: Some("See LICENSE file".to_string()),
            referenced_filenames: None,
        };

        let matches = vec![intro1, intro2];
        assert!(!has_unknown_intro_before_detection(&matches));
    }

    #[test]
    fn test_populate_detection_from_group_uses_analyze_detection() {
        let group = DetectionGroup {
            matches: vec![create_test_match_with_params(
                "mit",
                "1-hash",
                1,
                10,
                100.0,
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

        assert!(
            detection
                .detection_log
                .contains(&"perfect-detection".to_string())
        );
    }

    #[test]
    fn test_populate_detection_from_group_undetected() {
        let group = DetectionGroup {
            matches: vec![create_test_match_with_params(
                "unknown",
                "5-undetected",
                1,
                10,
                0.0,
                0,
                0.0,
                0,
                "undetected.LICENSE",
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

        assert!(
            detection
                .detection_log
                .contains(&"undetected-license".to_string())
        );
    }
}
