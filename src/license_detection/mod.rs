//! License Detection Engine

pub mod aho_match;
mod detection;
#[cfg(test)]
mod investigation;


pub mod expression;
#[cfg(test)]
mod golden_test;
pub mod hash_match;
pub mod index;
mod match_refine;
pub mod models;
pub mod query;
pub mod rules;
pub mod seq_match;
pub mod spans;
pub mod spdx_lid;
pub mod spdx_mapping;
#[cfg(test)]
mod test_utils;
mod tokenize;
pub mod unknown_match;

#[cfg(feature = "debug-pipeline")]
mod debug_pipeline;

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;

use crate::license_detection::index::build_index;
use crate::license_detection::query::Query;
use crate::license_detection::rules::{load_licenses_from_directory, load_rules_from_directory};
use crate::license_detection::spdx_mapping::{SpdxMapping, build_spdx_mapping};
use crate::utils::text::strip_utf8_bom_str;

use crate::license_detection::detection::populate_detection_from_group_with_spdx;

pub use detection::{
    LicenseDetection, create_detection_from_group, group_matches_by_region,
    post_process_detections, sort_matches_by_line,
};
pub use models::LicenseMatch;

pub use aho_match::aho_match;
pub use hash_match::hash_match;
pub use match_refine::{
    filter_invalid_contained_unknown_matches, merge_overlapping_matches, refine_aho_matches,
    refine_matches, refine_matches_without_false_positive_filter, split_weak_matches,
};
pub use seq_match::{
    MAX_NEAR_DUPE_CANDIDATES, compute_candidates_with_msets, seq_match_with_candidates,
};
pub use spdx_lid::spdx_lid_match;
pub use unknown_match::unknown_match;

#[cfg(feature = "debug-pipeline")]
pub use debug_pipeline::{
    filter_below_rule_minimum_coverage_debug_only, filter_contained_matches_debug_only,
    filter_false_positive_matches_debug_only,
    filter_invalid_matches_to_single_word_gibberish_debug_only,
    filter_matches_missing_required_phrases_debug_only,
    filter_matches_to_spurious_single_token_debug_only, filter_overlapping_matches_debug_only,
    filter_short_matches_scattered_on_too_many_lines_debug_only,
    filter_spurious_matches_debug_only, filter_too_short_matches_debug_only,
};

/// License detection engine that orchestrates the detection pipeline.
///
/// The engine loads license rules and builds an index for efficient matching.
/// It supports multiple matching strategies (hash, SPDX-LID, Aho-Corasick, sequence)
/// and combines their results into final license detections.
#[derive(Debug, Clone)]
pub struct LicenseDetectionEngine {
    index: Arc<index::LicenseIndex>,
    spdx_mapping: SpdxMapping,
}

const MAX_DETECTION_SIZE: usize = 1024 * 1024; // 1MB

impl LicenseDetectionEngine {
    /// Create a new license detection engine from a directory of license rules.
    ///
    /// # Arguments
    /// * `rules_path` - Path to directory containing .LICENSE and .RULE files
    ///
    /// # Returns
    /// A Result containing the engine or an error
    pub fn new(rules_path: &Path) -> Result<Self> {
        let (rules_dir, licenses_dir) = if rules_path.ends_with("data") {
            (rules_path.join("rules"), rules_path.join("licenses"))
        } else if rules_path.ends_with("rules") {
            let parent = rules_path.parent().ok_or_else(|| {
                anyhow::anyhow!("Cannot determine parent directory for rules path")
            })?;
            (rules_path.to_path_buf(), parent.join("licenses"))
        } else {
            (rules_path.to_path_buf(), rules_path.to_path_buf())
        };

        let rules = load_rules_from_directory(&rules_dir, false)?;
        let licenses = load_licenses_from_directory(&licenses_dir, false)?;
        let index = build_index(rules, licenses);
        let mut license_vec: Vec<_> = index.licenses_by_key.values().cloned().collect();
        license_vec.sort_by(|a, b| a.key.cmp(&b.key));
        let spdx_mapping = build_spdx_mapping(&license_vec);

        Ok(Self {
            index: Arc::new(index),
            spdx_mapping,
        })
    }

    /// Detect licenses in the given text.
    ///
    /// This runs the full detection pipeline:
    /// 1. Create a Query from the text
    /// 2. Run matchers in priority order (hash, SPDX-LID, Aho-Corasick)
    /// 3. Phase 2: Near-duplicate detection (ALWAYS runs, even with exact matches)
    /// 4. Phase 3: Query run matching (per-run with high_resemblance=False)
    /// 5. Unknown matching (only if `unknown_licenses` is true)
    /// 6. Refine matches
    /// 7. Group matches by region
    /// 8. Create LicenseDetection objects
    ///
    /// # Arguments
    /// * `text` - The text to analyze
    /// * `unknown_licenses` - Whether to detect unknown licenses (default: false)
    ///
    /// # Returns
    /// A Result containing a vector of LicenseDetection objects
    pub fn detect(&self, text: &str, unknown_licenses: bool) -> Result<Vec<LicenseDetection>> {
        let clean_text = strip_utf8_bom_str(text);

        let content = if clean_text.len() > MAX_DETECTION_SIZE {
            log::warn!(
                "Content size {} exceeds limit {}, truncating for detection",
                clean_text.len(),
                MAX_DETECTION_SIZE
            );
            &clean_text[..MAX_DETECTION_SIZE]
        } else {
            clean_text
        };

        let mut query = Query::new(content, &self.index)?;

        let mut all_matches = Vec::new();
        let mut matched_qspans: Vec<query::PositionSpan> = Vec::new();

        // Phase 1a: Hash matching
        // Python returns immediately if hash matches found (index.py:987-991)
        {
            let whole_run = query.whole_query_run();
            let hash_matches = hash_match(&self.index, &whole_run);

            if !hash_matches.is_empty() {
                let mut matches = hash_matches;
                sort_matches_by_line(&mut matches);

                let groups = group_matches_by_region(&matches);
                let detections: Vec<LicenseDetection> = groups
                    .iter()
                    .map(|group| {
                        let mut detection = create_detection_from_group(group);
                        populate_detection_from_group_with_spdx(
                            &mut detection,
                            group,
                            &self.spdx_mapping,
                        );
                        detection
                    })
                    .collect();

                return Ok(post_process_detections(detections, 0.0));
            }
        }

        // Phase 1b: SPDX-LID matching
        {
            let spdx_matches = spdx_lid_match(&self.index, &query);
            let merged_spdx = merge_overlapping_matches(&spdx_matches);
            for m in &merged_spdx {
                if (m.match_coverage * 100.0).round() / 100.0 == 100.0
                    && m.end_token > m.start_token
                {
                    matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
                }
                if m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0 {
                    let span =
                        query::PositionSpan::new(m.start_token, m.end_token.saturating_sub(1));
                    query.subtract(&span);
                }
            }
            all_matches.extend(merged_spdx);
        }

        // Phase 1c: Aho-Corasick matching
        {
            let whole_run = query.whole_query_run();
            let aho_matches = aho_match(&self.index, &whole_run);

            #[cfg(debug_assertions)]
            let aho_count = aho_matches.len();

            // Python's get_exact_matches() calls refine_matches with merge=False
            // This applies quality filters including required phrase filtering
            let refined_aho = match_refine::refine_aho_matches(&self.index, aho_matches, &query);

            #[cfg(debug_assertions)]
            eprintln!(
                "DEBUG: aho_matches before refine: {}, after refine: {}",
                aho_count,
                refined_aho.len()
            );
            #[cfg(debug_assertions)]
            for m in refined_aho.iter().take(5) {
                eprintln!(
                    "  DEBUG AHO: {} rule={}",
                    m.license_expression, m.rule_identifier
                );
            }

            for m in &refined_aho {
                if (m.match_coverage * 100.0).round() / 100.0 == 100.0
                    && m.end_token > m.start_token
                {
                    matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
                }
            }
            all_matches.extend(refined_aho);
        }

        // Phases 2-4: Sequence matching (near_dupe + seq + query_runs)
        // Collect all sequence matches, merge ONCE after all phases
        // Corresponds to Python's single `approx` matcher (index.py:724-812)
        // Python always calls get_approximate_matches() - the is_matchable() check
        // happens AFTER each matcher to decide whether to continue (index.py:1059-1067)
        // The internal matchable_tokens().is_empty() check in compute_candidates_with_msets
        // handles the case where there are no matchables efficiently.
        let mut seq_all_matches = Vec::new();

        // Phase 2: Near-duplicate detection
        {
            let whole_run = query.whole_query_run();
            let near_dupe_candidates = compute_candidates_with_msets(
                &self.index,
                &whole_run,
                true,
                MAX_NEAR_DUPE_CANDIDATES,
            );

            if !near_dupe_candidates.is_empty() {
                let near_dupe_matches =
                    seq_match_with_candidates(&self.index, &whole_run, &near_dupe_candidates);

                for m in &near_dupe_matches {
                    if m.end_token > m.start_token {
                        let span = query::PositionSpan::new(m.start_token, m.end_token - 1);
                        query.subtract(&span);
                        matched_qspans.push(span);
                    }
                }

                seq_all_matches.extend(near_dupe_matches);
            }
        }

        // Phase 3: Query run matching
        // Python: index.py:787-812 - iterates over query_runs with high_resemblance=False
        // NOTE: Python does NOT call query.subtract() in this loop - only in near-dupe phase
        // The is_matchable() check prevents double-matching using matched_qspans from near-dupe
        const MAX_QUERY_RUN_CANDIDATES: usize = 70;
        {
            for query_run in query.query_runs().iter() {
                if !query_run.is_matchable(false, &matched_qspans) {
                    continue;
                }

                let candidates = compute_candidates_with_msets(
                    &self.index,
                    query_run,
                    false,
                    MAX_QUERY_RUN_CANDIDATES,
                );
                if !candidates.is_empty() {
                    let matches =
                        seq_match_with_candidates(&self.index, query_run, &candidates);
                    seq_all_matches.extend(matches);
                }
            }
        }

        // Merge all sequence matches ONCE (like Python's approx matcher)
        let merged_seq = merge_overlapping_matches(&seq_all_matches);
        all_matches.extend(merged_seq);

        // Step 1: Initial refine WITHOUT false positive filtering
        // Python: refine_matches with filter_false_positive=False (index.py:1073-1080)
        let merged_matches =
            refine_matches_without_false_positive_filter(&self.index, all_matches, &query);

        // Step 2: Unknown detection and weak match handling
        // Python: index.py:1079-1118 - only runs when unknown_licenses=True
        let refined_matches = if unknown_licenses {
            // Split weak from good - Python: index.py:1083
            let (good_matches, weak_matches) = split_weak_matches(&merged_matches);

            // Unknown detection on uncovered regions - Python: index.py:1093-1114
            let unknown_matches = unknown_match(&self.index, &query, &good_matches);
            let filtered_unknown =
                filter_invalid_contained_unknown_matches(&unknown_matches, &good_matches);

            let mut all_matches = good_matches;
            all_matches.extend(filtered_unknown);
            // reinject weak matches and let refine matches keep the bests
            // Python: index.py:1117-1118
            all_matches.extend(weak_matches);
            all_matches
        } else {
            merged_matches
        };

        // Step 5: Final refine WITH false positive filtering - Python: index.py:1130-1145
        let refined = refine_matches(&self.index, refined_matches, &query);

        let mut sorted = refined;
        sort_matches_by_line(&mut sorted);

        let groups = group_matches_by_region(&sorted);

        let detections: Vec<LicenseDetection> = groups
            .iter()
            .map(|group| {
                let mut detection = create_detection_from_group(group);
                populate_detection_from_group_with_spdx(&mut detection, group, &self.spdx_mapping);
                detection
            })
            .collect();

        let detections = post_process_detections(detections, 0.0);

        Ok(detections)
    }

    /// Detect licenses and return raw matches (like Python's idx.match()).
    ///
    /// This method returns matches after refinement, WITHOUT grouping into detections.
    /// Use this for testing and comparison with Python's idx.match() output.
    /// For production use, prefer detect() which returns grouped detections.
    ///
    /// # Arguments
    /// * `text` - The text to analyze
    /// * `unknown_licenses` - Whether to detect unknown licenses (default: false)
    ///
    /// # Returns
    /// A Result containing a vector of LicenseMatch objects (ungrouped)
    pub fn detect_matches(&self, text: &str, unknown_licenses: bool) -> Result<Vec<LicenseMatch>> {
        let clean_text = strip_utf8_bom_str(text);

        let content = if clean_text.len() > MAX_DETECTION_SIZE {
            log::warn!(
                "Content size {} exceeds limit {}, truncating for detection",
                clean_text.len(),
                MAX_DETECTION_SIZE
            );
            &clean_text[..MAX_DETECTION_SIZE]
        } else {
            clean_text
        };

        let mut query = Query::new(content, &self.index)?;

        let mut all_matches = Vec::new();
        let mut matched_qspans: Vec<query::PositionSpan> = Vec::new();

        // Phase 1a: Hash matching
        {
            let whole_run = query.whole_query_run();
            let hash_matches = hash_match(&self.index, &whole_run);

            if !hash_matches.is_empty() {
                let mut matches = hash_matches;
                sort_matches_by_line(&mut matches);
                return Ok(matches);
            }
        }

        // Phase 1b: SPDX-LID matching
        {
            let spdx_matches = spdx_lid_match(&self.index, &query);
            let merged_spdx = merge_overlapping_matches(&spdx_matches);
            for m in &merged_spdx {
                if (m.match_coverage * 100.0).round() / 100.0 == 100.0
                    && m.end_token > m.start_token
                {
                    matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
                }
                if m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0 {
                    let span =
                        query::PositionSpan::new(m.start_token, m.end_token.saturating_sub(1));
                    query.subtract(&span);
                }
            }
            all_matches.extend(merged_spdx);
        }

        // Phase 1c: Aho-Corasick matching
        {
            let whole_run = query.whole_query_run();
            let aho_matches = aho_match(&self.index, &whole_run);
            let refined_aho = match_refine::refine_aho_matches(&self.index, aho_matches, &query);

            for m in &refined_aho {
                if (m.match_coverage * 100.0).round() / 100.0 == 100.0
                    && m.end_token > m.start_token
                {
                    matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
                }
            }
            all_matches.extend(refined_aho);
        }

        let whole_run = query.whole_query_run();
        let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);

        let mut seq_all_matches = Vec::new();
        if !skip_seq_matching {
            // Phase 2: Near-duplicate detection
            {
                let whole_run = query.whole_query_run();
                let near_dupe_candidates = compute_candidates_with_msets(
                    &self.index,
                    &whole_run,
                    true,
                    MAX_NEAR_DUPE_CANDIDATES,
                );

                if !near_dupe_candidates.is_empty() {
                    let near_dupe_matches =
                        seq_match_with_candidates(&self.index, &whole_run, &near_dupe_candidates);

                    for m in &near_dupe_matches {
                        if m.end_token > m.start_token {
                            let span = query::PositionSpan::new(m.start_token, m.end_token - 1);
                            query.subtract(&span);
                            matched_qspans.push(span);
                        }
                    }

                    seq_all_matches.extend(near_dupe_matches);
                }
            }

            // Phase 3: Regular sequence matching
            const MAX_SEQ_CANDIDATES: usize = 70;
            {
                let whole_run = query.whole_query_run();
                let candidates = compute_candidates_with_msets(
                    &self.index,
                    &whole_run,
                    false,
                    MAX_SEQ_CANDIDATES,
                );
                if !candidates.is_empty() {
                    let matches = seq_match_with_candidates(&self.index, &whole_run, &candidates);

                    // Add to matched_qspans to prevent double-matching in Phase 4
                    for m in &matches {
                        if m.end_token > m.start_token {
                            let span = query::PositionSpan::new(m.start_token, m.end_token - 1);
                            query.subtract(&span); // Update matchables
                            matched_qspans.push(span); // Track matched regions
                        }
                    }

                    seq_all_matches.extend(matches);
                }
            }

            // Phase 4: Query run matching
            const MAX_QUERY_RUN_CANDIDATES: usize = 70;
            {
                let whole_run = query.whole_query_run();
                let mut phase4_spans: Vec<query::PositionSpan> = Vec::new();
                for query_run in query.query_runs().iter() {
                    if query_run.start == whole_run.start && query_run.end == whole_run.end {
                        continue;
                    }

                    if !query_run.is_matchable(false, &matched_qspans) {
                        continue;
                    }

                    let candidates = compute_candidates_with_msets(
                        &self.index,
                        query_run,
                        false,
                        MAX_QUERY_RUN_CANDIDATES,
                    );
                    if !candidates.is_empty() {
                        let matches =
                            seq_match_with_candidates(&self.index, query_run, &candidates);

                        // Collect spans to add to matched_qspans (apply after loop due to borrow)
                        for m in &matches {
                            if m.end_token > m.start_token {
                                let span = query::PositionSpan::new(m.start_token, m.end_token - 1);
                                phase4_spans.push(span);
                            }
                        }

                        seq_all_matches.extend(matches);
                    }
                }
                // Apply spans after the loop completes
                for span in &phase4_spans {
                    query.subtract(span);
                }
                matched_qspans.extend(phase4_spans);
            }

            let merged_seq = merge_overlapping_matches(&seq_all_matches);
            all_matches.extend(merged_seq);
        }

        // Step 1: Initial refine WITHOUT false positive filtering
        let merged_matches =
            refine_matches_without_false_positive_filter(&self.index, all_matches, &query);

        // Step 2: Unknown detection and weak match handling
        let refined_matches = if unknown_licenses {
            let (good_matches, weak_matches) = split_weak_matches(&merged_matches);
            let unknown_matches = unknown_match(&self.index, &query, &good_matches);
            let filtered_unknown =
                filter_invalid_contained_unknown_matches(&unknown_matches, &good_matches);

            let mut all_matches = good_matches;
            all_matches.extend(filtered_unknown);
            all_matches.extend(weak_matches);
            all_matches
        } else {
            merged_matches
        };

        // Step 3: Final refine WITH false positive filtering - Python: index.py:1130-1145
        let refined = refine_matches(&self.index, refined_matches, &query);

        let mut sorted = refined;
        sort_matches_by_line(&mut sorted);

        // DEBUG: Print match count
        #[cfg(feature = "debug-pipeline")]
        {
            eprintln!("DEBUG detect_matches(): returning {} matches", sorted.len());
            for m in &sorted {
                eprintln!("  - {} ({})", m.rule_identifier, m.license_expression);
            }
        }

        // Return raw matches (NOT grouped) - this is Python's idx.match() behavior
        Ok(sorted)
    }

    /// Get a reference to the license index.
    pub fn index(&self) -> &index::LicenseIndex {
        &self.index
    }

    /// Get a reference to the SPDX mapping.<
    pub fn spdx_mapping(&self) -> &SpdxMapping {
        &self.spdx_mapping
    }
}

#[cfg(test)]
mod tests;
