//! License detection assembly and grouping logic.
//!
//! This module implements Phase 6 of the license detection pipeline:
//! grouping raw matches into LicenseDetection objects based on proximity

pub mod analysis;
pub mod grouping;
pub mod identifier;
mod types;

pub use grouping::{group_matches_by_region, sort_matches_by_line};
pub use types::{DetectionGroup, LicenseDetection};

use crate::license_detection::spdx_mapping::SpdxMapping;

use analysis::{
    analyze_detection, classify_detection, compute_detection_score, determine_license_expression,
    determine_spdx_expression, determine_spdx_expression_from_scancode, filter_license_intros,
    filter_license_intros_and_references,
};
#[cfg(test)]
use identifier::compute_detection_identifier;
use identifier::{compute_content_identifier, compute_detection_coverage, python_safe_name};

/// Matches with line gap > this are considered separate groups.
/// Corresponds to Python's LINES_THRESHOLD = 4 (query.py:108)
const LINES_THRESHOLD: usize = 4;

// ============================================================================
// Detection Log Categories (Python parity: DetectionRule enum)
// ============================================================================

/// License clues - low quality matches.
pub const DETECTION_LOG_LICENSE_CLUES: &str = "license-clues";

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

    let log_category = analyze_detection(&group.matches, false);

    let matches_for_expression = if log_category == DETECTION_LOG_UNKNOWN_INTRO_FOLLOWED_BY_MATCH {
        filter_license_intros(&group.matches)
    } else if log_category == DETECTION_LOG_UNKNOWN_REFERENCE_TO_LOCAL_FILE {
        filter_license_intros_and_references(&group.matches)
    } else {
        group.matches.clone()
    };

    detection.matches = group.matches.clone();

    let _score = compute_detection_score(&detection.matches);

    if let Ok(expr) = determine_license_expression(&matches_for_expression) {
        detection.license_expression = Some(expr.clone());

        if let Ok(spdx_expr) = determine_spdx_expression(&matches_for_expression) {
            detection.license_expression_spdx = Some(spdx_expr);
        }
    }

    detection.detection_log.push(log_category.to_string());

    // Compute identifier like Python: detection.identifier = detection.identifier_with_expression
    if let Some(ref expr) = detection.license_expression {
        let id_safe_expression = python_safe_name(expr);
        let content_uuid = compute_content_identifier(&detection.matches);
        detection.identifier = Some(format!("{}-{}", id_safe_expression, content_uuid));
    } else {
        detection.identifier = None;
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

    for match_item in &mut detection.matches {
        if match_item.license_expression_spdx.is_none()
            && let Ok(spdx_expr) =
                determine_spdx_expression_from_scancode(&match_item.license_expression, spdx_mapping)
        {
            match_item.license_expression_spdx = Some(spdx_expr);
        }
    }

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

    // Store RAW matches in detection.matches (matching Python behavior)
    // Python's LicenseDetection.from_matches() stores original unfiltered matches
    detection.matches = group.matches.clone();

    let _score = compute_detection_score(&detection.matches);

    // Use FILTERED matches for expression computation (matching Python behavior)
    if let Ok(expr) = determine_license_expression(&matches_for_expression) {
        detection.license_expression = Some(expr.clone());

        if let Ok(spdx_expr) = determine_spdx_expression(&matches_for_expression) {
            detection.license_expression_spdx = Some(spdx_expr);
        }
    }

    detection.detection_log.push(log_category.to_string());

    // Compute identifier like Python: detection.identifier = detection.identifier_with_expression
    if let Some(ref expr) = detection.license_expression {
        let id_safe_expression = python_safe_name(expr);
        let content_uuid = compute_content_identifier(&detection.matches);
        detection.identifier = Some(format!("{}-{}", id_safe_expression, content_uuid));
    } else {
        detection.identifier = None;
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
/// NOTE: This function is currently unused. Python aggregates detections into
/// unique detections with per-file region metadata, but Rust does not implement
/// that feature yet. See: `docs/license-detection/PLAN-019-file-region-and-unique-detection.md`.
///
/// Based on Python get_detections_by_id behavior in detection.py.
#[cfg(test)]
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
            .then_with(|| a.identifier.cmp(&b.identifier))
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
        min_line_a
            .cmp(&min_line_b)
            .then_with(|| a.identifier.cmp(&b.identifier))
    });
    detections
}

pub fn apply_detection_preferences(detections: Vec<LicenseDetection>) -> Vec<LicenseDetection> {
    detections
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
    // NOTE: We do NOT call remove_duplicate_detections here.
    //
    // Python's get_unique_detections() groups detections by identifier and creates
    // UniqueDetection objects with aggregated file_regions, but it does NOT remove
    // detections. The Python test infrastructure uses idx.match() which returns
    // raw matches without any deduplication.
    //
    // Calling remove_duplicate_detections would incorrectly merge detections that
    // have the same license expression at different locations (e.g., two bsd-new
    // licenses in different parts of a file). The identifier is computed from
    // license_expression + rule_identifier + score + matched_text_tokens, which
    // would be identical for same-license texts at different locations.
    //
    // TODO: Implement UniqueDetection with file_regions aggregation for output
    // formatting when we add full ScanCode output compatibility.
    let preferred = apply_detection_preferences(filtered);
    let ranked = rank_detections(preferred);
    sort_detections_by_line(ranked)
}

#[cfg(test)]
mod tests {
    use super::identifier::compute_detection_identifier;
    use super::*;
    use crate::license_detection::models::License;
    use crate::license_detection::models::LicenseMatch;
    use crate::license_detection::spdx_mapping::build_spdx_mapping;

    fn create_test_match(
        start_line: usize,
        end_line: usize,
        matcher: &str,
        rule_identifier: &str,
    ) -> LicenseMatch {
        LicenseMatch {
            rid: 0,
            license_expression: "mit".to_string(),
            license_expression_spdx: Some("MIT".to_string()),
            from_file: Some("test.txt".to_string()),
            start_line,
            end_line,
            start_token: 0,
            end_token: 0,
            matcher: matcher.parse().expect("invalid test matcher"),
            score: 95.0,
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
            is_license_text: false,
            is_from_license: false,
            rule_length: 100,
            matched_token_positions: None,
            hilen: 50,
            rule_start_token: 0,
            qspan_positions: None,
            ispan_positions: None,
            hispan_positions: None,
            candidate_resemblance: 0.0,
            candidate_containment: 0.0,
        }
    }

    fn create_perfect_match(start_line: usize, end_line: usize) -> LicenseMatch {
        let mut m = create_test_match(start_line, end_line, "1-hash", "mit.LICENSE");
        m.match_coverage = 100.0;
        m.score = 100.0;
        m
    }

    fn create_test_license() -> License {
        License {
            key: "mit".to_string(),
            name: "MIT License".to_string(),
            spdx_license_key: Some("MIT".to_string()),
            other_spdx_license_keys: vec![],
            category: Some("Permissive".to_string()),
            text: "MIT License".to_string(),
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
        }
    }

    #[test]
    fn test_create_detection_from_group_empty() {
        let group = DetectionGroup::new(Vec::new());
        let detection = create_detection_from_group(&group);
        assert!(detection.matches.is_empty());
        assert!(detection.license_expression.is_none());
    }

    #[test]
    fn test_create_detection_from_group_with_matches() {
        let match1 = create_perfect_match(1, 10);
        let group = DetectionGroup::new(vec![match1]);
        let detection = create_detection_from_group(&group);
        assert_eq!(detection.matches.len(), 1);
        assert!(detection.license_expression.is_some());
    }

    #[test]
    fn test_populate_detection_from_group_perfect() {
        let mut m = create_perfect_match(1, 10);
        m.match_coverage = 100.0;
        let group = DetectionGroup::new(vec![m]);
        let mut detection = LicenseDetection {
            license_expression: None,
            license_expression_spdx: None,
            matches: Vec::new(),
            detection_log: Vec::new(),
            identifier: None,
        };
        populate_detection_from_group(&mut detection, &group);
        assert_eq!(detection.matches.len(), 1);
        assert!(detection.license_expression.is_some());
        assert!(
            detection.detection_log.contains(&"".to_string()) || detection.detection_log.is_empty(),
            "Perfect detection has empty log"
        );
    }

    #[test]
    fn test_populate_detection_from_group_empty() {
        let group = DetectionGroup::new(Vec::new());
        let mut detection = LicenseDetection {
            license_expression: None,
            license_expression_spdx: None,
            matches: Vec::new(),
            detection_log: Vec::new(),
            identifier: None,
        };
        populate_detection_from_group(&mut detection, &group);
        assert!(detection.matches.is_empty());
        assert!(detection.license_expression.is_none());
    }

    #[test]
    fn test_populate_detection_from_group_false_positive() {
        let mut m = create_test_match(2000, 2005, "2-aho", "gpl_bare.LICENSE");
        m.rule_relevance = 50;
        m.score = 30.0;
        m.match_coverage = 30.0;
        m.rule_length = 3;
        let group = DetectionGroup::new(vec![m]);
        let mut detection = LicenseDetection {
            license_expression: None,
            license_expression_spdx: None,
            matches: Vec::new(),
            detection_log: Vec::new(),
            identifier: None,
        };
        populate_detection_from_group(&mut detection, &group);
        assert!(
            detection
                .detection_log
                .contains(&"false-positive".to_string())
        );
    }

    #[test]
    fn test_populate_detection_from_group_with_spdx_perfect() {
        let mut m = create_perfect_match(1, 10);
        m.license_expression = "mit".to_string();
        m.license_expression_spdx = Some("MIT".to_string());
        let group = DetectionGroup::new(vec![m]);
        let licenses = vec![create_test_license()];
        let spdx_mapping = build_spdx_mapping(&licenses);
        let mut detection = LicenseDetection {
            license_expression: None,
            license_expression_spdx: None,
            matches: Vec::new(),
            detection_log: Vec::new(),
            identifier: None,
        };
        populate_detection_from_group_with_spdx(&mut detection, &group, &spdx_mapping);
        assert!(detection.license_expression_spdx.is_some());
    }

    #[test]
    fn test_populate_detection_from_group_with_spdx_empty() {
        let group = DetectionGroup::new(Vec::new());
        let licenses = vec![create_test_license()];
        let spdx_mapping = build_spdx_mapping(&licenses);
        let mut detection = LicenseDetection {
            license_expression: None,
            license_expression_spdx: None,
            matches: Vec::new(),
            detection_log: Vec::new(),
            identifier: None,
        };
        populate_detection_from_group_with_spdx(&mut detection, &group, &spdx_mapping);
        assert!(detection.matches.is_empty());
    }

    #[test]
    fn test_filter_detections_by_score_all_pass() {
        let mut detection = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: Some("MIT".to_string()),
            matches: vec![create_perfect_match(1, 10)],
            detection_log: vec!["perfect-detection".to_string()],
            identifier: None,
        };
        detection.identifier = Some(compute_detection_identifier(&detection));
        let filtered = filter_detections_by_score(vec![detection], 0.0);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_detections_by_score_some_filtered() {
        let mut d1 = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: Some("MIT".to_string()),
            matches: vec![create_perfect_match(1, 10)],
            detection_log: vec!["perfect-detection".to_string()],
            identifier: None,
        };
        d1.identifier = Some(compute_detection_identifier(&d1));

        let mut m = create_test_match(1, 10, "2-aho", "gpl_bare.LICENSE");
        m.rule_relevance = 50;
        m.score = 30.0;
        m.match_coverage = 30.0;
        let mut d2 = LicenseDetection {
            license_expression: Some("gpl".to_string()),
            license_expression_spdx: Some("GPL".to_string()),
            matches: vec![m],
            detection_log: vec![],
            identifier: None,
        };
        d2.identifier = Some(compute_detection_identifier(&d2));

        let filtered = filter_detections_by_score(vec![d1, d2], 50.0);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_detections_by_score_all_filtered() {
        let mut m = create_test_match(1, 10, "2-aho", "gpl_bare.LICENSE");
        m.rule_relevance = 50;
        m.score = 30.0;
        m.match_coverage = 30.0;
        let mut detection = LicenseDetection {
            license_expression: Some("gpl".to_string()),
            license_expression_spdx: Some("GPL".to_string()),
            matches: vec![m],
            detection_log: vec![],
            identifier: None,
        };
        detection.identifier = Some(compute_detection_identifier(&detection));
        let filtered = filter_detections_by_score(vec![detection], 50.0);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_detections_by_score_empty() {
        let filtered = filter_detections_by_score(vec![], 0.0);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_remove_duplicate_detections_different_expressions() {
        let d1 = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: Some("MIT".to_string()),
            matches: vec![create_perfect_match(1, 10)],
            detection_log: vec![],
            identifier: Some("mit-abc123".to_string()),
        };
        let d2 = LicenseDetection {
            license_expression: Some("apache-2.0".to_string()),
            license_expression_spdx: Some("Apache-2.0".to_string()),
            matches: vec![create_perfect_match(20, 30)],
            detection_log: vec![],
            identifier: Some("apache-abc123".to_string()),
        };
        let result = remove_duplicate_detections(vec![d1, d2]);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_remove_duplicate_detections_same_expression_different_identifier() {
        let d1 = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: Some("MIT".to_string()),
            matches: vec![create_perfect_match(1, 10)],
            detection_log: vec![],
            identifier: Some("mit-abc123".to_string()),
        };
        let d2 = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: Some("MIT".to_string()),
            matches: vec![create_perfect_match(20, 30)],
            detection_log: vec![],
            identifier: Some("mit-def456".to_string()),
        };
        let result = remove_duplicate_detections(vec![d1, d2]);
        assert_eq!(
            result.len(),
            2,
            "Different identifiers should not be deduplicated"
        );
    }

    #[test]
    fn test_remove_duplicate_detections_empty() {
        let result = remove_duplicate_detections(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_rank_detections_by_score() {
        let mut d1 = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: Some("MIT".to_string()),
            matches: vec![create_perfect_match(1, 10)],
            detection_log: vec![],
            identifier: None,
        };
        let mut d2 = LicenseDetection {
            license_expression: Some("apache-2.0".to_string()),
            license_expression_spdx: Some("Apache-2.0".to_string()),
            matches: vec![{
                let mut m = create_test_match(20, 30, "1-hash", "apache.LICENSE");
                m.score = 80.0;
                m
            }],
            detection_log: vec![],
            identifier: None,
        };
        d1.identifier = Some(compute_detection_identifier(&d1));
        d2.identifier = Some(compute_detection_identifier(&d2));
        let ranked = rank_detections(vec![d2, d1]);
        assert_eq!(ranked[0].license_expression, Some("mit".to_string()));
    }

    #[test]
    fn test_rank_detections_by_coverage_when_scores_equal() {
        let mut m1 = create_test_match(1, 10, "1-hash", "mit.LICENSE");
        m1.score = 90.0;
        m1.match_coverage = 100.0;
        let mut d1 = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: Some("MIT".to_string()),
            matches: vec![m1],
            detection_log: vec![],
            identifier: None,
        };
        let mut m2 = create_test_match(20, 30, "1-hash", "apache.LICENSE");
        m2.score = 90.0;
        m2.match_coverage = 80.0;
        let mut d2 = LicenseDetection {
            license_expression: Some("apache-2.0".to_string()),
            license_expression_spdx: Some("Apache-2.0".to_string()),
            matches: vec![m2],
            detection_log: vec![],
            identifier: None,
        };
        d1.identifier = Some(compute_detection_identifier(&d1));
        d2.identifier = Some(compute_detection_identifier(&d2));
        let ranked = rank_detections(vec![d2, d1]);
        assert_eq!(
            ranked[0].license_expression,
            Some("mit".to_string()),
            "Higher coverage should rank first"
        );
    }

    #[test]
    fn test_rank_detections_empty() {
        let result = rank_detections(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_compute_detection_identifier_deterministic() {
        let m = create_perfect_match(1, 10);
        let d1 = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: Some("MIT".to_string()),
            matches: vec![m.clone()],
            detection_log: vec![],
            identifier: None,
        };
        let d2 = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: Some("MIT".to_string()),
            matches: vec![m],
            detection_log: vec![],
            identifier: None,
        };
        let id1 = compute_detection_identifier(&d1);
        let id2 = compute_detection_identifier(&d2);
        assert_eq!(id1, id2, "Same content should produce same identifier");
    }

    #[test]
    fn test_compute_detection_identifier_different_content() {
        let m1 = create_perfect_match(1, 10);
        let m2 = create_perfect_match(20, 30);
        let d1 = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: Some("MIT".to_string()),
            matches: vec![m1],
            detection_log: vec![],
            identifier: None,
        };
        let d2 = LicenseDetection {
            license_expression: Some("apache-2.0".to_string()),
            license_expression_spdx: Some("Apache-2.0".to_string()),
            matches: vec![m2],
            detection_log: vec![],
            identifier: None,
        };
        let id1 = compute_detection_identifier(&d1);
        let id2 = compute_detection_identifier(&d2);
        assert_ne!(
            id1, id2,
            "Different content should produce different identifiers"
        );
    }

    #[test]
    fn test_apply_detection_preferences_preserves_all_detections() {
        let d1 = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: Some("MIT".to_string()),
            matches: vec![create_perfect_match(1, 10)],
            detection_log: vec![],
            identifier: Some("mit-abc123".to_string()),
        };
        let d2 = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: Some("MIT".to_string()),
            matches: vec![create_perfect_match(20, 30)],
            detection_log: vec![],
            identifier: Some("mit-def456".to_string()),
        };
        let result = apply_detection_preferences(vec![d1, d2]);
        assert_eq!(
            result.len(),
            2,
            "Detections with same expression but different identifiers should be kept separate"
        );
    }

    #[test]
    fn test_apply_detection_preferences_different_expressions() {
        let d1 = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: Some("MIT".to_string()),
            matches: vec![create_perfect_match(1, 10)],
            detection_log: vec![],
            identifier: Some("mit-abc123".to_string()),
        };
        let d2 = LicenseDetection {
            license_expression: Some("apache-2.0".to_string()),
            license_expression_spdx: Some("Apache-2.0".to_string()),
            matches: vec![create_perfect_match(20, 30)],
            detection_log: vec![],
            identifier: Some("apache-abc123".to_string()),
        };
        let result = apply_detection_preferences(vec![d1, d2]);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_apply_detection_preferences_empty() {
        let result = apply_detection_preferences(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_post_process_detections_full_pipeline() {
        let m = create_perfect_match(1, 10);
        let mut d = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: Some("MIT".to_string()),
            matches: vec![m],
            detection_log: vec!["perfect-detection".to_string()],
            identifier: None,
        };
        d.identifier = Some(compute_detection_identifier(&d));
        let result = post_process_detections(vec![d], 0.0);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_post_process_detections_all_filtered() {
        let mut m = create_test_match(1, 10, "2-aho", "gpl_bare.LICENSE");
        m.rule_relevance = 50;
        m.score = 30.0;
        m.match_coverage = 30.0;
        let mut d = LicenseDetection {
            license_expression: Some("gpl".to_string()),
            license_expression_spdx: Some("GPL".to_string()),
            matches: vec![m],
            detection_log: vec![],
            identifier: None,
        };
        d.identifier = Some(compute_detection_identifier(&d));
        let result = post_process_detections(vec![d], 50.0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_post_process_detections_empty() {
        let result = post_process_detections(vec![], 0.0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_sort_detections_by_line() {
        let d1 = LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: Some("MIT".to_string()),
            matches: vec![create_perfect_match(20, 30)],
            detection_log: vec![],
            identifier: Some("mit-1".to_string()),
        };
        let d2 = LicenseDetection {
            license_expression: Some("apache-2.0".to_string()),
            license_expression_spdx: Some("Apache-2.0".to_string()),
            matches: vec![create_perfect_match(1, 10)],
            detection_log: vec![],
            identifier: Some("apache-1".to_string()),
        };
        let sorted = sort_detections_by_line(vec![d1, d2]);
        assert_eq!(sorted[0].matches[0].start_line, 1);
        assert_eq!(sorted[1].matches[0].start_line, 20);
    }

    #[test]
    fn test_determine_spdx_expression_from_scancode_single() {
        let licenses = vec![create_test_license()];
        let mapping = build_spdx_mapping(&licenses);
        let result = determine_spdx_expression_from_scancode("mit", &mapping);
        assert!(result.is_ok());
    }

    #[test]
    fn test_determine_spdx_expression_from_scancode_multiple() {
        let licenses = vec![create_test_license()];
        let mapping = build_spdx_mapping(&licenses);
        let result = determine_spdx_expression_from_scancode("mit AND apache-2.0", &mapping);
        assert!(result.is_ok());
    }

    #[test]
    fn test_determine_spdx_expression_from_scancode_empty() {
        let licenses = vec![create_test_license()];
        let mapping = build_spdx_mapping(&licenses);
        let result = determine_spdx_expression_from_scancode("", &mapping);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn test_determine_spdx_expression_from_scancode_custom_license() {
        let licenses = vec![create_test_license()];
        let mapping = build_spdx_mapping(&licenses);
        let result = determine_spdx_expression_from_scancode("custom-1", &mapping);
        assert!(result.is_ok());
    }

    #[test]
    fn test_populate_detection_from_group_generates_spdx_expression() {
        let mut m = create_perfect_match(1, 10);
        m.license_expression = "mit".to_string();
        m.license_expression_spdx = Some("MIT".to_string());
        let group = DetectionGroup::new(vec![m]);
        let licenses = vec![create_test_license()];
        let spdx_mapping = build_spdx_mapping(&licenses);
        let mut detection = LicenseDetection {
            license_expression: None,
            license_expression_spdx: None,
            matches: Vec::new(),
            detection_log: Vec::new(),
            identifier: None,
        };
        populate_detection_from_group_with_spdx(&mut detection, &group, &spdx_mapping);
        assert!(detection.license_expression_spdx.is_some());
    }

    #[test]
    fn test_populate_detection_from_group_with_spdx_multiple() {
        let mut m1 = create_perfect_match(1, 10);
        m1.license_expression = "mit".to_string();
        let mut m2 = create_perfect_match(11, 20);
        m2.license_expression = "apache-2.0".to_string();
        m2.license_expression_spdx = Some("Apache-2.0".to_string());
        let group = DetectionGroup::new(vec![m1, m2]);
        let licenses = vec![create_test_license()];
        let spdx_mapping = build_spdx_mapping(&licenses);
        let mut detection = LicenseDetection {
            license_expression: None,
            license_expression_spdx: None,
            matches: Vec::new(),
            detection_log: Vec::new(),
            identifier: None,
        };
        populate_detection_from_group_with_spdx(&mut detection, &group, &spdx_mapping);
        assert!(detection.license_expression.is_some());
    }

    #[test]
    fn test_populate_detection_from_group_with_spdx_custom_license() {
        let mut m = create_perfect_match(1, 10);
        m.license_expression = "custom-license".to_string();
        m.license_expression_spdx = Some("custom-license".to_string());
        let group = DetectionGroup::new(vec![m]);
        let licenses = vec![create_test_license()];
        let spdx_mapping = build_spdx_mapping(&licenses);
        let mut detection = LicenseDetection {
            license_expression: None,
            license_expression_spdx: None,
            matches: Vec::new(),
            detection_log: Vec::new(),
            identifier: None,
        };
        populate_detection_from_group_with_spdx(&mut detection, &group, &spdx_mapping);
        assert!(detection.license_expression.is_some());
    }

    #[test]
    fn test_create_detection_from_group_unknown_reference_filters() {
        let mut m = create_test_match(1, 10, "2-aho", "mit.LICENSE");
        m.is_license_reference = true;
        let group = DetectionGroup::new(vec![m]);
        let detection = create_detection_from_group(&group);
        assert_eq!(detection.matches.len(), 1);
    }
}
