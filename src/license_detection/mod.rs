//! License Detection Engine

pub mod aho_match;
mod detection;

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

use std::collections::{HashMap, HashSet};
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
    filter_invalid_contained_unknown_matches, merge_overlapping_matches, refine_matches,
    refine_matches_without_false_positive_filter, split_weak_matches,
};
pub use seq_match::{
    MAX_NEAR_DUPE_CANDIDATES, compute_candidates_with_msets, seq_match_with_candidates,
};
pub use spdx_lid::spdx_lid_match;
pub use unknown_match::unknown_match;

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

const MAX_DETECTION_SIZE: usize = 10 * 1024 * 1024; // 10MB
const MAX_REGULAR_SEQ_CANDIDATES: usize = 70;
const OPENJ9_NOTICE_PREAMBLE_CUE: &str = "Subject to the following notices:";
const MAX_REDUNDANT_SEQ_CONTAINER_BOUNDARY_GAP: usize = 8;
const MAX_REDUNDANT_SEQ_CONTAINER_UNMATCHED_GAP: usize = 2;

fn query_span_for_match(m: &LicenseMatch) -> Option<query::PositionSpan> {
    (m.end_token > m.start_token).then(|| query::PositionSpan::new(m.start_token, m.end_token - 1))
}

fn has_full_match_coverage(m: &LicenseMatch) -> bool {
    ((m.match_coverage * 100.0).round() / 100.0) == 100.0
}

fn is_redundant_same_expression_seq_container(
    container: &LicenseMatch,
    candidate_contained_matches: &[LicenseMatch],
) -> bool {
    let container_is_redundant_coverage =
        has_full_match_coverage(container) || container.match_coverage >= 99.0;
    if container.matcher != seq_match::MATCH_SEQ || !container_is_redundant_coverage {
        return false;
    }

    let mut contained: Vec<&LicenseMatch> = candidate_contained_matches
        .iter()
        .filter(|m| {
            m.matcher == aho_match::MATCH_AHO
                && has_full_match_coverage(m)
                && m.license_expression == container.license_expression
                && (container.qcontains(m) || container.qoverlap(m) > 0)
        })
        .collect();

    if contained.len() < 2 {
        return false;
    }

    contained.sort_by_key(|m| m.qspan_bounds());

    let container_qspan: HashSet<usize> = container.qspan().into_iter().collect();
    let mut child_union = HashSet::new();
    for child in &contained {
        child_union.extend(child.qspan());
    }

    let container_only_positions: HashSet<usize> =
        container_qspan.difference(&child_union).copied().collect();
    let child_only_positions: HashSet<usize> =
        child_union.difference(&container_qspan).copied().collect();

    let mut bridge_positions = HashSet::new();
    for pair in contained.windows(2) {
        let (_, previous_end) = pair[0].qspan_bounds();
        let (next_start, _) = pair[1].qspan_bounds();

        if next_start < previous_end {
            return false;
        }

        bridge_positions.extend(previous_end..next_start);
    }

    let container_only_boundary_positions = container_only_positions
        .difference(&bridge_positions)
        .count();

    let max_container_only_positions =
        MAX_REDUNDANT_SEQ_CONTAINER_BOUNDARY_GAP * contained.len() + 1;
    let max_container_boundary_positions =
        MAX_REDUNDANT_SEQ_CONTAINER_BOUNDARY_GAP * (contained.len() - 1);
    let max_child_only_positions = MAX_REDUNDANT_SEQ_CONTAINER_UNMATCHED_GAP + 1;

    container_only_positions.len() <= max_container_only_positions
        && container_only_boundary_positions <= max_container_boundary_positions
        && child_only_positions.len() <= max_child_only_positions
}

fn filter_redundant_same_expression_seq_containers(
    seq_matches: Vec<LicenseMatch>,
    candidate_contained_matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    seq_matches
        .into_iter()
        .filter(|m| !is_redundant_same_expression_seq_container(m, candidate_contained_matches))
        .collect()
}

fn subtract_spdx_match_qspans(
    query: &mut Query<'_>,
    matched_qspans: &mut Vec<query::PositionSpan>,
    aho_extra_matchables: &mut HashSet<usize>,
    spdx_matches: &[LicenseMatch],
) {
    for m in spdx_matches {
        let Some(span) = query_span_for_match(m) else {
            continue;
        };

        aho_extra_matchables.extend(span.positions());
        query.subtract(&span);

        if (m.match_coverage * 100.0).round() / 100.0 == 100.0 {
            matched_qspans.push(span);
        }
    }
}

fn merge_and_prepare_aho_matches(
    index: &index::LicenseIndex,
    query: &mut Query<'_>,
    matched_qspans: &mut Vec<query::PositionSpan>,
    refined_aho: &[LicenseMatch],
) -> (Vec<LicenseMatch>, bool) {
    let merged_aho = merge_overlapping_matches(refined_aho);
    let mut saw_long_exact_license_text_match = false;

    for m in &merged_aho {
        let Some(span) = query_span_for_match(m) else {
            continue;
        };

        if has_full_match_coverage(m) {
            matched_qspans.push(span.clone());
        }

        if index
            .rules_by_rid
            .get(m.rid)
            .is_some_and(|rule| rule.is_license_text && !rule.is_from_license)
            && m.rule_length > 120
            && m.match_coverage > 98.0
        {
            query.subtract(&span);
            saw_long_exact_license_text_match = true;
        }
    }

    (merged_aho, saw_long_exact_license_text_match)
}

fn is_top_level_embedded_section_boundary(line: &str) -> bool {
    let trimmed = line.trim_start();
    let bytes = trimmed.as_bytes();

    bytes.len() >= 5
        && bytes[0].is_ascii_uppercase()
        && bytes[1] == b'.'
        && bytes[2].is_ascii_whitespace()
        && bytes[3].is_ascii_whitespace()
        && !trimmed[4..].trim().is_empty()
}

fn synthetic_openj9_notice_preamble_run<'a>(query: &'a Query<'a>) -> Option<query::QueryRun<'a>> {
    if query.is_empty() {
        return None;
    }

    let lines: Vec<&str> = query.text.lines().collect();
    let cue_index = lines
        .iter()
        .position(|line| line.trim() == OPENJ9_NOTICE_PREAMBLE_CUE)?;

    let boundary_line =
        lines
            .iter()
            .enumerate()
            .skip(cue_index + 1)
            .find_map(|(index, line)| {
                is_top_level_embedded_section_boundary(line).then_some(index + 1)
            })?;

    let end_token = query
        .line_by_pos
        .iter()
        .position(|&line| line >= boundary_line)
        .and_then(|pos| pos.checked_sub(1))?;

    Some(query::QueryRun::new(query, 0, Some(end_token)))
}

fn synthetic_openj9_notice_matches(
    index: &index::LicenseIndex,
    query: &mut Query<'_>,
    matched_qspans: &mut Vec<query::PositionSpan>,
) -> Vec<LicenseMatch> {
    let Some(query_run) = synthetic_openj9_notice_preamble_run(query) else {
        return Vec::new();
    };

    if !query_run.is_matchable(false, &[]) {
        return Vec::new();
    }

    let candidates =
        compute_candidates_with_msets(index, &query_run, false, MAX_REGULAR_SEQ_CANDIDATES);
    if candidates.is_empty() {
        return Vec::new();
    }

    let notice_matches = candidates
        .iter()
        .filter(|candidate| candidate.rule.is_license_notice)
        .find_map(|candidate| {
            let matches =
                seq_match_with_candidates(index, &query_run, std::slice::from_ref(candidate));
            if matches.is_empty() {
                None
            } else {
                Some(coalesce_match_fragments(matches))
            }
        });

    let Some(notice_matches) = notice_matches else {
        return Vec::new();
    };

    let notice_matches = merge_overlapping_matches(&notice_matches);
    for m in &notice_matches {
        let Some(span) = query_span_for_match(m) else {
            continue;
        };

        query.subtract(&span);
        matched_qspans.push(span);
    }

    notice_matches
}

fn coalesce_match_fragments(matches: Vec<LicenseMatch>) -> Vec<LicenseMatch> {
    let mut matches_by_rid: HashMap<usize, Vec<LicenseMatch>> = HashMap::new();
    for m in matches {
        matches_by_rid.entry(m.rid).or_default().push(m);
    }

    let mut coalesced = Vec::new();
    for mut group in matches_by_rid.into_values() {
        group.sort_by_key(|m| (m.start_token, m.end_token));
        let mut combined = group.remove(0);

        for fragment in group {
            combined = coalesce_match_pair(combined, fragment);
        }

        coalesced.push(combined);
    }

    coalesced.sort_by_key(|m| (m.start_token, m.end_token, m.rule_identifier.clone()));
    coalesced
}

fn coalesce_match_pair(mut combined: LicenseMatch, fragment: LicenseMatch) -> LicenseMatch {
    let mut qspan_positions = combined.qspan();
    qspan_positions.extend(fragment.qspan());
    qspan_positions.sort_unstable();
    qspan_positions.dedup();

    let mut ispan_positions = combined.ispan();
    ispan_positions.extend(fragment.ispan());
    ispan_positions.sort_unstable();
    ispan_positions.dedup();

    let mut hispan_positions = combined.hispan();
    hispan_positions.extend(fragment.hispan());
    hispan_positions.sort_unstable();
    hispan_positions.dedup();

    combined.start_token = *qspan_positions.first().unwrap_or(&combined.start_token);
    combined.end_token = qspan_positions
        .last()
        .map(|position| position + 1)
        .unwrap_or(combined.end_token);
    combined.start_line = combined.start_line.min(fragment.start_line);
    combined.end_line = combined.end_line.max(fragment.end_line);
    combined.rule_start_token = *ispan_positions
        .first()
        .unwrap_or(&combined.rule_start_token);
    combined.matched_length = qspan_positions.len();
    combined.hilen = hispan_positions.len();
    combined.match_coverage = if combined.rule_length == 0 {
        combined.match_coverage.max(fragment.match_coverage)
    } else {
        (combined.matched_length as f32 / combined.rule_length as f32) * 100.0
    };
    combined.score = combined.score.max(fragment.score);
    combined.candidate_resemblance = combined
        .candidate_resemblance
        .max(fragment.candidate_resemblance);
    combined.candidate_containment = combined
        .candidate_containment
        .max(fragment.candidate_containment);
    combined.matched_token_positions = Some(qspan_positions.clone());
    combined.qspan_positions = Some(qspan_positions);
    combined.ispan_positions = Some(ispan_positions);
    combined.hispan_positions = Some(hispan_positions);

    combined
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RegularSeqEntrypoint {
    QueryRuns,
    DetectMatchesRawParity,
}

fn collect_whole_query_exact_followup_matches(
    index: &index::LicenseIndex,
    query: &mut Query<'_>,
    matched_qspans: &mut Vec<query::PositionSpan>,
) -> Vec<LicenseMatch> {
    let mut seq_all_matches = Vec::new();

    seq_all_matches.extend(synthetic_openj9_notice_matches(
        index,
        query,
        matched_qspans,
    ));

    let whole_run = query.whole_query_run();

    if whole_run.is_matchable(false, matched_qspans) {
        let near_dupe_candidates =
            compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);

        if !near_dupe_candidates.is_empty() {
            let near_dupe_matches =
                seq_match_with_candidates(index, &whole_run, &near_dupe_candidates);

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

    seq_all_matches
}

fn regular_seq_runs<'a>(
    query: &'a Query<'a>,
    entrypoint: RegularSeqEntrypoint,
) -> Vec<query::QueryRun<'a>> {
    match entrypoint {
        RegularSeqEntrypoint::QueryRuns => query.query_runs(),
        RegularSeqEntrypoint::DetectMatchesRawParity => {
            if synthetic_openj9_notice_preamble_run(query).is_some() {
                vec![query.whole_query_run()]
            } else {
                query.query_runs()
            }
        }
    }
}

fn collect_regular_seq_matches(
    index: &index::LicenseIndex,
    query: &Query<'_>,
    matched_qspans: &[query::PositionSpan],
    candidate_contained_matches: &[LicenseMatch],
    entrypoint: RegularSeqEntrypoint,
) -> Vec<LicenseMatch> {
    let mut seq_all_matches = Vec::new();

    for query_run in regular_seq_runs(query, entrypoint) {
        if !query_run.is_matchable(false, matched_qspans) {
            continue;
        }

        let candidates =
            compute_candidates_with_msets(index, &query_run, false, MAX_REGULAR_SEQ_CANDIDATES);
        if !candidates.is_empty() {
            let matches = seq_match_with_candidates(index, &query_run, &candidates);
            seq_all_matches.extend(matches);
        }
    }

    let merged_seq = merge_overlapping_matches(&seq_all_matches);
    filter_redundant_same_expression_seq_containers(merged_seq, candidate_contained_matches)
}

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
        let mut candidate_contained_matches = Vec::new();
        let mut aho_extra_matchables = HashSet::new();
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
            subtract_spdx_match_qspans(
                &mut query,
                &mut matched_qspans,
                &mut aho_extra_matchables,
                &merged_spdx,
            );
            all_matches.extend(merged_spdx);
        }

        // Phase 1c: Aho-Corasick matching
        {
            let whole_run = query.whole_query_run();
            let aho_matches = if aho_extra_matchables.is_empty() {
                aho_match(&self.index, &whole_run)
            } else {
                aho_match::aho_match_with_extra_matchables(
                    &self.index,
                    &whole_run,
                    Some(&aho_extra_matchables),
                )
            };

            // Python's get_exact_matches() calls refine_matches with merge=False
            // This applies quality filters including required phrase filtering
            let refined_aho = match_refine::refine_aho_matches(&self.index, aho_matches, &query);
            candidate_contained_matches.extend(refined_aho.clone());
            let (merged_aho, _) = merge_and_prepare_aho_matches(
                &self.index,
                &mut query,
                &mut matched_qspans,
                &refined_aho,
            );
            all_matches.extend(merged_aho);

            let whole_query_followup = collect_whole_query_exact_followup_matches(
                &self.index,
                &mut query,
                &mut matched_qspans,
            );
            all_matches.extend(whole_query_followup);

            let merged_seq = collect_regular_seq_matches(
                &self.index,
                &query,
                &matched_qspans,
                &candidate_contained_matches,
                RegularSeqEntrypoint::QueryRuns,
            );
            all_matches.extend(merged_seq);
        }

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
        let mut candidate_contained_matches = Vec::new();
        let mut aho_extra_matchables = HashSet::new();
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
            subtract_spdx_match_qspans(
                &mut query,
                &mut matched_qspans,
                &mut aho_extra_matchables,
                &merged_spdx,
            );
            all_matches.extend(merged_spdx);
        }

        // Phase 1c: Aho-Corasick matching
        {
            let whole_run = query.whole_query_run();
            let aho_matches = if aho_extra_matchables.is_empty() {
                aho_match(&self.index, &whole_run)
            } else {
                aho_match::aho_match_with_extra_matchables(
                    &self.index,
                    &whole_run,
                    Some(&aho_extra_matchables),
                )
            };
            let refined_aho = match_refine::refine_aho_matches(&self.index, aho_matches, &query);
            candidate_contained_matches.extend(refined_aho.clone());
            let (merged_aho, _) = merge_and_prepare_aho_matches(
                &self.index,
                &mut query,
                &mut matched_qspans,
                &refined_aho,
            );
            all_matches.extend(merged_aho);

            let whole_query_followup = collect_whole_query_exact_followup_matches(
                &self.index,
                &mut query,
                &mut matched_qspans,
            );
            all_matches.extend(whole_query_followup);

            let merged_seq = collect_regular_seq_matches(
                &self.index,
                &query,
                &matched_qspans,
                &candidate_contained_matches,
                RegularSeqEntrypoint::DetectMatchesRawParity,
            );
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
