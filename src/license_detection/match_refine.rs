//! Match refinement - merge, filter, and finalize license matches.
//!
//! This module implements the final phase of license matching where raw matches
//! from all strategies are combined, refined, and finalized.
//!
//! Based on the Python ScanCode Toolkit implementation at:
//! reference/scancode-toolkit/src/licensedcode/match.py

use crate::license_detection::expression::licensing_contains;
use crate::license_detection::index::LicenseIndex;
use crate::license_detection::models::LicenseMatch;
use crate::license_detection::query::Query;
use crate::license_detection::spans::Span;
use std::collections::HashSet;

const OVERLAP_SMALL: f64 = 0.10;
const OVERLAP_MEDIUM: f64 = 0.40;
const OVERLAP_LARGE: f64 = 0.70;
const OVERLAP_EXTRA_LARGE: f64 = 0.90;

const MIN_SHORT_FP_LIST_LENGTH: usize = 15;
const MIN_LONG_FP_LIST_LENGTH: usize = 150;
const MIN_UNIQUE_LICENSES: usize = MIN_SHORT_FP_LIST_LENGTH / 3;
const MIN_UNIQUE_LICENSES_PROPORTION: f64 = 1.0 / 3.0;
const MAX_CANDIDATE_LENGTH: usize = 20;
const MAX_DISTANCE_BETWEEN_CANDIDATES: usize = 10;
const MAX_DIST: usize = 100;

/// Filter unknown matches contained within good matches' qregion.
///
/// Unknown license matches that are fully contained within the qregion
/// (token span from start_token to end_token) of a known good match
/// should be discarded as they are redundant.
///
/// # Arguments
/// * `unknown_matches` - Slice of unknown license matches to filter
/// * `good_matches` - Slice of known good matches to check containment against
///
/// # Returns
/// Vector of unknown LicenseMatch with contained matches removed
///
/// Based on Python: `filter_invalid_contained_unknown_matches()` (match.py:1904-1926)
pub fn filter_invalid_contained_unknown_matches(
    unknown_matches: &[LicenseMatch],
    good_matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    unknown_matches
        .iter()
        .filter(|unknown| {
            let unknown_start = unknown.start_token;
            let unknown_end = unknown.end_token;

            let is_contained = good_matches
                .iter()
                .any(|good| good.start_token <= unknown_start && good.end_token >= unknown_end);

            !is_contained
        })
        .cloned()
        .collect()
}

fn filter_too_short_matches(index: &LicenseIndex, matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    matches
        .iter()
        .filter(|m| {
            if m.matcher != "3-seq" {
                return true;
            }

            if let Some(rid) = parse_rule_id(&m.rule_identifier)
                && let Some(rule) = index.rules_by_rid.get(rid)
            {
                return !m.is_small(
                    rule.min_matched_length,
                    rule.min_high_matched_length,
                    rule.is_small,
                );
            }

            true
        })
        .cloned()
        .collect()
}

/// Parse rule ID from rule_identifier string.
///
/// Rule identifiers typically have the format "#42" where the numeric portion is the rule ID.
///
/// # Arguments
/// * `rule_identifier` - String like "#42" or "mit.LICENSE"
///
/// # Returns
/// * `Some(usize)` - Parsed rule ID if valid
/// * `None` - If rule_identifier is empty or doesn't contain a valid number
///
/// # Examples
/// ```
/// assert_eq!(parse_rule_id("#42"), Some(42));
/// assert_eq!(parse_rule_id("#0"), Some(0));
/// assert_eq!(parse_rule_id("invalid"), None);
/// ```
fn parse_rule_id(rule_identifier: &str) -> Option<usize> {
    let trimmed = rule_identifier.trim();
    if let Some(stripped) = trimmed.strip_prefix('#') {
        stripped.parse().ok()
    } else {
        trimmed.parse().ok()
    }
}

fn combine_matches(a: &LicenseMatch, b: &LicenseMatch) -> LicenseMatch {
    let mut merged = a.clone();

    let mut qspan: HashSet<usize> = a.qspan().into_iter().collect();
    qspan.extend(b.qspan());
    let mut qspan_vec: Vec<usize> = qspan.into_iter().collect();
    qspan_vec.sort();

    let mut ispan: HashSet<usize> = a.ispan().into_iter().collect();
    ispan.extend(b.ispan());
    let mut ispan_vec: Vec<usize> = ispan.into_iter().collect();
    ispan_vec.sort();

    let a_hispan: HashSet<usize> = (a.rule_start_token..a.rule_start_token + a.hilen)
        .filter(|&p| a.ispan().contains(&p))
        .collect();
    let b_hispan: HashSet<usize> = (b.rule_start_token..b.rule_start_token + b.hilen)
        .filter(|&p| b.ispan().contains(&p))
        .collect();
    let combined_hispan: HashSet<usize> = a_hispan.union(&b_hispan).copied().collect();
    let hilen = combined_hispan.len();

    merged.start_token = *qspan_vec.first().unwrap_or(&a.start_token);
    merged.end_token = qspan_vec.last().map(|&x| x + 1).unwrap_or(a.end_token);
    merged.rule_start_token = *ispan_vec.first().unwrap_or(&a.rule_start_token);
    merged.matched_length = qspan_vec.len();
    merged.hilen = hilen;
    merged.start_line = a.start_line.min(b.start_line);
    merged.end_line = a.end_line.max(b.end_line);
    merged.score = a.score.max(b.score);
    merged.qspan_positions = Some(qspan_vec);
    merged.ispan_positions = Some(ispan_vec);

    if merged.rule_length > 0 {
        merged.match_coverage = (merged.matched_length.min(merged.rule_length) as f32
            / merged.rule_length as f32)
            * 100.0;
    }

    merged
}

/// Merge overlapping and adjacent matches for the same rule.
///
/// Based on Python: `merge_matches()` (match.py:869-1068)
/// Uses distance-based merging with multiple merge conditions.
fn merge_overlapping_matches(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    if matches.is_empty() {
        return Vec::new();
    }

    if matches.len() == 1 {
        return matches.to_vec();
    }

    let mut sorted: Vec<&LicenseMatch> = matches.iter().collect();
    sorted.sort_by(|a, b| {
        a.rule_identifier
            .cmp(&b.rule_identifier)
            .then_with(|| a.start_token.cmp(&b.start_token))
            .then_with(|| b.hilen.cmp(&a.hilen))
            .then_with(|| b.matched_length.cmp(&a.matched_length))
    });

    let mut grouped: Vec<Vec<&LicenseMatch>> = Vec::new();
    let mut current_group: Vec<&LicenseMatch> = Vec::new();

    for m in sorted {
        if current_group.is_empty() || current_group[0].rule_identifier == m.rule_identifier {
            current_group.push(m);
        } else {
            grouped.push(current_group);
            current_group = vec![m];
        }
    }
    if !current_group.is_empty() {
        grouped.push(current_group);
    }

    let mut merged = Vec::new();

    for rule_matches in grouped {
        if rule_matches.len() == 1 {
            merged.push(rule_matches[0].clone());
            continue;
        }

        let rule_length = rule_matches[0].rule_length;
        let max_rule_side_dist = (rule_length / 2).clamp(1, MAX_DIST);

        let mut rule_matches: Vec<LicenseMatch> =
            rule_matches.iter().map(|m| (*m).clone()).collect();
        let mut i = 0;

        while i < rule_matches.len().saturating_sub(1) {
            let mut j = i + 1;

            while j < rule_matches.len() {
                let current = rule_matches[i].clone();
                let next = rule_matches[j].clone();

                if current.qdistance_to(&next) > max_rule_side_dist
                    || current.idistance_to(&next) > max_rule_side_dist
                {
                    break;
                }

                if current.qspan() == next.qspan() && current.ispan() == next.ispan() {
                    rule_matches.remove(j);
                    continue;
                }

                if current.ispan() == next.ispan() && current.qoverlap(&next) > 0 {
                    if current.matched_length >= next.matched_length {
                        rule_matches.remove(j);
                        continue;
                    } else {
                        rule_matches.remove(i);
                        i = i.saturating_sub(1);
                        break;
                    }
                }

                if current.qcontains(&next) {
                    rule_matches.remove(j);
                    continue;
                }
                if next.qcontains(&current) {
                    rule_matches.remove(i);
                    i = i.saturating_sub(1);
                    break;
                }

                if current.surround(&next) {
                    let combined = combine_matches(&current, &next);
                    if combined.qspan().len() == combined.ispan().len() {
                        rule_matches[i] = combined;
                        rule_matches.remove(j);
                        continue;
                    }
                }
                if next.surround(&current) {
                    let combined = combine_matches(&current, &next);
                    if combined.qspan().len() == combined.ispan().len() {
                        rule_matches[j] = combined;
                        rule_matches.remove(i);
                        i = i.saturating_sub(1);
                        break;
                    }
                }

                if next.is_after(&current) {
                    rule_matches[i] = combine_matches(&current, &next);
                    rule_matches.remove(j);
                    continue;
                }

                let (cur_qstart, cur_qend) = current.qspan_bounds();
                let (next_qstart, next_qend) = next.qspan_bounds();
                let (cur_istart, cur_iend) = current.ispan_bounds();
                let (next_istart, next_iend) = next.ispan_bounds();

                if cur_qstart <= next_qstart
                    && cur_qend <= next_qend
                    && cur_istart <= next_istart
                    && cur_iend <= next_iend
                {
                    let qoverlap = current.qoverlap(&next);
                    if qoverlap > 0 {
                        let ioverlap = current.ispan_overlap(&next);
                        if qoverlap == ioverlap {
                            rule_matches[i] = combine_matches(&current, &next);
                            rule_matches.remove(j);
                            continue;
                        }
                    }
                }

                j += 1;
            }
            i += 1;
        }
        merged.extend(rule_matches);
    }

    merged
}

/// Filter matches that are contained within other matches.
///
/// A match A is contained in match B if:
/// - A's qspan (token positions) is contained in B's qspan, OR
/// - B's license expression subsumes A's expression (e.g., "gpl-2.0 WITH exception" subsumes "gpl-2.0")
///
/// This uses token positions (start_token/end_token) instead of line numbers
/// for more precise containment detection, matching Python's qcontains behavior.
/// Expression subsumption handles WITH expressions where the base license should
/// not appear separately when the WITH expression is detected.
///
/// The containing (larger) match is kept, the contained (smaller) match is removed.
/// This function does NOT group by rule_identifier - matches from different rules
/// can contain each other.
///
/// # Arguments
/// * `matches` - Slice of LicenseMatch to filter
///
/// # Returns
/// Tuple of (kept matches, discarded matches)
///
/// Based on Python: `filter_contained_matches()` using qspan containment and expression subsumption
fn filter_contained_matches(matches: &[LicenseMatch]) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    if matches.len() < 2 {
        return (matches.to_vec(), Vec::new());
    }

    let mut matches: Vec<LicenseMatch> = matches.to_vec();
    let mut discarded = Vec::new();

    matches.sort_by(|a, b| {
        a.start_token
            .cmp(&b.start_token)
            .then_with(|| b.hilen.cmp(&a.hilen))
            .then_with(|| b.matched_length.cmp(&a.matched_length))
            .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
    });

    let mut i = 0;
    while i < matches.len().saturating_sub(1) {
        let mut j = i + 1;
        while j < matches.len() {
            let current = matches[i].clone();
            let next = matches[j].clone();

            if next.end_token > current.end_token {
                break;
            }

            if current.start_token == next.start_token && current.end_token == next.end_token {
                if current.match_coverage >= next.match_coverage {
                    discarded.push(matches.remove(j));
                    continue;
                } else {
                    discarded.push(matches.remove(i));
                    i = i.saturating_sub(1);
                    break;
                }
            }

            if current.qcontains(&next) || licensing_contains_match(&current, &next) {
                discarded.push(matches.remove(j));
                continue;
            }
            if next.qcontains(&current) || licensing_contains_match(&next, &current) {
                discarded.push(matches.remove(i));
                i = i.saturating_sub(1);
                break;
            }

            j += 1;
        }
        i += 1;
    }

    (matches, discarded)
}

/// Filter matches to false positive rules.
///
/// Removes matches whose rule ID is in the index's false_positive_rids set.
///
/// # Arguments
/// * `index` - LicenseIndex containing false_positive_rids
/// * `matches` - Slice of LicenseMatch to filter
///
/// # Returns
/// Vector of LicenseMatch with false positive matches removed
///
/// Based on Python: `filter_false_positive_matches()` (lines 1950-1970)
fn filter_false_positive_matches(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    let mut filtered = Vec::new();

    for m in matches {
        if let Some(rid) = parse_rule_id(&m.rule_identifier)
            && index.false_positive_rids.contains(&rid)
        {
            continue;
        }

        filtered.push(m.clone());
    }

    filtered
}

/// Update match scores for all matches.
///
/// Ensures all matches have correctly computed scores using the formula:
/// `score = match_coverage * rule_relevance / 100`
///
/// This function is idempotent - safe to call multiple times.
///
/// # Arguments
/// * `matches` - Mutable slice of LicenseMatch to update
///
/// Based on Python: LicenseMatch.score() method and score calculation in refine_matches()
fn update_match_scores(matches: &mut [LicenseMatch]) {
    for m in matches.iter_mut() {
        m.score = m.match_coverage * m.rule_relevance as f32 / 100.0;
    }
}

fn is_false_positive(m: &LicenseMatch, index: &LicenseIndex) -> bool {
    parse_rule_id(&m.rule_identifier)
        .map(|rid| index.false_positive_rids.contains(&rid))
        .unwrap_or(false)
}

/// Filter spurious matches with low density.
///
/// Spurious matches are matches with low density (where the matched tokens
/// are separated by many unmatched tokens). This filter only applies to
/// sequence and unknown matcher types - exact matches are always kept.
///
/// Based on Python: `filter_spurious_matches()` (match.py:1768-1836)
fn filter_spurious_matches(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    matches
        .iter()
        .filter(|m| {
            let is_seq_or_unknown = m.matcher == "3-seq" || m.matcher == "5-unknown";
            if !is_seq_or_unknown {
                return true;
            }

            let qdens = m.qdensity();
            let idens = m.idensity();
            let mlen = m.matched_length;
            let hilen = m.hilen();

            if mlen < 10 && (qdens < 0.1 || idens < 0.1) {
                return false;
            }
            if mlen < 15 && (qdens < 0.2 || idens < 0.2) {
                return false;
            }
            if mlen < 20 && hilen < 5 && (qdens < 0.3 || idens < 0.3) {
                return false;
            }
            if mlen < 30 && hilen < 8 && (qdens < 0.4 || idens < 0.4) {
                return false;
            }
            if qdens < 0.4 || idens < 0.4 {
                return false;
            }

            true
        })
        .cloned()
        .collect()
}

fn licensing_contains_match(current: &LicenseMatch, other: &LicenseMatch) -> bool {
    if current.license_expression.is_empty() || other.license_expression.is_empty() {
        return false;
    }
    licensing_contains(&current.license_expression, &other.license_expression)
}

pub fn filter_overlapping_matches(
    matches: Vec<LicenseMatch>,
    index: &LicenseIndex,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    if matches.len() < 2 {
        return (matches, vec![]);
    }

    let mut matches = matches;
    let mut discarded: Vec<LicenseMatch> = vec![];

    matches.sort_by(|a, b| {
        a.start_token
            .cmp(&b.start_token)
            .then_with(|| b.hilen.cmp(&a.hilen))
            .then_with(|| b.matched_length.cmp(&a.matched_length))
            .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
            .then_with(|| a.rule_identifier.cmp(&b.rule_identifier))
    });

    let mut i = 0;
    while i < matches.len().saturating_sub(1) {
        let mut j = i + 1;
        while j < matches.len() {
            let current_end = matches[i].end_token;
            let next_start = matches[j].start_token;

            if next_start >= current_end {
                break;
            }

            let both_fp =
                is_false_positive(&matches[i], index) && is_false_positive(&matches[j], index);
            if both_fp {
                j += 1;
                continue;
            }

            let overlap = matches[i].qoverlap(&matches[j]);
            if overlap == 0 {
                j += 1;
                continue;
            }

            let next_len = matches[j].matched_length;
            let current_len = matches[i].matched_length;

            if next_len == 0 || current_len == 0 {
                j += 1;
                continue;
            }

            let overlap_ratio_to_next = overlap as f64 / next_len as f64;
            let overlap_ratio_to_current = overlap as f64 / current_len as f64;

            let extra_large_next = overlap_ratio_to_next >= OVERLAP_EXTRA_LARGE;
            let large_next = overlap_ratio_to_next >= OVERLAP_LARGE;
            let medium_next = overlap_ratio_to_next >= OVERLAP_MEDIUM;
            let small_next = overlap_ratio_to_next >= OVERLAP_SMALL;

            let extra_large_current = overlap_ratio_to_current >= OVERLAP_EXTRA_LARGE;
            let large_current = overlap_ratio_to_current >= OVERLAP_LARGE;
            let medium_current = overlap_ratio_to_current >= OVERLAP_MEDIUM;
            let small_current = overlap_ratio_to_current >= OVERLAP_SMALL;

            let current_len_val = matches[i].matched_length;
            let next_len_val = matches[j].matched_length;
            let current_hilen = matches[i].hilen();
            let next_hilen = matches[j].hilen();

            if extra_large_next && current_len_val >= next_len_val {
                discarded.push(matches.remove(j));
                continue;
            }

            if extra_large_current && current_len_val <= next_len_val {
                discarded.push(matches.remove(i));
                i = i.saturating_sub(1);
                break;
            }

            if large_next && current_len_val >= next_len_val && current_hilen >= next_hilen {
                discarded.push(matches.remove(j));
                continue;
            }

            if large_current && current_len_val <= next_len_val && current_hilen <= next_hilen {
                discarded.push(matches.remove(i));
                i = i.saturating_sub(1);
                break;
            }

            if medium_next {
                if licensing_contains_match(&matches[i], &matches[j])
                    && current_len_val >= next_len_val
                    && current_hilen >= next_hilen
                {
                    discarded.push(matches.remove(j));
                    continue;
                }

                if licensing_contains_match(&matches[j], &matches[i])
                    && current_len_val <= next_len_val
                    && current_hilen <= next_hilen
                {
                    discarded.push(matches.remove(i));
                    i = i.saturating_sub(1);
                    break;
                }

                if next_len_val == 2
                    && current_len_val >= next_len_val + 2
                    && current_hilen >= next_hilen
                {
                    let current_ends = parse_rule_id(&matches[i].rule_identifier)
                        .and_then(|rid| index.rules_by_rid.get(rid))
                        .map(|r| r.ends_with_license)
                        .unwrap_or(false);
                    let next_starts = parse_rule_id(&matches[j].rule_identifier)
                        .and_then(|rid| index.rules_by_rid.get(rid))
                        .map(|r| r.starts_with_license)
                        .unwrap_or(false);

                    if current_ends && next_starts {
                        discarded.push(matches.remove(j));
                        continue;
                    }
                }
            }

            if medium_current {
                if licensing_contains_match(&matches[i], &matches[j])
                    && current_len_val >= next_len_val
                    && current_hilen >= next_hilen
                {
                    discarded.push(matches.remove(j));
                    continue;
                }

                if licensing_contains_match(&matches[j], &matches[i])
                    && current_len_val <= next_len_val
                    && current_hilen <= next_hilen
                {
                    discarded.push(matches.remove(i));
                    i = i.saturating_sub(1);
                    break;
                }
            }

            if small_next
                && matches[i].surround(&matches[j])
                && licensing_contains_match(&matches[i], &matches[j])
                && current_len_val >= next_len_val
                && current_hilen >= next_hilen
            {
                discarded.push(matches.remove(j));
                continue;
            }

            if small_current
                && matches[j].surround(&matches[i])
                && licensing_contains_match(&matches[j], &matches[i])
                && current_len_val <= next_len_val
                && current_hilen <= next_hilen
            {
                discarded.push(matches.remove(i));
                i = i.saturating_sub(1);
                break;
            }

            if i > 0 {
                let prev_end = matches[i - 1].end_token;
                let next_match_start = matches[j].start_token;

                let prev_next_overlap = if prev_end > next_match_start {
                    prev_end.saturating_sub(next_match_start.max(matches[i - 1].start_token))
                } else {
                    0
                };

                if prev_next_overlap == 0 {
                    let cpo = matches[i].qoverlap(&matches[i - 1]);
                    let cno = matches[i].qoverlap(&matches[j]);

                    if cpo > 0 && cno > 0 {
                        let overlap_len = cpo + cno;
                        let clen = matches[i].matched_length;

                        if overlap_len as f64 >= clen as f64 * 0.9 {
                            discarded.push(matches.remove(i));
                            i = i.saturating_sub(1);
                            break;
                        }
                    }
                }
            }

            j += 1;
        }
        i += 1;
    }

    (matches, discarded)
}

fn match_to_span(m: &LicenseMatch) -> Span {
    Span::from_range(m.start_line..m.end_line + 1)
}

pub fn restore_non_overlapping(
    kept: &[LicenseMatch],
    discarded: Vec<LicenseMatch>,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    let all_matched_qspans = kept
        .iter()
        .fold(Span::new(), |acc, m| acc.union_span(&match_to_span(m)));

    let mut to_keep = Vec::new();
    let mut to_discard = Vec::new();

    let merged_discarded = merge_overlapping_matches(&discarded);

    for disc in merged_discarded {
        let disc_span = match_to_span(&disc);
        if !disc_span.intersects(&all_matched_qspans) {
            to_keep.push(disc);
        } else {
            to_discard.push(disc);
        }
    }

    (to_keep, to_discard)
}

fn is_candidate_false_positive(m: &LicenseMatch) -> bool {
    let is_tag_or_ref =
        m.is_license_reference || m.is_license_tag || m.is_license_intro || m.is_license_clue;

    let is_not_spdx_id = m.matcher != "1-spdx-id";
    let is_exact_match = (m.match_coverage - 100.0).abs() < f32::EPSILON;
    let is_short = m.matched_length <= MAX_CANDIDATE_LENGTH;

    is_tag_or_ref && is_not_spdx_id && is_exact_match && is_short
}

fn count_unique_licenses(matches: &[LicenseMatch]) -> usize {
    let mut seen = std::collections::HashSet::new();
    for m in matches {
        seen.insert(&m.license_expression);
    }
    seen.len()
}

fn is_list_of_false_positives(
    matches: &[LicenseMatch],
    min_matches: usize,
    min_unique_licenses: usize,
    min_unique_licenses_proportion: f64,
    min_candidate_proportion: f64,
) -> bool {
    if matches.is_empty() {
        return false;
    }

    let len_matches = matches.len();

    let is_long_enough_sequence = len_matches >= min_matches;

    let len_unique_licenses = count_unique_licenses(matches);
    let unique_proportion = len_unique_licenses as f64 / len_matches as f64;
    let mut has_enough_licenses = unique_proportion > min_unique_licenses_proportion;

    if !has_enough_licenses {
        has_enough_licenses = len_unique_licenses >= min_unique_licenses;
    }

    let has_enough_candidates = if min_candidate_proportion > 0.0 {
        let candidates_count = matches
            .iter()
            .filter(|m| is_candidate_false_positive(m))
            .count();
        (candidates_count as f64 / len_matches as f64) > min_candidate_proportion
    } else {
        true
    };

    is_long_enough_sequence && has_enough_licenses && has_enough_candidates
}

#[allow(dead_code)]
fn match_distance(a: &LicenseMatch, b: &LicenseMatch) -> usize {
    if a.start_line <= b.end_line && b.start_line <= a.end_line {
        return 0;
    }

    let a_end = a.end_line + 1;
    let b_end = b.end_line + 1;

    if a_end == b.start_line || b_end == a.start_line {
        return 1;
    }

    if a_end < b.start_line {
        b.start_line - a_end
    } else {
        a.start_line - b_end
    }
}

pub fn filter_false_positive_license_lists_matches(
    matches: Vec<LicenseMatch>,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    let len_matches = matches.len();

    if len_matches < MIN_SHORT_FP_LIST_LENGTH {
        return (matches, vec![]);
    }

    if len_matches > MIN_LONG_FP_LIST_LENGTH
        && is_list_of_false_positives(
            &matches,
            MIN_LONG_FP_LIST_LENGTH,
            MIN_LONG_FP_LIST_LENGTH,
            MIN_UNIQUE_LICENSES_PROPORTION,
            0.95,
        )
    {
        return (vec![], matches);
    }

    let mut kept = Vec::new();
    let mut discarded = Vec::new();
    let mut candidates: Vec<&LicenseMatch> = Vec::new();

    for match_item in &matches {
        let is_candidate = is_candidate_false_positive(match_item);

        if is_candidate {
            let is_close_enough = candidates
                .last()
                .map(|last| last.qdistance_to(match_item) <= MAX_DISTANCE_BETWEEN_CANDIDATES)
                .unwrap_or(true);

            if is_close_enough {
                candidates.push(match_item);
            } else {
                let owned: Vec<LicenseMatch> = candidates.iter().map(|m| (*m).clone()).collect();
                if is_list_of_false_positives(
                    &owned,
                    MIN_SHORT_FP_LIST_LENGTH,
                    MIN_UNIQUE_LICENSES,
                    MIN_UNIQUE_LICENSES_PROPORTION,
                    0.0,
                ) {
                    discarded.extend(owned);
                } else {
                    kept.extend(owned);
                }
                candidates.clear();
                candidates.push(match_item);
            }
        } else {
            let owned: Vec<LicenseMatch> = candidates.iter().map(|m| (*m).clone()).collect();
            if is_list_of_false_positives(
                &owned,
                MIN_SHORT_FP_LIST_LENGTH,
                MIN_UNIQUE_LICENSES,
                MIN_UNIQUE_LICENSES_PROPORTION,
                0.0,
            ) {
                discarded.extend(owned);
            } else {
                kept.extend(owned);
            }
            candidates.clear();
            kept.push(match_item.clone());
        }
    }

    let owned: Vec<LicenseMatch> = candidates.iter().map(|m| (*m).clone()).collect();
    if is_list_of_false_positives(
        &owned,
        MIN_SHORT_FP_LIST_LENGTH,
        MIN_UNIQUE_LICENSES,
        MIN_UNIQUE_LICENSES_PROPORTION,
        0.0,
    ) {
        discarded.extend(owned);
    } else {
        kept.extend(owned);
    }

    (kept, discarded)
}

/// Filter matches below rule's minimum_coverage threshold.
///
/// Rules can have a `minimum_coverage` attribute that specifies the minimum
/// match coverage required for the match to be valid. Matches with coverage
/// below this threshold should be discarded.
///
/// This filter only applies to sequence matches (matcher == "3-seq").
/// Exact matches (hash, aho, spdx) are always kept.
///
/// # Arguments
/// * `index` - LicenseIndex containing rules_by_rid
/// * `matches` - Slice of LicenseMatch to filter
///
/// # Returns
/// Vector of LicenseMatch with below-minimum-coverage matches removed
///
/// Based on Python: `filter_below_rule_minimum_coverage()` (lines 1551-1587)
fn filter_below_rule_minimum_coverage(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    matches
        .iter()
        .filter(|m| {
            if m.matcher != "3-seq" {
                return true;
            }

            if let Some(rid) = parse_rule_id(&m.rule_identifier)
                && let Some(rule) = index.rules_by_rid.get(rid)
                && let Some(min_cov) = rule.minimum_coverage
            {
                return m.match_coverage >= min_cov as f32;
            }

            true
        })
        .cloned()
        .collect()
}

/// Filter short matches scattered on too many lines.
///
/// Short matches that are scattered across more lines than their token count
/// are likely spurious and should be filtered. For example, a 3-token match
/// spanning 50 lines is probably not a valid license reference.
///
/// This filter only applies to small rules (rule.is_small == true).
/// License tag rules get a +2 tolerance on matched_len comparison.
///
/// # Arguments
/// * `index` - LicenseIndex containing rules_by_rid
/// * `matches` - Slice of LicenseMatch to filter
///
/// # Returns
/// Vector of LicenseMatch with scattered matches removed
///
/// Based on Python: `filter_short_matches_scattered_on_too_many_lines()` (lines 1931-1972)
fn filter_short_matches_scattered_on_too_many_lines(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    if matches.len() == 1 {
        return matches.to_vec();
    }

    matches
        .iter()
        .filter(|m| {
            if let Some(rid) = parse_rule_id(&m.rule_identifier)
                && let Some(rule) = index.rules_by_rid.get(rid)
                && rule.is_small
            {
                let matched_len = m.len();
                let line_span = m.end_line.saturating_sub(m.start_line) + 1;

                let effective_matched_len = if rule.is_license_tag {
                    matched_len + 2
                } else {
                    matched_len
                };

                if line_span > effective_matched_len {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect()
}

/// Filter matches that are missing required phrases.
///
/// A match to a rule with required phrases ({{...}} markers) must contain
/// all those required phrases in the matched region. If any required phrase
/// is missing or interrupted by unknown/stopwords, the match is discarded.
///
/// This also handles:
/// - `is_continuous` rules: the entire match must be continuous
/// - `is_required_phrase` rules: same as is_continuous
///
/// # Arguments
/// * `index` - LicenseIndex containing rules_by_rid
/// * `matches` - Slice of LicenseMatch to filter
/// * `query` - Query object for unknowns_by_pos and stopwords_by_pos
///
/// # Returns
/// Tuple of (kept matches, discarded matches)
///
/// Based on Python: `filter_matches_missing_required_phrases()` (match.py:2154-2328)
fn filter_matches_missing_required_phrases(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
    query: &Query,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    if matches.is_empty() {
        return (Vec::new(), Vec::new());
    }

    // NOTE: Python has a solo match exception at lines 2172-2175, but it has a bug:
    // `rule = matches[0]` assigns a LicenseMatch (not a Rule), so `rule.is_continuous`
    // is a method object (always truthy). The exception never triggers.
    // We intentionally skip the solo exception to match Python's actual behavior.

    let mut kept = Vec::new();
    let mut discarded = Vec::new();

    for m in matches {
        let rid = match parse_rule_id(&m.rule_identifier) {
            Some(rid) => rid,
            None => {
                kept.push(m.clone());
                continue;
            }
        };

        let rule = match index.rules_by_rid.get(rid) {
            Some(r) => r,
            None => {
                kept.push(m.clone());
                continue;
            }
        };

        let is_continuous = rule.is_continuous || rule.is_required_phrase;
        let ikey_spans = &rule.required_phrase_spans;

        // No required phrases and not continuous -> always keep
        if ikey_spans.is_empty() && !is_continuous {
            kept.push(m.clone());
            continue;
        }

        // is_continuous but match is not continuous -> discard
        if is_continuous && !m.is_continuous(query) {
            discarded.push(m.clone());
            continue;
        }

        let ispan = m.ispan();
        let ispan_set: HashSet<usize> = ispan.iter().copied().collect();
        let qspan = m.qspan();

        // Determine the actual spans to check
        if is_continuous {
            if !ispan.is_empty() {
                let qkey_span: Vec<usize> = qspan.clone();

                if let Some(_qkey_end) = qkey_span.last() {
                    let contains_unknown = qkey_span
                        .iter()
                        .take(qkey_span.len() - 1)
                        .any(|&qpos| query.unknowns_by_pos.contains_key(&Some(qpos as i32)));

                    if contains_unknown {
                        discarded.push(m.clone());
                        continue;
                    }
                }

                let qkey_span_set: HashSet<usize> = qkey_span.iter().copied().collect();
                let qkey_span_end = qkey_span.last().copied();

                let has_same_stopwords = {
                    let mut ok = true;
                    for (&qpos, &ipos) in qspan.iter().zip(ispan.iter()) {
                        if !qkey_span_set.contains(&qpos) || Some(qpos) == qkey_span_end {
                            continue;
                        }

                        let i_stop = rule.stopwords_by_pos.get(&ipos).copied().unwrap_or(0);
                        let q_stop = query
                            .stopwords_by_pos
                            .get(&Some(qpos as i32))
                            .copied()
                            .unwrap_or(0);

                        if i_stop != q_stop {
                            ok = false;
                            break;
                        }
                    }
                    ok
                };

                if !has_same_stopwords {
                    discarded.push(m.clone());
                    continue;
                }
            }
            kept.push(m.clone());
            continue;
        }

        // Non-continuous case: check if all required phrase spans are contained in ispan
        let all_contained = ikey_spans
            .iter()
            .all(|span| (span.start..span.end).all(|pos| ispan_set.contains(&pos)));

        if !all_contained {
            discarded.push(m.clone());
            continue;
        }

        let mut is_valid = true;

        for ikey_span in ikey_spans {
            let qkey_span: Vec<usize> = qspan
                .iter()
                .zip(ispan.iter())
                .filter_map(|(&qpos, &ipos)| {
                    if ikey_span.contains(&ipos) {
                        Some(qpos)
                    } else {
                        None
                    }
                })
                .collect();

            if qkey_span.len() > 1 {
                for i in 1..qkey_span.len() {
                    if qkey_span[i] != qkey_span[i - 1] + 1 {
                        is_valid = false;
                        break;
                    }
                }
                if !is_valid {
                    break;
                }
            }

            if let Some(_qkey_end) = qkey_span.last() {
                let contains_unknown = qkey_span
                    .iter()
                    .take(qkey_span.len() - 1)
                    .any(|&qpos| query.unknowns_by_pos.contains_key(&Some(qpos as i32)));

                if contains_unknown {
                    is_valid = false;
                    break;
                }
            }

            let qkey_span_set: HashSet<usize> = qkey_span.iter().copied().collect();
            let qkey_span_end = qkey_span.last().copied();

            let has_same_stopwords = {
                let mut ok = true;
                for (&qpos, &ipos) in qspan.iter().zip(ispan.iter()) {
                    if !qkey_span_set.contains(&qpos) || Some(qpos) == qkey_span_end {
                        continue;
                    }

                    let i_stop = rule.stopwords_by_pos.get(&ipos).copied().unwrap_or(0);
                    let q_stop = query
                        .stopwords_by_pos
                        .get(&Some(qpos as i32))
                        .copied()
                        .unwrap_or(0);

                    if i_stop != q_stop {
                        ok = false;
                        break;
                    }
                }
                ok
            };

            if !has_same_stopwords {
                is_valid = false;
                break;
            }
        }

        if is_valid {
            kept.push(m.clone());
        } else {
            discarded.push(m.clone());
        }
    }

    (kept, discarded)
}

/// Filter single-token matches surrounded by many unknown/short/digit tokens.
///
/// A "spurious" single token match is a match to a single token that is
/// surrounded on both sides by at least `unknown_count` tokens that are either
/// unknown tokens, short tokens composed of a single character, tokens
/// composed only of digits or several punctuations and stopwords.
///
/// This filter only applies to sequence matches (matcher == "3-seq") with
/// exactly 1 matched token.
///
/// # Arguments
/// * `matches` - Slice of LicenseMatch to filter
/// * `query` - Query object containing unknowns_by_pos and shorts_and_digits_pos
/// * `unknown_count` - Minimum number of surrounding unknown/short tokens (default: 5)
///
/// # Returns
/// Vector of LicenseMatch with spurious single-token matches removed
///
/// Based on Python: `filter_matches_to_spurious_single_token()` (lines 1622-1700)
fn filter_matches_to_spurious_single_token(
    matches: &[LicenseMatch],
    query: &Query,
    unknown_count: usize,
) -> Vec<LicenseMatch> {
    matches
        .iter()
        .filter(|m| {
            if m.matcher != "3-seq" {
                return true;
            }
            if m.len() != 1 {
                return true;
            }

            let qstart = m.start_token;

            let before = query
                .unknowns_by_pos
                .get(&Some(qstart as i32 - 1))
                .copied()
                .unwrap_or(0)
                + (qstart.saturating_sub(unknown_count)..qstart)
                    .filter(|p| query.shorts_and_digits_pos.contains(p))
                    .count();

            if before < unknown_count {
                return true;
            }

            let after = query
                .unknowns_by_pos
                .get(&Some(qstart as i32))
                .copied()
                .unwrap_or(0)
                + (qstart + 1..qstart + 1 + unknown_count)
                    .filter(|p| query.shorts_and_digits_pos.contains(p))
                    .count();

            if after >= unknown_count {
                return false;
            }

            true
        })
        .cloned()
        .collect()
}

/// Check if a matched text is a valid short match.
///
/// A short match is valid if:
/// - The matched text equals the rule text (exact match)
/// - The matched text equals rule text when normalized (whitespace)
/// - For rules >= 5 chars, all matches are considered valid
/// - Length difference equals max_diff (allowed extra chars)
/// - Matched text is title case or same case throughout
/// - Rule text is contained in matched text
///
/// # Arguments
/// * `matched_text` - The matched text from the document
/// * `rule_text` - The rule text to compare against
/// * `max_diff` - Maximum allowed length difference
///
/// Based on Python: `is_valid_short_match()` (lines 1975-2123)
fn is_valid_short_match(matched_text: &str, rule_text: &str, max_diff: usize) -> bool {
    let matched = matched_text.trim();
    let rule = rule_text.trim();

    if matched == rule {
        return true;
    }

    let normalized_matched: String = matched.split_whitespace().collect::<Vec<_>>().join(" ");
    let normalized_rule: String = rule.split_whitespace().collect::<Vec<_>>().join(" ");

    if normalized_matched == normalized_rule {
        return true;
    }

    if normalized_rule.len() >= 5 {
        return true;
    }

    let diff_len = normalized_matched.len().abs_diff(normalized_rule.len());
    if diff_len > 0 && diff_len != max_diff {
        return false;
    }

    let (matched_check, rule_check) = if rule.ends_with('+') {
        (matched.trim_end_matches('+'), rule.trim_end_matches('+'))
    } else {
        (matched, rule)
    };

    let is_title_case = matched_check
        .chars()
        .next()
        .map(|c| c.is_ascii_uppercase())
        .unwrap_or(false)
        && matched_check
            .chars()
            .skip(1)
            .all(|c| !c.is_ascii_uppercase());

    if is_title_case {
        return true;
    }

    let is_same_case = matched_check.to_lowercase() == matched_check
        || matched_check.to_uppercase() == matched_check;

    if is_same_case {
        return true;
    }

    if matched_check.contains(rule_check) {
        return true;
    }

    false
}

/// Filter invalid matches to single-word gibberish in binary files.
///
/// Filters gibberish matches considered as invalid under these conditions:
/// - The scanned file is a binary file
/// - The matched rule has a single word (length_unique == 1)
/// - The matched rule "is_license_reference" or "is_license_clue"
/// - The matched rule has a low relevance (< 80)
/// - The matched text has leading/trailing punctuation or mixed case issues
///
/// # Arguments
/// * `index` - LicenseIndex containing rules_by_rid
/// * `matches` - Slice of LicenseMatch to filter
/// * `query` - Query object with is_binary flag
///
/// # Returns
/// Vector of LicenseMatch with gibberish matches removed
///
/// Based on Python: `filter_invalid_matches_to_single_word_gibberish()` (lines 1839-1901)
fn filter_invalid_matches_to_single_word_gibberish(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
    query: &Query,
) -> Vec<LicenseMatch> {
    if !query.is_binary {
        return matches.to_vec();
    }

    matches
        .iter()
        .filter(|m| {
            if let Some(rid) = parse_rule_id(&m.rule_identifier)
                && let Some(rule) = index.rules_by_rid.get(rid)
                && rule.length_unique == 1
                && (rule.is_license_reference || rule.is_license_clue)
                && let Some(matched_text) = &m.matched_text
            {
                let max_diff = if rule.relevance >= 80 { 1 } else { 0 };

                if !is_valid_short_match(matched_text, &rule.text, max_diff) {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect()
}

/// Main refinement function - applies all refinement operations to match results.
///
/// This is the main entry point for Phase 4.6 match refinement. It applies
/// filters in the same order as Python's refine_matches():
///
/// 1. Filter matches missing required phrases
/// 2. Filter spurious matches (low density)
/// 3. Filter below rule minimum coverage
/// 4. Filter spurious single-token matches
/// 5. Filter too short matches
/// 6. Filter scattered short matches
/// 7. Filter invalid single-word gibberish (binary files)
/// 8. Merge overlapping/adjacent matches
/// 9. Filter contained matches
/// 10. Filter overlapping matches
/// 11. Restore non-overlapping discarded matches
/// 12. Filter false positive matches
/// 13. Filter false positive license list matches
/// 14. Update match scores
///
/// The operations are applied in sequence to produce final refined matches.
///
/// # Arguments
/// * `index` - LicenseIndex containing false_positive_rids and rules_by_rid
/// * `matches` - Vector of raw LicenseMatch from all strategies
/// * `query` - Query object for spurious/gibberish filtering
///
/// # Returns
/// Vector of refined LicenseMatch ready for detection assembly
///
/// Based on Python: `refine_matches()` (lines 2691-2833)
pub fn refine_matches(
    index: &LicenseIndex,
    matches: Vec<LicenseMatch>,
    query: &Query,
) -> Vec<LicenseMatch> {
    if matches.is_empty() {
        return Vec::new();
    }

    // Python: merge_matches FIRST (line 2719), then filter, then merge again (line 2773)
    let merged = merge_overlapping_matches(&matches);

    // Filter matches missing required phrases
    let (with_required_phrases, _missing_phrases) =
        filter_matches_missing_required_phrases(index, &merged, query);

    let non_spurious = filter_spurious_matches(&with_required_phrases);

    let above_min_cov = filter_below_rule_minimum_coverage(index, &non_spurious);

    let non_single_spurious = filter_matches_to_spurious_single_token(&above_min_cov, query, 5);

    let non_short = filter_too_short_matches(index, &non_single_spurious);

    let non_scattered = filter_short_matches_scattered_on_too_many_lines(index, &non_short);

    let non_gibberish =
        filter_invalid_matches_to_single_word_gibberish(index, &non_scattered, query);

    // Python: merge_matches again at line 2773
    let merged_again = merge_overlapping_matches(&non_gibberish);

    let (non_contained, discarded_contained) = filter_contained_matches(&merged_again);

    let (kept, discarded_overlapping) = filter_overlapping_matches(non_contained, index);

    let mut matches_after_first_restore = kept.clone();

    if !discarded_contained.is_empty() {
        let (restored_contained, _) = restore_non_overlapping(&kept, discarded_contained);
        matches_after_first_restore.extend(restored_contained);
    }

    let mut final_matches = matches_after_first_restore.clone();

    if !discarded_overlapping.is_empty() {
        let (restored_overlapping, _) =
            restore_non_overlapping(&matches_after_first_restore, discarded_overlapping);
        final_matches.extend(restored_overlapping);
    }

    let (non_contained_final, _) = filter_contained_matches(&final_matches);

    let non_fp = filter_false_positive_matches(index, &non_contained_final);

    let (kept, _discarded) = filter_false_positive_license_lists_matches(non_fp);

    let merged_final = merge_overlapping_matches(&kept);
    let mut final_scored = merged_final;
    update_match_scores(&mut final_scored);

    final_scored
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_match(
        rule_identifier: &str,
        start_line: usize,
        end_line: usize,
        score: f32,
        coverage: f32,
        relevance: u8,
    ) -> LicenseMatch {
        LicenseMatch {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: None,
            start_line,
            end_line,
            start_token: start_line,
            end_token: end_line + 1,
            matcher: "2-aho".to_string(),
            score,
            matched_length: 100,
            rule_length: 100,
            matched_token_positions: None,
            match_coverage: coverage,
            rule_relevance: relevance,
            rule_identifier: rule_identifier.to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: None,
            referenced_filenames: None,
            is_license_intro: false,
            is_license_clue: false,
            is_license_reference: false,
            is_license_tag: false,
            is_license_text: false,
            hilen: 50,
            rule_start_token: 0,
            qspan_positions: None,
            ispan_positions: None,
        }
    }

    fn create_test_match_with_tokens(
        rule_identifier: &str,
        start_token: usize,
        end_token: usize,
        matched_length: usize,
    ) -> LicenseMatch {
        LicenseMatch {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: None,
            start_line: start_token,
            end_line: end_token.saturating_sub(1),
            start_token,
            end_token,
            matcher: "2-aho".to_string(),
            score: 1.0,
            matched_length,
            rule_length: matched_length,
            match_coverage: 100.0,
            rule_relevance: 100,
            rule_identifier: rule_identifier.to_string(),
            rule_url: "https://example.com".to_string(),
            matched_text: None,
            referenced_filenames: None,
            is_license_intro: false,
            is_license_clue: false,
            is_license_reference: false,
            is_license_tag: false,
            is_license_text: false,
            matched_token_positions: None,
            hilen: matched_length / 2,
            rule_start_token: 0,
            qspan_positions: None,
            ispan_positions: None,
        }
    }

    #[test]
    fn test_parse_rule_id_valid_hashes() {
        assert_eq!(parse_rule_id("#0"), Some(0));
        assert_eq!(parse_rule_id("#1"), Some(1));
        assert_eq!(parse_rule_id("#42"), Some(42));
        assert_eq!(parse_rule_id("#100"), Some(100));
        assert_eq!(parse_rule_id("#999"), Some(999));
    }

    #[test]
    fn test_parse_rule_id_plain_numbers() {
        assert_eq!(parse_rule_id("0"), Some(0));
        assert_eq!(parse_rule_id("42"), Some(42));
        assert_eq!(parse_rule_id("100"), Some(100));
    }

    #[test]
    fn test_parse_rule_id_invalid_formats() {
        assert_eq!(parse_rule_id(""), None);
        assert_eq!(parse_rule_id("#"), None);
        assert_eq!(parse_rule_id("#-1"), None);
        assert_eq!(parse_rule_id("invalid"), None);
        assert_eq!(parse_rule_id("#abc"), None);
        assert_eq!(parse_rule_id("mit.LICENSE"), None);
    }

    #[test]
    fn test_merge_overlapping_matches_same_rule() {
        let matches = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#1", 5, 15, 0.85, 85.0, 100),
        ];

        let merged = merge_overlapping_matches(&matches);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].rule_identifier, "#1");
        assert_eq!(merged[0].start_line, 1);
        assert_eq!(merged[0].end_line, 15);
        assert_eq!(merged[0].score, 0.9);
    }

    #[test]
    fn test_merge_adjacent_matches_same_rule() {
        let matches = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#1", 11, 20, 0.85, 85.0, 100),
        ];

        let merged = merge_overlapping_matches(&matches);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].rule_identifier, "#1");
        assert_eq!(merged[0].start_line, 1);
        assert_eq!(merged[0].end_line, 20);
        assert_eq!(merged[0].score, 0.9);
    }

    #[test]
    fn test_merge_no_overlap_different_rules() {
        let matches = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#2", 5, 15, 0.85, 85.0, 100),
        ];

        let merged = merge_overlapping_matches(&matches);

        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn test_merge_no_overlap_same_rule() {
        let matches = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#1", 20, 30, 0.85, 85.0, 100),
        ];

        let merged = merge_overlapping_matches(&matches);

        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn test_merge_multiple_matches_same_rule() {
        let matches = vec![
            create_test_match("#1", 1, 5, 0.8, 80.0, 100),
            create_test_match("#1", 6, 10, 0.9, 90.0, 100),
            create_test_match("#1", 11, 15, 0.85, 85.0, 100),
        ];

        let merged = merge_overlapping_matches(&matches);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].start_line, 1);
        assert_eq!(merged[0].end_line, 15);
    }

    #[test]
    fn test_merge_empty_matches() {
        let matches: Vec<LicenseMatch> = vec![];
        let merged = merge_overlapping_matches(&matches);
        assert_eq!(merged.len(), 0);
    }

    #[test]
    fn test_merge_single_match() {
        let matches = vec![create_test_match("#1", 1, 10, 0.9, 90.0, 100)];
        let merged = merge_overlapping_matches(&matches);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].start_line, 1);
        assert_eq!(merged[0].end_line, 10);
    }

    #[test]
    fn test_filter_contained_matches_simple() {
        let matches = vec![
            create_test_match("#1", 1, 20, 0.9, 90.0, 100),
            create_test_match("#1", 5, 15, 0.85, 85.0, 100),
        ];

        let (filtered, _) = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].start_line, 1);
        assert_eq!(filtered[0].end_line, 20);
    }

    #[test]
    fn test_filter_contained_matches_multiple() {
        let matches = vec![
            create_test_match("#1", 1, 30, 0.9, 90.0, 100),
            create_test_match("#1", 5, 10, 0.8, 80.0, 100),
            create_test_match("#1", 15, 20, 0.85, 85.0, 100),
        ];

        let (filtered, _) = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].start_line, 1);
        assert_eq!(filtered[0].end_line, 30);
    }

    #[test]
    fn test_filter_contained_matches_different_rules() {
        let mut matches = vec![
            create_test_match("#1", 1, 20, 0.9, 90.0, 100),
            create_test_match("#2", 5, 15, 0.85, 85.0, 100),
        ];
        matches[0].matched_length = 200;
        matches[1].matched_length = 100;

        let (filtered, _) = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].rule_identifier, "#1");
    }

    #[test]
    fn test_filter_contained_matches_no_containment() {
        let matches = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#1", 15, 25, 0.85, 85.0, 100),
        ];

        let (filtered, _) = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_contained_matches_empty() {
        let matches: Vec<LicenseMatch> = vec![];
        let (filtered, _) = filter_contained_matches(&matches);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_contained_matches_single() {
        let matches = vec![create_test_match("#1", 1, 10, 0.9, 90.0, 100)];
        let (filtered, _) = filter_contained_matches(&matches);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_false_positive_matches_with_false_positive() {
        let mut index = LicenseIndex::with_legalese_count(10);
        let _ = index.false_positive_rids.insert(42);

        let matches = vec![
            create_test_match("#42", 1, 10, 0.9, 90.0, 100),
            create_test_match("#1", 15, 25, 0.85, 85.0, 100),
        ];

        let filtered = filter_false_positive_matches(&index, &matches);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].rule_identifier, "#1");
    }

    #[test]
    fn test_filter_false_positive_matches_no_false_positive() {
        let index = LicenseIndex::with_legalese_count(10);

        let matches = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#2", 15, 25, 0.85, 85.0, 100),
        ];

        let filtered = filter_false_positive_matches(&index, &matches);

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_false_positive_matches_all_false_positive() {
        let mut index = LicenseIndex::with_legalese_count(10);
        let _ = index.false_positive_rids.insert(42);
        let _ = index.false_positive_rids.insert(43);

        let matches = vec![
            create_test_match("#42", 1, 10, 0.9, 90.0, 100),
            create_test_match("#43", 15, 25, 0.85, 85.0, 100),
        ];

        let filtered = filter_false_positive_matches(&index, &matches);

        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_false_positive_matches_empty() {
        let index = LicenseIndex::with_legalese_count(10);
        let matches: Vec<LicenseMatch> = vec![];
        let filtered = filter_false_positive_matches(&index, &matches);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_update_match_scores_basic() {
        let mut matches = vec![create_test_match("#1", 1, 10, 0.5, 50.0, 100)];

        update_match_scores(&mut matches);

        assert_eq!(matches[0].score, 50.0);
    }

    #[test]
    fn test_update_match_scores_multiple() {
        let mut matches = vec![
            create_test_match("#1", 1, 10, 0.5, 50.0, 80),
            create_test_match("#2", 15, 25, 0.5, 50.0, 100),
        ];

        update_match_scores(&mut matches);

        assert_eq!(matches[0].score, 40.0);
        assert_eq!(matches[1].score, 50.0);
    }

    #[test]
    fn test_update_match_scores_idempotent() {
        let mut matches = vec![create_test_match("#1", 1, 10, 50.0, 50.0, 100)];

        update_match_scores(&mut matches);
        let score1 = matches[0].score;

        update_match_scores(&mut matches);
        let score2 = matches[0].score;

        assert_eq!(score1, score2);
    }

    #[test]
    fn test_update_match_scores_empty() {
        let mut matches: Vec<LicenseMatch> = vec![];
        update_match_scores(&mut matches);
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_refine_matches_full_pipeline() {
        let mut index = LicenseIndex::with_legalese_count(10);
        let _ = index.false_positive_rids.insert(99);

        let matches = vec![
            create_test_match("#1", 1, 10, 0.5, 50.0, 100),
            create_test_match("#1", 5, 15, 0.5, 50.0, 100),
            create_test_match("#2", 20, 25, 0.5, 50.0, 80),
            create_test_match("#99", 30, 35, 0.5, 50.0, 100),
        ];

        let query = Query::new("test text", &index).unwrap();
        let refined = refine_matches(&index, matches, &query);

        assert_eq!(refined.len(), 2);

        let rule1_match = refined.iter().find(|m| m.rule_identifier == "#1").unwrap();
        assert_eq!(rule1_match.start_line, 1);
        assert_eq!(rule1_match.end_line, 15);

        let rule2_match = refined.iter().find(|m| m.rule_identifier == "#2").unwrap();
        assert_eq!(rule2_match.score, 40.0);
    }

    #[test]
    fn test_refine_matches_empty() {
        let index = LicenseIndex::with_legalese_count(10);
        let matches: Vec<LicenseMatch> = vec![];
        let query = Query::new("", &index).unwrap();

        let refined = refine_matches(&index, matches, &query);

        assert_eq!(refined.len(), 0);
    }

    #[test]
    fn test_refine_matches_single() {
        let index = LicenseIndex::with_legalese_count(10);
        let matches = vec![create_test_match("#1", 1, 10, 0.5, 50.0, 100)];
        let query = Query::new("test text", &index).unwrap();

        let refined = refine_matches(&index, matches, &query);

        assert_eq!(refined.len(), 1);
        assert_eq!(refined[0].score, 50.0);
    }

    #[test]
    fn test_refine_matches_no_merging_needed() {
        let index = LicenseIndex::with_legalese_count(10);

        let matches = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#2", 20, 30, 0.85, 85.0, 100),
        ];

        let query = Query::new("test text", &index).unwrap();

        let refined = refine_matches(&index, matches, &query);

        assert_eq!(refined.len(), 2);
    }

    #[test]
    fn test_filter_spurious_matches_keeps_non_seq_matchers() {
        let matches = vec![
            LicenseMatch {
                matcher: "1-hash".to_string(),
                matched_length: 5,
                ..create_test_match("#1", 1, 10, 1.0, 100.0, 100)
            },
            LicenseMatch {
                matcher: "2-aho".to_string(),
                matched_length: 5,
                ..create_test_match("#2", 1, 10, 1.0, 100.0, 100)
            },
        ];

        let filtered = filter_spurious_matches(&matches);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_spurious_matches_keeps_high_density_seq() {
        let mut m = create_test_match("#1", 1, 10, 1.0, 100.0, 100);
        m.matcher = "3-seq".to_string();
        m.matched_length = 50;
        m.matched_token_positions = Some((0..50).collect());

        let matches = vec![m];
        let filtered = filter_spurious_matches(&matches);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_spurious_matches_filters_low_density_short() {
        let mut m = create_test_match("#1", 1, 10, 1.0, 100.0, 100);
        m.matcher = "3-seq".to_string();
        m.matched_length = 5;
        m.start_token = 0;
        m.end_token = 100;
        m.matched_token_positions = Some(vec![0, 50, 75, 80, 99]);

        let matches = vec![m];
        let filtered = filter_spurious_matches(&matches);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_spurious_matches_filters_unknown_matcher() {
        let mut m = create_test_match("#1", 1, 10, 1.0, 100.0, 100);
        m.matcher = "5-unknown".to_string();
        m.matched_length = 5;
        m.start_token = 0;
        m.end_token = 100;
        m.matched_token_positions = Some(vec![0, 50, 75, 80, 99]);

        let matches = vec![m];
        let filtered = filter_spurious_matches(&matches);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_spurious_matches_keeps_medium_length() {
        let mut m = create_test_match("#1", 1, 10, 1.0, 100.0, 100);
        m.matcher = "3-seq".to_string();
        m.matched_length = 25;
        m.start_token = 0;
        m.end_token = 30;
        m.matched_token_positions = Some((0..25).collect());
        m.hilen = 10;

        let matches = vec![m];
        let filtered = filter_spurious_matches(&matches);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_spurious_matches_empty() {
        let matches: Vec<LicenseMatch> = vec![];
        let filtered = filter_spurious_matches(&matches);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_merge_partially_overlapping_matches_same_rule() {
        let matches = vec![
            create_test_match("#1", 1, 15, 0.9, 90.0, 100),
            create_test_match("#1", 10, 25, 0.85, 85.0, 100),
        ];

        let merged = merge_overlapping_matches(&matches);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].start_line, 1);
        assert_eq!(merged[0].end_line, 25);
    }

    #[test]
    fn test_merge_matches_with_gap_larger_than_one() {
        let matches = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#1", 15, 25, 0.85, 85.0, 100),
        ];

        let merged = merge_overlapping_matches(&matches);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].start_line, 1);
        assert_eq!(merged[0].end_line, 10);
        assert_eq!(merged[1].start_line, 15);
        assert_eq!(merged[1].end_line, 25);
    }

    #[test]
    fn test_merge_preserves_max_score() {
        let matches = vec![
            create_test_match("#1", 1, 10, 0.7, 70.0, 100),
            create_test_match("#1", 5, 15, 0.95, 95.0, 100),
            create_test_match("#1", 12, 20, 0.8, 80.0, 100),
        ];

        let merged = merge_overlapping_matches(&matches);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].score, 0.95);
    }

    #[test]
    fn test_filter_contained_matches_partial_overlap_no_containment() {
        let mut m1 = create_test_match("#1", 1, 20, 0.9, 90.0, 100);
        m1.matched_length = 150;
        let mut m2 = create_test_match("#2", 15, 30, 0.85, 85.0, 100);
        m2.matched_length = 100;
        let matches = vec![m1, m2];

        let (filtered, _) = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_contained_matches_equal_start_different_end() {
        let mut m1 = create_test_match("#1", 1, 30, 0.9, 90.0, 100);
        m1.matched_length = 200;
        let mut m2 = create_test_match("#2", 1, 15, 0.85, 85.0, 100);
        m2.matched_length = 100;
        let matches = vec![m1, m2];

        let (filtered, _) = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].end_line, 30);
    }

    #[test]
    fn test_filter_contained_matches_nested_containment() {
        let mut outer = create_test_match("#1", 1, 50, 0.9, 90.0, 100);
        outer.matched_length = 300;
        let mut middle = create_test_match("#2", 10, 40, 0.85, 85.0, 100);
        middle.matched_length = 200;
        let mut inner = create_test_match("#3", 15, 35, 0.8, 80.0, 100);
        inner.matched_length = 100;
        let matches = vec![inner, middle, outer];

        let (filtered, _) = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].start_line, 1);
        assert_eq!(filtered[0].end_line, 50);
    }

    #[test]
    fn test_filter_contained_matches_same_boundaries_different_matched_length() {
        let mut m1 = create_test_match("#1", 1, 10, 0.9, 90.0, 100);
        m1.matched_length = 200;
        let mut m2 = create_test_match("#2", 1, 10, 0.85, 85.0, 100);
        m2.matched_length = 100;
        let matches = vec![m1, m2];

        let (filtered, _) = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].matched_length, 200);
    }

    #[test]
    fn test_filter_contained_matches_token_positions_fully_contained() {
        let outer = create_test_match_with_tokens("#1", 0, 20, 20);
        let inner = create_test_match_with_tokens("#2", 5, 15, 10);
        let matches = vec![outer, inner];

        let (filtered, _) = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].start_token, 0);
        assert_eq!(filtered[0].end_token, 20);
    }

    #[test]
    fn test_filter_contained_matches_token_positions_partial_overlap_not_contained() {
        let m1 = create_test_match_with_tokens("#1", 0, 10, 10);
        let m2 = create_test_match_with_tokens("#2", 5, 15, 10);
        let matches = vec![m1, m2];

        let (filtered, _) = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_contained_matches_token_positions_non_overlapping() {
        let m1 = create_test_match_with_tokens("#1", 0, 10, 10);
        let m2 = create_test_match_with_tokens("#2", 20, 30, 10);
        let matches = vec![m1, m2];

        let (filtered, _) = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_contained_matches_token_positions_nested_containment() {
        let outer = create_test_match_with_tokens("#1", 0, 50, 50);
        let middle = create_test_match_with_tokens("#2", 10, 40, 30);
        let inner = create_test_match_with_tokens("#3", 15, 35, 20);
        let matches = vec![inner, middle, outer];

        let (filtered, _) = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].start_token, 0);
        assert_eq!(filtered[0].end_token, 50);
    }

    #[test]
    fn test_filter_contained_matches_token_positions_same_boundaries() {
        let m1 = create_test_match_with_tokens("#1", 0, 10, 10);
        let m2 = create_test_match_with_tokens("#2", 0, 10, 10);
        let matches = vec![m1, m2];

        let (filtered, _) = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_contained_matches_token_positions_multiple_contained() {
        let outer = create_test_match_with_tokens("#1", 0, 100, 100);
        let inner1 = create_test_match_with_tokens("#2", 10, 20, 10);
        let inner2 = create_test_match_with_tokens("#3", 30, 40, 10);
        let inner3 = create_test_match_with_tokens("#4", 50, 60, 10);
        let matches = vec![outer, inner1, inner2, inner3];

        let (filtered, _) = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].start_token, 0);
        assert_eq!(filtered[0].end_token, 100);
    }

    #[test]
    fn test_filter_contained_matches_gpl_variant_issue() {
        let gpl_1_0 = create_test_match_with_tokens("#20560", 10, 19, 9);
        let gpl_2_0 = create_test_match_with_tokens("#16218", 10, 32, 22);
        let matches = vec![gpl_1_0.clone(), gpl_2_0.clone()];

        let (filtered, _) = filter_contained_matches(&matches);

        assert_eq!(filtered.len(), 1, "Should filter contained GPL match");
        assert_eq!(
            filtered[0].rule_identifier, "#16218",
            "Should keep gpl-2.0-plus"
        );
        assert_eq!(filtered[0].end_token, 32, "Should have correct end_token");
    }

    #[test]
    fn test_filter_contained_matches_gpl_variant_zero_tokens() {
        let mut gpl_1_0 = create_test_match_with_tokens("#20560", 0, 0, 9);
        gpl_1_0.start_line = 13;
        gpl_1_0.end_line = 14;

        let mut gpl_2_0 = create_test_match_with_tokens("#16218", 0, 0, 22);
        gpl_2_0.start_line = 13;
        gpl_2_0.end_line = 15;

        let matches = vec![gpl_1_0.clone(), gpl_2_0.clone()];

        let (filtered, _) = filter_contained_matches(&matches);

        assert_eq!(
            filtered.len(),
            1,
            "Should filter contained GPL match (line-based)"
        );
        assert_eq!(
            filtered[0].rule_identifier, "#16218",
            "Should keep gpl-2.0-plus"
        );
    }

    #[test]
    fn debug_gpl_token_positions_real() {
        use crate::license_detection::aho_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };

        let rules_path =
            std::path::Path::new("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            std::path::Path::new("reference/scancode-toolkit/src/licensedcode/data/licenses");
        let rules = load_rules_from_directory(rules_path, false).expect("Failed to load rules");
        let licenses =
            load_licenses_from_directory(licenses_path, false).expect("Failed to load licenses");
        let index = build_index(rules, licenses);

        let text =
            std::fs::read_to_string("testdata/license-golden/datadriven/lic1/gpl-2.0-plus_1.txt")
                .unwrap();
        let query = Query::new(&text, &index).unwrap();
        let run = query.whole_query_run();

        let matches = aho_match::aho_match(&index, &run);

        let gpl_matches: Vec<_> = matches
            .iter()
            .filter(|m| m.license_expression.to_lowercase().contains("gpl"))
            .collect();

        println!("\nGPL matches with token positions:");
        for m in &gpl_matches {
            println!(
                "  {} start_token={}, end_token={}, lines {}-{}, len={}",
                m.license_expression,
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line,
                m.matched_length
            );
        }

        if gpl_matches.len() >= 2 {
            let m1 = &gpl_matches[0];
            let m2 = &gpl_matches[1];

            let same_start = m1.start_token == m2.start_token;
            println!("\nSame start_token? {}", same_start);

            println!("m1.qcontains(m2): {}", m1.qcontains(m2));
            println!("m2.qcontains(m1): {}", m2.qcontains(m1));
        }
    }

    #[test]
    fn test_refine_matches_pipeline_preserves_non_overlapping_different_rules() {
        let index = LicenseIndex::with_legalese_count(10);

        let matches = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#2", 20, 30, 0.85, 85.0, 100),
            create_test_match("#3", 40, 50, 0.8, 80.0, 100),
        ];

        let query = Query::new("test text", &index).unwrap();
        let refined = refine_matches(&index, matches, &query);

        assert_eq!(refined.len(), 3);
    }

    #[test]
    fn test_refine_matches_complex_scenario() {
        let mut index = LicenseIndex::with_legalese_count(10);
        let _ = index.false_positive_rids.insert(999);

        let mut m1 = create_test_match("#1", 1, 10, 0.7, 70.0, 100);
        m1.matched_length = 100;
        let mut m2 = create_test_match("#1", 8, 15, 0.8, 80.0, 100);
        m2.matched_length = 100;
        let mut m3 = create_test_match("#2", 20, 50, 0.9, 90.0, 100);
        m3.matched_length = 300;
        let mut m4 = create_test_match("#2", 25, 45, 0.85, 85.0, 100);
        m4.matched_length = 150;
        let m5 = create_test_match("#999", 60, 70, 0.9, 90.0, 100);

        let matches = vec![m1, m2, m3, m4, m5];

        let query = Query::new("test text", &index).unwrap();
        let refined = refine_matches(&index, matches, &query);

        assert_eq!(refined.len(), 2);

        let rule1_match = refined.iter().find(|m| m.rule_identifier == "#1").unwrap();
        assert_eq!(rule1_match.start_line, 1);
        assert_eq!(rule1_match.end_line, 15);

        let rule2_match = refined.iter().find(|m| m.rule_identifier == "#2").unwrap();
        assert_eq!(rule2_match.start_line, 20);
        assert_eq!(rule2_match.end_line, 50);
    }

    #[test]
    fn test_parse_rule_id_with_whitespace() {
        assert_eq!(parse_rule_id("  #42  "), Some(42));
        assert_eq!(parse_rule_id("  42  "), Some(42));
    }

    #[test]
    fn test_filter_false_positive_matches_mixed_identifiers() {
        let mut index = LicenseIndex::with_legalese_count(10);
        let _ = index.false_positive_rids.insert(42);

        let matches = vec![
            create_test_match("#42", 1, 10, 0.9, 90.0, 100),
            create_test_match("mit.LICENSE", 15, 25, 0.85, 85.0, 100),
            create_test_match("#1", 30, 40, 0.8, 80.0, 100),
        ];

        let filtered = filter_false_positive_matches(&index, &matches);

        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|m| m.rule_identifier == "mit.LICENSE"));
        assert!(filtered.iter().any(|m| m.rule_identifier == "#1"));
    }

    #[test]
    fn test_restore_non_overlapping_empty_both() {
        let kept: Vec<LicenseMatch> = vec![];
        let discarded: Vec<LicenseMatch> = vec![];

        let (to_keep, to_discard) = restore_non_overlapping(&kept, discarded);

        assert_eq!(to_keep.len(), 0);
        assert_eq!(to_discard.len(), 0);
    }

    #[test]
    fn test_restore_non_overlapping_empty_kept() {
        let kept: Vec<LicenseMatch> = vec![];
        let discarded = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#2", 20, 30, 0.85, 85.0, 100),
        ];

        let (to_keep, to_discard) = restore_non_overlapping(&kept, discarded);

        assert_eq!(to_keep.len(), 2);
        assert_eq!(to_discard.len(), 0);
    }

    #[test]
    fn test_restore_non_overlapping_empty_discarded() {
        let kept = vec![create_test_match("#1", 1, 10, 0.9, 90.0, 100)];
        let discarded: Vec<LicenseMatch> = vec![];

        let (to_keep, to_discard) = restore_non_overlapping(&kept, discarded);

        assert_eq!(to_keep.len(), 0);
        assert_eq!(to_discard.len(), 0);
    }

    #[test]
    fn test_restore_non_overlapping_non_overlapping_restored() {
        let kept = vec![create_test_match("#1", 1, 10, 0.9, 90.0, 100)];
        let discarded = vec![
            create_test_match("#2", 50, 60, 0.85, 85.0, 100),
            create_test_match("#3", 100, 110, 0.8, 80.0, 100),
        ];

        let (to_keep, to_discard) = restore_non_overlapping(&kept, discarded);

        assert_eq!(to_keep.len(), 2);
        assert_eq!(to_discard.len(), 0);
    }

    #[test]
    fn test_restore_non_overlapping_overlapping_not_restored() {
        let kept = vec![create_test_match("#1", 1, 20, 0.9, 90.0, 100)];
        let discarded = vec![
            create_test_match("#2", 5, 15, 0.85, 85.0, 100),
            create_test_match("#3", 10, 25, 0.8, 80.0, 100),
        ];

        let (to_keep, to_discard) = restore_non_overlapping(&kept, discarded);

        assert_eq!(to_keep.len(), 0);
        assert_eq!(to_discard.len(), 2);
    }

    #[test]
    fn test_restore_non_overlapping_partial_overlap() {
        let kept = vec![create_test_match("#1", 10, 20, 0.9, 90.0, 100)];
        let discarded = vec![
            create_test_match("#2", 1, 5, 0.85, 85.0, 100),
            create_test_match("#3", 15, 25, 0.8, 80.0, 100),
            create_test_match("#4", 50, 60, 0.9, 90.0, 100),
        ];

        let (to_keep, to_discard) = restore_non_overlapping(&kept, discarded);

        assert_eq!(to_keep.len(), 2);
        assert_eq!(to_discard.len(), 1);

        let kept_identifiers: Vec<&str> =
            to_keep.iter().map(|m| m.rule_identifier.as_str()).collect();
        assert!(kept_identifiers.contains(&"#2"));
        assert!(kept_identifiers.contains(&"#4"));

        assert_eq!(to_discard[0].rule_identifier, "#3");
    }

    #[test]
    fn test_restore_non_overlapping_multiple_kept() {
        let kept = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#2", 30, 40, 0.85, 85.0, 100),
        ];
        let discarded = vec![
            create_test_match("#3", 15, 20, 0.8, 80.0, 100),
            create_test_match("#4", 5, 15, 0.9, 90.0, 100),
            create_test_match("#5", 50, 60, 0.9, 90.0, 100),
        ];

        let (to_keep, to_discard) = restore_non_overlapping(&kept, discarded);

        assert_eq!(to_keep.len(), 2);
        assert_eq!(to_discard.len(), 1);

        let kept_identifiers: Vec<&str> =
            to_keep.iter().map(|m| m.rule_identifier.as_str()).collect();
        assert!(kept_identifiers.contains(&"#3"));
        assert!(kept_identifiers.contains(&"#5"));

        assert_eq!(to_discard[0].rule_identifier, "#4");
    }

    #[test]
    fn test_restore_non_overlapping_merges_discarded() {
        let kept = vec![create_test_match("#1", 1, 10, 0.9, 90.0, 100)];
        let discarded = vec![
            create_test_match("#2", 50, 60, 0.85, 85.0, 100),
            create_test_match("#2", 55, 65, 0.8, 80.0, 100),
        ];

        let (to_keep, _to_discard) = restore_non_overlapping(&kept, discarded);

        assert_eq!(to_keep.len(), 1);
        assert_eq!(to_keep[0].rule_identifier, "#2");
        assert_eq!(to_keep[0].start_line, 50);
        assert_eq!(to_keep[0].end_line, 65);
    }

    #[test]
    fn test_restore_non_overlapping_adjacent_not_overlapping() {
        let kept = vec![create_test_match("#1", 1, 10, 0.9, 90.0, 100)];
        let discarded = vec![create_test_match("#2", 11, 20, 0.85, 85.0, 100)];

        let (to_keep, to_discard) = restore_non_overlapping(&kept, discarded);

        assert_eq!(to_keep.len(), 1);
        assert_eq!(to_discard.len(), 0);
    }

    #[test]
    fn test_restore_non_overlapping_touching_is_overlapping() {
        let kept = vec![create_test_match("#1", 1, 10, 0.9, 90.0, 100)];
        let discarded = vec![create_test_match("#2", 10, 20, 0.85, 85.0, 100)];

        let (to_keep, to_discard) = restore_non_overlapping(&kept, discarded);

        assert_eq!(to_keep.len(), 0);
        assert_eq!(to_discard.len(), 1);
    }

    #[test]
    fn test_filter_overlapping_matches_empty() {
        let index = LicenseIndex::with_legalese_count(10);
        let matches: Vec<LicenseMatch> = vec![];

        let (kept, discarded) = filter_overlapping_matches(matches, &index);

        assert_eq!(kept.len(), 0);
        assert_eq!(discarded.len(), 0);
    }

    #[test]
    fn test_filter_overlapping_matches_single() {
        let index = LicenseIndex::with_legalese_count(10);
        let matches = vec![create_test_match("#1", 1, 10, 0.9, 90.0, 100)];

        let (kept, discarded) = filter_overlapping_matches(matches, &index);

        assert_eq!(kept.len(), 1);
        assert_eq!(discarded.len(), 0);
    }

    #[test]
    fn test_filter_overlapping_matches_non_overlapping() {
        let index = LicenseIndex::with_legalese_count(10);
        let matches = vec![
            create_test_match("#1", 1, 10, 0.9, 90.0, 100),
            create_test_match("#2", 20, 30, 0.85, 85.0, 100),
        ];

        let (kept, discarded) = filter_overlapping_matches(matches, &index);

        assert_eq!(kept.len(), 2);
        assert_eq!(discarded.len(), 0);
    }

    #[test]
    fn test_filter_overlapping_matches_extra_large_discard_shorter() {
        let index = LicenseIndex::with_legalese_count(10);
        let mut m1 = create_test_match("#1", 1, 100, 0.9, 90.0, 100);
        m1.matched_length = 100;
        let mut m2 = create_test_match("#2", 5, 100, 0.85, 85.0, 100);
        m2.matched_length = 10;

        let matches = vec![m1, m2];

        let (kept, discarded) = filter_overlapping_matches(matches, &index);

        assert_eq!(kept.len(), 1);
        assert_eq!(discarded.len(), 1);
        assert_eq!(kept[0].matched_length, 100);
    }

    #[test]
    fn test_filter_overlapping_matches_large_with_hilen() {
        let index = LicenseIndex::with_legalese_count(10);
        let mut m1 = create_test_match("#1", 1, 100, 0.9, 90.0, 100);
        m1.matched_length = 100;
        let mut m2 = create_test_match("#2", 30, 100, 0.85, 85.0, 100);
        m2.matched_length = 10;

        let matches = vec![m1, m2];

        let (kept, discarded) = filter_overlapping_matches(matches, &index);

        assert_eq!(kept.len(), 1);
        assert_eq!(discarded.len(), 1);
    }

    #[test]
    fn test_filter_overlapping_matches_false_positive_skip() {
        let mut index = LicenseIndex::with_legalese_count(10);
        let _ = index.false_positive_rids.insert(1);
        let _ = index.false_positive_rids.insert(2);

        let mut m1 = create_test_match("#1", 1, 20, 0.9, 90.0, 100);
        m1.matched_length = 100;
        let mut m2 = create_test_match("#2", 10, 30, 0.85, 85.0, 100);
        m2.matched_length = 100;

        let matches = vec![m1, m2];

        let (kept, discarded) = filter_overlapping_matches(matches, &index);

        assert_eq!(kept.len(), 2);
        assert_eq!(discarded.len(), 0);
    }

    #[test]
    fn test_filter_overlapping_matches_sandwich_detection() {
        let index = LicenseIndex::with_legalese_count(10);

        let mut prev = create_test_match("#1", 1, 10, 0.9, 90.0, 100);
        prev.matched_length = 100;
        let mut current = create_test_match("#2", 5, 15, 0.85, 85.0, 100);
        current.matched_length = 50;
        let mut next = create_test_match("#3", 12, 25, 0.8, 80.0, 100);
        next.matched_length = 100;

        let matches = vec![prev, current, next];

        let (kept, discarded) = filter_overlapping_matches(matches, &index);

        assert!(kept.len() >= 2);
        assert!(!discarded.is_empty() || kept.len() == 3);
    }

    #[test]
    fn test_filter_overlapping_matches_sorting_order() {
        let index = LicenseIndex::with_legalese_count(10);

        let m1 = create_test_match("#1", 5, 10, 0.9, 90.0, 100);
        let m2 = create_test_match("#2", 1, 20, 0.85, 85.0, 100);
        let m3 = create_test_match("#3", 25, 35, 0.8, 80.0, 100);

        let matches = vec![m1, m2, m3];

        let (kept, _) = filter_overlapping_matches(matches, &index);

        assert_eq!(kept.len(), 3);
        assert_eq!(kept[0].start_line, 1);
        assert_eq!(kept[1].start_line, 5);
        assert_eq!(kept[2].start_line, 25);
    }

    #[test]
    fn test_filter_overlapping_matches_partial_overlap_no_filter() {
        let index = LicenseIndex::with_legalese_count(10);

        let mut m1 = create_test_match("#1", 1, 20, 0.9, 90.0, 100);
        m1.matched_length = 200;
        let mut m2 = create_test_match("#2", 15, 35, 0.85, 85.0, 100);
        m2.matched_length = 150;

        let matches = vec![m1, m2];

        let (kept, discarded) = filter_overlapping_matches(matches, &index);

        assert_eq!(kept.len(), 2);
        assert_eq!(discarded.len(), 0);
    }

    #[test]
    fn test_filter_overlapping_matches_surround_check() {
        let index = LicenseIndex::with_legalese_count(10);

        let mut outer = create_test_match("#1", 1, 100, 0.9, 90.0, 100);
        outer.matched_length = 500;
        let mut inner = create_test_match("#2", 20, 30, 0.85, 85.0, 100);
        inner.matched_length = 50;

        let matches = vec![outer, inner];

        let (kept, discarded) = filter_overlapping_matches(matches, &index);

        assert_eq!(kept.len(), 1);
        assert_eq!(discarded.len(), 1);
        assert!(kept[0].rule_identifier == "#1" || kept[0].matched_length == 500);
    }

    #[test]
    fn test_calculate_overlap_no_overlap() {
        let m1 = create_test_match("#1", 1, 10, 0.9, 90.0, 100);
        let m2 = create_test_match("#2", 20, 30, 0.85, 85.0, 100);

        assert_eq!(m1.qoverlap(&m2), 0);
        assert_eq!(m2.qoverlap(&m1), 0);
    }

    #[test]
    fn test_calculate_overlap_partial() {
        let m1 = create_test_match("#1", 1, 10, 0.9, 90.0, 100);
        let m2 = create_test_match("#2", 5, 15, 0.85, 85.0, 100);

        assert_eq!(m1.qoverlap(&m2), 6);
        assert_eq!(m2.qoverlap(&m1), 6);
    }

    #[test]
    fn test_calculate_overlap_contained() {
        let m1 = create_test_match("#1", 1, 20, 0.9, 90.0, 100);
        let m2 = create_test_match("#2", 5, 15, 0.85, 85.0, 100);

        assert_eq!(m1.qoverlap(&m2), 11);
        assert_eq!(m2.qoverlap(&m1), 11);
    }

    #[test]
    fn test_calculate_overlap_identical() {
        let m1 = create_test_match("#1", 1, 10, 0.9, 90.0, 100);
        let m2 = create_test_match("#2", 1, 10, 0.85, 85.0, 100);

        assert_eq!(m1.qoverlap(&m2), 10);
    }

    #[test]
    fn test_calculate_overlap_adjacent() {
        let m1 = create_test_match("#1", 1, 10, 0.9, 90.0, 100);
        let m2 = create_test_match("#2", 11, 20, 0.85, 85.0, 100);

        assert_eq!(m1.qoverlap(&m2), 0);
    }

    #[allow(clippy::too_many_arguments)]
    fn create_test_match_with_flags(
        rule_identifier: &str,
        start_line: usize,
        end_line: usize,
        is_license_reference: bool,
        is_license_tag: bool,
        is_license_intro: bool,
        is_license_clue: bool,
        matcher: &str,
        match_coverage: f32,
        matched_length: usize,
        rule_length: usize,
        license_expression: &str,
    ) -> LicenseMatch {
        LicenseMatch {
            license_expression: license_expression.to_string(),
            license_expression_spdx: license_expression.to_string(),
            from_file: None,
            start_line,
            end_line,
            start_token: 0,
            end_token: 0,
            matcher: matcher.to_string(),
            score: 1.0,
            matched_length,
            rule_length,
            match_coverage,
            rule_relevance: 100,
            rule_identifier: rule_identifier.to_string(),
            rule_url: String::new(),
            matched_text: None,
            referenced_filenames: None,
            is_license_intro,
            is_license_clue,
            is_license_reference,
            is_license_tag,
            is_license_text: false,
            matched_token_positions: None,
            hilen: matched_length / 2,
            rule_start_token: 0,
            qspan_positions: None,
            ispan_positions: None,
        }
    }

    #[test]
    fn test_is_candidate_false_positive_tag_match() {
        let m = create_test_match_with_flags(
            "#1", 1, 1, false, true, false, false, "2-aho", 100.0, 5, 5, "mit",
        );
        assert!(is_candidate_false_positive(&m));
    }

    #[test]
    fn test_is_candidate_false_positive_reference_match() {
        let m = create_test_match_with_flags(
            "#2",
            1,
            1,
            true,
            false,
            false,
            false,
            "2-aho",
            100.0,
            3,
            3,
            "apache-2.0",
        );
        assert!(is_candidate_false_positive(&m));
    }

    #[test]
    fn test_is_candidate_false_positive_spdx_id_excluded() {
        let m = create_test_match_with_flags(
            "#3",
            1,
            1,
            true,
            false,
            false,
            false,
            "1-spdx-id",
            100.0,
            3,
            3,
            "mit",
        );
        assert!(!is_candidate_false_positive(&m));
    }

    #[test]
    fn test_is_candidate_false_partial_coverage_excluded() {
        let m = create_test_match_with_flags(
            "#4", 1, 1, true, false, false, false, "2-aho", 80.0, 5, 5, "mit",
        );
        assert!(!is_candidate_false_positive(&m));
    }

    #[test]
    fn test_is_candidate_false_long_match_excluded() {
        let m = create_test_match_with_flags(
            "#5", 1, 1, true, false, false, false, "2-aho", 100.0, 25, 25, "mit",
        );
        assert!(!is_candidate_false_positive(&m));
    }

    #[test]
    fn test_filter_short_list_not_filtered() {
        let matches: Vec<LicenseMatch> = (0..10)
            .map(|i| {
                create_test_match_with_flags(
                    &format!("#{}", i),
                    i + 1,
                    i + 1,
                    true,
                    false,
                    false,
                    false,
                    "2-aho",
                    100.0,
                    3,
                    3,
                    &format!("license-{}", i),
                )
            })
            .collect();

        let (kept, discarded) = filter_false_positive_license_lists_matches(matches);
        assert_eq!(kept.len(), 10);
        assert_eq!(discarded.len(), 0);
    }

    #[test]
    fn test_filter_long_list_all_candidates() {
        let matches: Vec<LicenseMatch> = (0..160)
            .map(|i| {
                create_test_match_with_flags(
                    &format!("#{}", i),
                    i + 1,
                    i + 1,
                    true,
                    false,
                    false,
                    false,
                    "2-aho",
                    100.0,
                    3,
                    3,
                    &format!("license-{}", i),
                )
            })
            .collect();

        let (kept, discarded) = filter_false_positive_license_lists_matches(matches);
        assert_eq!(kept.len(), 0);
        assert_eq!(discarded.len(), 160);
    }

    #[test]
    fn test_filter_mixed_list_keeps_non_candidates() {
        let mut matches = Vec::new();

        for i in 0..15 {
            matches.push(create_test_match_with_flags(
                &format!("#{}", i),
                i + 1,
                i + 1,
                true,
                false,
                false,
                false,
                "2-aho",
                100.0,
                3,
                3,
                &format!("license-{}", i),
            ));
        }

        for i in 0..5 {
            matches.push(create_test_match_with_flags(
                &format!("#{}", 100 + i),
                100 + i,
                100 + i + 20,
                false,
                false,
                false,
                false,
                "2-aho",
                100.0,
                100,
                100,
                "gpl-3.0",
            ));
        }

        let (kept, discarded) = filter_false_positive_license_lists_matches(matches);

        assert_eq!(kept.len(), 5);
        assert_eq!(discarded.len(), 15);
    }

    #[test]
    fn test_filter_candidates_with_real_license() {
        let mut matches = Vec::new();

        for i in 0..15 {
            matches.push(create_test_match_with_flags(
                &format!("#{}", i),
                i + 1,
                i + 1,
                true,
                false,
                false,
                false,
                "2-aho",
                100.0,
                3,
                3,
                &format!("license-{}", i),
            ));
        }

        matches.push(create_test_match_with_flags(
            "#real", 100, 150, false, false, false, false, "2-aho", 100.0, 200, 200, "mit",
        ));

        for i in 0..15 {
            matches.push(create_test_match_with_flags(
                &format!("#{}", 200 + i),
                200 + i,
                200 + i,
                true,
                false,
                false,
                false,
                "2-aho",
                100.0,
                3,
                3,
                &format!("license-{}", 200 + i),
            ));
        }

        let (kept, discarded) = filter_false_positive_license_lists_matches(matches);

        assert_eq!(kept.len(), 1);
        assert_eq!(discarded.len(), 30);
    }

    #[test]
    fn test_match_distance_overlapping() {
        let a = create_test_match_with_flags(
            "#1", 1, 10, false, false, false, false, "2-aho", 100.0, 10, 10, "mit",
        );
        let b = create_test_match_with_flags(
            "#2", 5, 15, false, false, false, false, "2-aho", 100.0, 10, 10, "mit",
        );
        assert_eq!(match_distance(&a, &b), 0);
    }

    #[test]
    fn test_match_distance_touching() {
        let a = create_test_match_with_flags(
            "#1", 1, 10, false, false, false, false, "2-aho", 100.0, 10, 10, "mit",
        );
        let b = create_test_match_with_flags(
            "#2", 11, 20, false, false, false, false, "2-aho", 100.0, 10, 10, "mit",
        );
        assert_eq!(match_distance(&a, &b), 1);
    }

    #[test]
    fn test_match_distance_gap() {
        let a = create_test_match_with_flags(
            "#1", 1, 10, false, false, false, false, "2-aho", 100.0, 10, 10, "mit",
        );
        let b = create_test_match_with_flags(
            "#2", 15, 25, false, false, false, false, "2-aho", 100.0, 10, 10, "mit",
        );
        assert_eq!(match_distance(&a, &b), 4);
    }

    #[test]
    fn test_count_unique_licenses() {
        let matches = vec![
            create_test_match_with_flags(
                "#1", 1, 1, false, false, false, false, "2-aho", 100.0, 5, 5, "mit",
            ),
            create_test_match_with_flags(
                "#2", 2, 2, false, false, false, false, "2-aho", 100.0, 5, 5, "mit",
            ),
            create_test_match_with_flags(
                "#3",
                3,
                3,
                false,
                false,
                false,
                false,
                "2-aho",
                100.0,
                5,
                5,
                "apache-2.0",
            ),
        ];
        assert_eq!(count_unique_licenses(&matches), 2);
    }

    #[test]
    fn test_min_unique_licenses_fallback() {
        let matches: Vec<LicenseMatch> = (0..20)
            .map(|i| {
                let mut m = create_test_match("#1", 10, 10, 1.0, 100.0, 100);
                m.license_expression = format!("license-{}", i % 4);
                m.is_license_reference = true;
                m
            })
            .collect();

        // 20 matches, 4 unique licenses
        // Proportion = 4/20 = 0.2 < 1/3, so proportion check fails
        // Fallback uses min_unique_licenses
        assert!(is_list_of_false_positives(&matches, 15, 3, 1.0 / 3.0, 0.0)); // 4 >= 3
        assert!(!is_list_of_false_positives(&matches, 15, 5, 1.0 / 3.0, 0.0)); // 4 < 5
    }

    #[test]
    fn test_filter_too_short_matches_non_seq_match_kept() {
        let index = LicenseIndex::with_legalese_count(10);

        let mut m = create_test_match("#1", 1, 10, 0.9, 50.0, 100);
        m.matcher = "2-aho".to_string();
        m.matched_length = 2;

        let matches = vec![m];
        let filtered = filter_too_short_matches(&index, &matches);

        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_too_short_matches_small_seq_match_filtered() {
        let mut index = LicenseIndex::with_legalese_count(10);
        index
            .rules_by_rid
            .push(crate::license_detection::models::Rule {
                identifier: "test".to_string(),
                license_expression: "mit".to_string(),
                text: "test".to_string(),
                tokens: vec![],
                is_license_text: false,
                is_license_notice: false,
                is_license_reference: false,
                is_license_tag: false,
                is_license_intro: false,
                is_license_clue: false,
                is_false_positive: false,
                is_required_phrase: false,
                is_from_license: false,
                relevance: 100,
                minimum_coverage: None,
                is_continuous: true,
                referenced_filenames: None,
                ignorable_urls: None,
                ignorable_emails: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                language: None,
                notes: None,
                length_unique: 0,
                high_length_unique: 0,
                high_length: 0,
                min_matched_length: 10,
                min_high_matched_length: 5,
                min_matched_length_unique: 0,
                min_high_matched_length_unique: 0,
                is_small: true,
                is_tiny: false,
                starts_with_license: false,
                ends_with_license: false,
                is_deprecated: false,
                spdx_license_key: None,
                other_spdx_license_keys: vec![],
                required_phrase_spans: vec![],
                stopwords_by_pos: std::collections::HashMap::new(),
            });

        let mut m = create_test_match("#0", 1, 10, 0.9, 50.0, 100);
        m.matcher = "3-seq".to_string();
        m.matched_length = 5;
        m.hilen = 2;

        let matches = vec![m];
        let filtered = filter_too_short_matches(&index, &matches);

        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_too_short_matches_large_seq_match_kept() {
        let mut index = LicenseIndex::with_legalese_count(10);
        index
            .rules_by_rid
            .push(crate::license_detection::models::Rule {
                identifier: "test".to_string(),
                license_expression: "mit".to_string(),
                text: "test".to_string(),
                tokens: vec![],
                is_license_text: false,
                is_license_notice: false,
                is_license_reference: false,
                is_license_tag: false,
                is_license_intro: false,
                is_license_clue: false,
                is_false_positive: false,
                is_required_phrase: false,
                is_from_license: false,
                relevance: 100,
                minimum_coverage: None,
                is_continuous: true,
                referenced_filenames: None,
                ignorable_urls: None,
                ignorable_emails: None,
                ignorable_copyrights: None,
                ignorable_holders: None,
                ignorable_authors: None,
                language: None,
                notes: None,
                length_unique: 0,
                high_length_unique: 0,
                high_length: 0,
                min_matched_length: 10,
                min_high_matched_length: 5,
                min_matched_length_unique: 0,
                min_high_matched_length_unique: 0,
                is_small: true,
                is_tiny: false,
                starts_with_license: false,
                ends_with_license: false,
                is_deprecated: false,
                spdx_license_key: None,
                other_spdx_license_keys: vec![],
                required_phrase_spans: vec![],
                stopwords_by_pos: std::collections::HashMap::new(),
            });

        let mut m = create_test_match("#0", 1, 10, 0.9, 90.0, 100);
        m.matcher = "3-seq".to_string();
        m.matched_length = 15;
        m.hilen = 8;

        let matches = vec![m];
        let filtered = filter_too_short_matches(&index, &matches);

        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_below_rule_minimum_coverage_keeps_non_seq() {
        let index = LicenseIndex::with_legalese_count(10);
        let mut m = create_test_match("#0", 1, 10, 0.9, 50.0, 100);
        m.matcher = "2-aho".to_string();

        let matches = vec![m];
        let filtered = filter_below_rule_minimum_coverage(&index, &matches);

        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_below_rule_minimum_coverage_filters_low_coverage() {
        use crate::license_detection::models::Rule;

        let mut index = LicenseIndex::with_legalese_count(10);
        index.rules_by_rid.push(Rule {
            identifier: "test".to_string(),
            license_expression: "mit".to_string(),
            text: "test".to_string(),
            tokens: vec![],
            is_license_text: false,
            is_license_notice: false,
            is_license_reference: false,
            is_license_tag: false,
            is_license_intro: false,
            is_license_clue: false,
            is_false_positive: false,
            is_required_phrase: false,
            is_from_license: false,
            relevance: 100,
            minimum_coverage: Some(80),
            is_continuous: true,
            referenced_filenames: None,
            ignorable_urls: None,
            ignorable_emails: None,
            ignorable_copyrights: None,
            ignorable_holders: None,
            ignorable_authors: None,
            language: None,
            notes: None,
            length_unique: 0,
            high_length_unique: 0,
            high_length: 0,
            min_matched_length: 10,
            min_high_matched_length: 5,
            min_matched_length_unique: 0,
            min_high_matched_length_unique: 0,
            is_small: false,
            is_tiny: false,
            starts_with_license: false,
            ends_with_license: false,
            is_deprecated: false,
            spdx_license_key: None,
            other_spdx_license_keys: vec![],
            required_phrase_spans: vec![],
            stopwords_by_pos: std::collections::HashMap::new(),
        });

        let mut m = create_test_match("#0", 1, 10, 0.9, 50.0, 100);
        m.matcher = "3-seq".to_string();

        let matches = vec![m];
        let filtered = filter_below_rule_minimum_coverage(&index, &matches);

        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_scattered_keeps_concentrated() {
        use crate::license_detection::models::Rule;

        let mut index = LicenseIndex::with_legalese_count(10);
        index.rules_by_rid.push(Rule {
            identifier: "test".to_string(),
            license_expression: "mit".to_string(),
            text: "test".to_string(),
            tokens: vec![],
            is_license_text: false,
            is_license_notice: false,
            is_license_reference: false,
            is_license_tag: false,
            is_license_intro: false,
            is_license_clue: false,
            is_false_positive: false,
            is_required_phrase: false,
            is_from_license: false,
            relevance: 100,
            minimum_coverage: None,
            is_continuous: true,
            referenced_filenames: None,
            ignorable_urls: None,
            ignorable_emails: None,
            ignorable_copyrights: None,
            ignorable_holders: None,
            ignorable_authors: None,
            language: None,
            notes: None,
            length_unique: 0,
            high_length_unique: 0,
            high_length: 0,
            min_matched_length: 10,
            min_high_matched_length: 5,
            min_matched_length_unique: 0,
            min_high_matched_length_unique: 0,
            is_small: true,
            is_tiny: false,
            starts_with_license: false,
            ends_with_license: false,
            is_deprecated: false,
            spdx_license_key: None,
            other_spdx_license_keys: vec![],
            required_phrase_spans: vec![],
            stopwords_by_pos: std::collections::HashMap::new(),
        });

        let m1 = create_test_match_with_tokens("#0", 0, 10, 10);
        let m2 = create_test_match_with_tokens("#0", 20, 30, 10);

        let matches = vec![m1, m2];
        let filtered = filter_short_matches_scattered_on_too_many_lines(&index, &matches);

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_scattered_filters_scattered() {
        use crate::license_detection::models::Rule;

        let mut index = LicenseIndex::with_legalese_count(10);
        index.rules_by_rid.push(Rule {
            identifier: "test".to_string(),
            license_expression: "mit".to_string(),
            text: "test".to_string(),
            tokens: vec![],
            is_license_text: false,
            is_license_notice: false,
            is_license_reference: false,
            is_license_tag: false,
            is_license_intro: false,
            is_license_clue: false,
            is_false_positive: false,
            is_required_phrase: false,
            is_from_license: false,
            relevance: 100,
            minimum_coverage: None,
            is_continuous: true,
            referenced_filenames: None,
            ignorable_urls: None,
            ignorable_emails: None,
            ignorable_copyrights: None,
            ignorable_holders: None,
            ignorable_authors: None,
            language: None,
            notes: None,
            length_unique: 0,
            high_length_unique: 0,
            high_length: 0,
            min_matched_length: 10,
            min_high_matched_length: 5,
            min_matched_length_unique: 0,
            min_high_matched_length_unique: 0,
            is_small: true,
            is_tiny: false,
            starts_with_license: false,
            ends_with_license: false,
            is_deprecated: false,
            spdx_license_key: None,
            other_spdx_license_keys: vec![],
            required_phrase_spans: vec![],
            stopwords_by_pos: std::collections::HashMap::new(),
        });

        let mut m = create_test_match_with_tokens("#0", 0, 3, 3);
        m.start_line = 1;
        m.end_line = 50;

        let mut m2 = create_test_match_with_tokens("#0", 10, 13, 3);
        m2.start_line = 1;
        m2.end_line = 50;

        let matches = vec![m, m2];
        let filtered = filter_short_matches_scattered_on_too_many_lines(&index, &matches);

        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_is_valid_short_match_exact() {
        assert!(is_valid_short_match("GPL", "GPL", 0));
        assert!(is_valid_short_match("gpl", "GPL", 0));
        assert!(is_valid_short_match("MIT", "MIT", 0));
    }

    #[test]
    fn test_is_valid_short_match_with_diff() {
        assert!(is_valid_short_match("gpl~", "GPL", 1));
        assert!(!is_valid_short_match("gpl~", "GPL", 0));
    }

    #[test]
    fn test_is_valid_short_match_rejects_punctuation() {
        assert!(!is_valid_short_match("~gpl", "GPL", 0));
        assert!(!is_valid_short_match("gpl)", "GPL", 0));
        assert!(is_valid_short_match("gpl+", "gpl+", 0));
    }

    #[test]
    fn test_is_valid_short_match_rejects_mixed_case() {
        assert!(!is_valid_short_match("gPl", "GPL", 0));
        assert!(is_valid_short_match("Gpl", "GPL", 0));
    }

    #[test]
    fn debug_gpl_2_0_9_required_phrases_filter() {
        use crate::license_detection::aho_match;
        use crate::license_detection::index::build_index;
        use crate::license_detection::query::Query;
        use crate::license_detection::rules::{
            load_licenses_from_directory, load_rules_from_directory,
        };

        let rules_path =
            std::path::Path::new("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            std::path::Path::new("reference/scancode-toolkit/src/licensedcode/data/licenses");
        let rules = load_rules_from_directory(rules_path, false).expect("Failed to load rules");
        let licenses =
            load_licenses_from_directory(licenses_path, false).expect("Failed to load licenses");
        let index = build_index(rules, licenses);

        let text = std::fs::read_to_string("testdata/license-golden/datadriven/lic1/gpl-2.0_9.txt")
            .unwrap();

        println!("\n=== ORIGINAL TEXT lines 45-46 ===");
        for (i, line) in text.lines().enumerate() {
            if (44..=46).contains(&i) {
                println!("{}: {:?}", i + 1, line);
            }
        }

        let query = Query::new(&text, &index).unwrap();
        let run = query.whole_query_run();

        // Check rule #20733 (gpl_66.RULE) details
        let rule_20733 = index.rules_by_rid.get(20733);
        if let Some(rule) = rule_20733 {
            println!("\n=== RULE #20733 (gpl_66.RULE) ===");
            println!("license_expression: {}", rule.license_expression);
            println!("text: {:?}", rule.text);
            println!("required_phrase_spans: {:?}", rule.required_phrase_spans);
            println!("is_license_notice: {}", rule.is_license_notice);
            println!("\n=== RULE #20733 STOPWORDS ===");
            for (&pos, &count) in &rule.stopwords_by_pos {
                println!("  pos {}: {} stopwords after", pos, count);
            }
        }

        let matches = aho_match::aho_match(&index, &run);

        println!("\n=== ALL MATCHES ({}) ===", matches.len());
        for m in &matches {
            let rule = index.rules_by_rid.get(
                m.rule_identifier
                    .as_str()
                    .trim_start_matches('#')
                    .parse::<usize>()
                    .unwrap_or(0),
            );
            println!(
                "  {} (rid={}): lines {}-{}, start_token={}, end_token={}, len={}",
                m.license_expression,
                m.rule_identifier,
                m.start_line,
                m.end_line,
                m.start_token,
                m.end_token,
                m.matched_length
            );
            if let Some(r) = rule {
                println!("    required_phrase_spans: {:?}", r.required_phrase_spans);
            }
        }

        // Check the gpl-1.0-plus match specifically (rid=#20733)
        let gpl_1_0_plus_match = matches.iter().find(|m| m.rule_identifier == "#20733");
        if let Some(m) = gpl_1_0_plus_match {
            println!("\n=== GPL-1.0-PLUS MATCH #20733 DETAILS ===");
            println!("start_token={}, end_token={}", m.start_token, m.end_token);
            println!(
                "matched_length={}, rule_start_token={}",
                m.matched_length, m.rule_start_token
            );

            // Show the ispan
            let ispan = m.ispan();
            println!("ispan: {:?}", ispan);

            // Show the qspan
            let qspan = m.qspan();
            println!("qspan: {:?}", qspan);

            // Check required phrase spans
            let rule = index.rules_by_rid.get(20733);
            if let Some(r) = rule {
                println!(
                    "\nChecking required_phrase_spans: {:?}",
                    r.required_phrase_spans
                );
                for rp_span in &r.required_phrase_spans {
                    let in_ispan = rp_span.clone().into_iter().all(|pos| ispan.contains(&pos));
                    println!("  span {:?} in ispan? {}", rp_span, in_ispan);

                    // Check qspan continuity for this required phrase
                    let qkey_positions: Vec<_> = qspan
                        .iter()
                        .zip(ispan.iter())
                        .filter(|(_, ipos)| rp_span.contains(*ipos))
                        .map(|(qpos, _)| *qpos)
                        .collect();
                    println!("    qkey_positions for this span: {:?}", qkey_positions);

                    // Check if continuous
                    if !qkey_positions.is_empty() {
                        let is_continuous = qkey_positions.windows(2).all(|w| w[1] == w[0] + 1);
                        println!("    is continuous in qspan? {}", is_continuous);
                    }
                }
            }
        }

        // Check the gpl-2.0 match #17911 (gpl-2.0_7.RULE)
        let gpl_2_0_match = matches.iter().find(|m| m.rule_identifier == "#17911");
        if let Some(m) = gpl_2_0_match {
            println!("\n=== GPL-2.0 MATCH #17911 DETAILS ===");
            println!("start_token={}, end_token={}", m.start_token, m.end_token);
            println!(
                "matched_length={}, rule_start_token={}",
                m.matched_length, m.rule_start_token
            );

            let ispan = m.ispan();
            println!("ispan length: {}", ispan.len());
            println!(
                "ispan first 30: {:?}",
                ispan.iter().take(30).collect::<Vec<_>>()
            );
            println!(
                "ispan last 10: {:?}",
                ispan.iter().rev().take(10).collect::<Vec<_>>()
            );

            // Check required phrase spans for rule #17911
            let rule = index.rules_by_rid.get(17911);
            if let Some(r) = rule {
                println!(
                    "\nChecking required_phrase_spans: {:?}",
                    r.required_phrase_spans
                );
                for rp_span in &r.required_phrase_spans {
                    let in_ispan = rp_span.clone().into_iter().all(|pos| ispan.contains(&pos));
                    println!("  span {:?} in ispan? {}", rp_span, in_ispan);

                    // Show which positions are missing
                    let missing: Vec<_> =
                        rp_span.clone().filter(|pos| !ispan.contains(pos)).collect();
                    if !missing.is_empty() {
                        println!("    MISSING positions: {:?}", missing);
                    }
                }
            }
        }

        let refined = refine_matches(&index, matches.clone(), &query);
        println!("\n=== FINAL REFINED ({}) ===", refined.len());
        for m in &refined {
            println!(
                "  {} (rid={}): lines {}-{}",
                m.license_expression, m.rule_identifier, m.start_line, m.end_line
            );
        }
    }
}
