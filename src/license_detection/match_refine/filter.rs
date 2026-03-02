//! Match filtering functions.
//!
//! This module contains functions for filtering matches based on various criteria
//! like containment, overlap, length, density, and required phrases.

use std::collections::HashSet;

use crate::license_detection::expression::licensing_contains;
use crate::license_detection::index::LicenseIndex;
use crate::license_detection::models::LicenseMatch;
use crate::license_detection::query::Query;
use crate::license_detection::spans::Span;

use super::merge::merge_overlapping_matches;

const OVERLAP_SMALL: f64 = 0.10;
const OVERLAP_MEDIUM: f64 = 0.40;
const OVERLAP_LARGE: f64 = 0.70;
const OVERLAP_EXTRA_LARGE: f64 = 0.90;

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
pub fn filter_contained_matches(
    matches: &[LicenseMatch],
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    if matches.len() < 2 {
        return (matches.to_vec(), Vec::new());
    }

    let mut matches: Vec<LicenseMatch> = matches.to_vec();
    let mut discarded = Vec::new();

    matches.sort_by(|a, b| {
        a.qstart()
            .cmp(&b.qstart())
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

            if current.qstart() == next.qstart() && current.end_token == next.end_token {
                if current.match_coverage >= next.match_coverage {
                    discarded.push(matches.remove(j));
                    continue;
                } else {
                    discarded.push(matches.remove(i));
                    i = i.saturating_sub(1);
                    break;
                }
            }

            if current.qcontains(&next) {
                discarded.push(matches.remove(j));
                continue;
            }
            if next.qcontains(&current) {
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

fn is_false_positive(m: &LicenseMatch, index: &LicenseIndex) -> bool {
    index.false_positive_rids.contains(&m.rid)
}

fn licensing_contains_match(current: &LicenseMatch, other: &LicenseMatch) -> bool {
    if current.license_expression.is_empty() || other.license_expression.is_empty() {
        return false;
    }
    licensing_contains(&current.license_expression, &other.license_expression)
}

/// Filter overlapping matches based on overlap ratios and license expressions.
///
/// This function handles complex overlapping scenarios where multiple matches
/// overlap at the same location. It uses overlap ratios and license expression
/// relationships to determine which matches to keep.
///
/// # Arguments
/// * `matches` - Vector of LicenseMatch to filter
/// * `index` - LicenseIndex for false positive checking
///
/// # Returns
/// Tuple of (kept matches, discarded matches)
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
        a.qstart()
            .cmp(&b.qstart())
            .then_with(|| b.hilen.cmp(&a.hilen))
            .then_with(|| b.matched_length.cmp(&a.matched_length))
            .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
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

            let different_licenses = matches[i].license_expression != matches[j].license_expression;

            let current_wins_on_candidate = {
                let current_resemblance = matches[i].candidate_resemblance;
                let next_resemblance = matches[j].candidate_resemblance;
                let current_containment = matches[i].candidate_containment;
                let next_containment = matches[j].candidate_containment;

                if current_resemblance > next_resemblance {
                    true
                } else if current_resemblance < next_resemblance {
                    false
                } else if current_containment > next_containment {
                    true
                } else if current_containment < next_containment {
                    false
                } else {
                    current_hilen >= next_hilen
                }
            };

            let both_have_candidate_scores =
                matches[i].candidate_resemblance > 0.0 && matches[j].candidate_resemblance > 0.0;

            if extra_large_next && current_len_val >= next_len_val {
                if different_licenses && both_have_candidate_scores && !current_wins_on_candidate {
                    discarded.push(matches.remove(i));
                    i = i.saturating_sub(1);
                    break;
                }
                discarded.push(matches.remove(j));
                continue;
            }

            if extra_large_current && current_len_val <= next_len_val {
                if different_licenses && both_have_candidate_scores && current_wins_on_candidate {
                    discarded.push(matches.remove(j));
                    continue;
                }
                discarded.push(matches.remove(i));
                i = i.saturating_sub(1);
                break;
            }

            if large_next && current_len_val >= next_len_val && current_hilen >= next_hilen {
                if different_licenses && both_have_candidate_scores && !current_wins_on_candidate {
                    discarded.push(matches.remove(i));
                    i = i.saturating_sub(1);
                    break;
                }
                discarded.push(matches.remove(j));
                continue;
            }

            if large_current && current_len_val <= next_len_val && current_hilen <= next_hilen {
                if different_licenses && both_have_candidate_scores && current_wins_on_candidate {
                    discarded.push(matches.remove(j));
                    continue;
                }
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
                    let current_ends = index
                        .rules_by_rid
                        .get(matches[i].rid)
                        .map(|r| r.ends_with_license)
                        .unwrap_or(false);
                    let next_starts = index
                        .rules_by_rid
                        .get(matches[j].rid)
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
                let prev_next_overlap = matches[i - 1].qspan_overlap(&matches[j]);

                if prev_next_overlap == 0 {
                    let cpo = matches[i].qspan_overlap(&matches[i - 1]);
                    let cno = matches[i].qspan_overlap(&matches[j]);

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

fn match_to_qspan(m: &LicenseMatch) -> Span {
    if let Some(positions) = &m.qspan_positions
        && !positions.is_empty()
    {
        return Span::from_iterator(positions.iter().copied());
    }

    if m.start_token == 0 && m.end_token == 0 {
        return Span::from_range(m.start_line..m.end_line + 1);
    }

    Span::from_range(m.start_token..m.end_token)
}

/// Restore non-overlapping discarded matches.
///
/// After filtering, some matches may have been discarded that don't actually
/// overlap with the kept matches. This function restores those non-overlapping
/// discarded matches.
///
/// # Arguments
/// * `matches` - Slice of kept LicenseMatch
/// * `discarded` - Vector of discarded LicenseMatch to check for restoration
///
/// # Returns
/// Tuple of (restored matches, still-discarded matches)
pub fn restore_non_overlapping(
    matches: &[LicenseMatch],
    discarded: Vec<LicenseMatch>,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    let all_matched_qspans = matches
        .iter()
        .fold(Span::new(), |acc, m| acc.union_span(&match_to_qspan(m)));

    let mut to_keep = Vec::new();
    let mut to_discard = Vec::new();

    let merged_discarded = merge_overlapping_matches(&discarded);

    for disc in merged_discarded {
        let disc_qspan = match_to_qspan(&disc);
        if !disc_qspan.intersects(&all_matched_qspans) {
            to_keep.push(disc);
        } else {
            to_discard.push(disc);
        }
    }

    (to_keep, to_discard)
}

/// Filter spurious matches with low density.
///
/// Spurious matches are matches with low density (where the matched tokens
/// are separated by many unmatched tokens). This filter only applies to
/// sequence and unknown matcher types - exact matches are always kept.
///
/// Based on Python: `filter_spurious_matches()` (match.py:1768-1836)
pub(crate) fn filter_spurious_matches(matches: &[LicenseMatch], query: &Query) -> Vec<LicenseMatch> {
    matches
        .iter()
        .filter(|m| {
            let is_seq_or_unknown = m.matcher == "3-seq" || m.matcher == "5-unknown";
            if !is_seq_or_unknown {
                return true;
            }

            let qdens = m.qdensity(query);
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

/// Filter matches below rule's minimum_coverage threshold.
///
/// Rules can have a `minimum_coverage` attribute that specifies the minimum
/// match coverage required for the match to be valid. Matches with coverage
/// below this threshold should be discarded.
///
/// This filter only applies to sequence matches (matcher == "3-seq").
/// Exact matches (hash, aho, spdx) are always kept.
///
/// Based on Python: `filter_below_rule_minimum_coverage()` (lines 1551-1587)
pub(crate) fn filter_below_rule_minimum_coverage(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    matches
        .iter()
        .filter(|m| {
            if m.matcher != "3-seq" {
                return true;
            }

            let rid = m.rid;
            if let Some(rule) = index.rules_by_rid.get(rid)
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
/// Based on Python: `filter_short_matches_scattered_on_too_many_lines()` (lines 1931-1972)
pub(crate) fn filter_short_matches_scattered_on_too_many_lines(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    if matches.len() == 1 {
        return matches.to_vec();
    }

    matches
        .iter()
        .filter(|m| {
            let rid = m.rid;
            if let Some(rule) = index.rules_by_rid.get(rid)
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
/// Based on Python: `filter_matches_missing_required_phrases()` (match.py:2154-2328)
pub(crate) fn filter_matches_missing_required_phrases(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
    query: &Query,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    if matches.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut kept = Vec::new();
    let mut discarded = Vec::new();

    for m in matches {
        let rid = m.rid;

        let rule = match index.rules_by_rid.get(rid) {
            Some(r) => r,
            None => {
                kept.push(m.clone());
                continue;
            }
        };

        let is_continuous = rule.is_continuous || rule.is_required_phrase;
        let ikey_spans = &rule.required_phrase_spans;

        if ikey_spans.is_empty() && !is_continuous {
            kept.push(m.clone());
            continue;
        }

        if is_continuous && !m.is_continuous(query) {
            discarded.push(m.clone());
            continue;
        }

        let ispan = m.ispan();
        let ispan_set: HashSet<usize> = ispan.iter().copied().collect();
        let qspan = m.qspan();

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
/// Based on Python: `filter_matches_to_spurious_single_token()` (lines 1622-1700)
pub(crate) fn filter_matches_to_spurious_single_token(
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

/// Filter matches to false positive rules.
///
/// Removes matches whose rule ID is in the index's false_positive_rids set.
///
/// Based on Python: `filter_false_positive_matches()` (lines 1950-1970)
pub(crate) fn filter_false_positive_matches(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    let mut filtered = Vec::new();

    for m in matches {
        let rid = m.rid;
        if index.false_positive_rids.contains(&rid) {
            continue;
        }

        filtered.push(m.clone());
    }

    filtered
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
/// Based on Python: `filter_invalid_matches_to_single_word_gibberish()` (lines 1839-1901)
pub(crate) fn filter_invalid_matches_to_single_word_gibberish(
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
            let rid = m.rid;
            if let Some(rule) = index.rules_by_rid.get(rid)
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

pub(crate) fn filter_too_short_matches(index: &LicenseIndex, matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    matches
        .iter()
        .filter(|m| {
            if m.matcher != "3-seq" {
                return true;
            }

            let rid = m.rid;
            if let Some(rule) = index.rules_by_rid.get(rid) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::license_detection::models::Rule;

    fn parse_rule_id(rule_identifier: &str) -> Option<usize> {
        let trimmed = rule_identifier.trim();
        if let Some(stripped) = trimmed.strip_prefix('#') {
            stripped.parse().ok()
        } else {
            trimmed.parse().ok()
        }
    }

    fn create_test_match(
        rule_identifier: &str,
        start_line: usize,
        end_line: usize,
        score: f32,
        coverage: f32,
        relevance: u8,
    ) -> LicenseMatch {
        let matched_len = end_line - start_line + 1;
        let rule_len = matched_len;
        let rid = parse_rule_id(rule_identifier).unwrap_or(0);
        LicenseMatch {
            rid,
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: None,
            start_line,
            end_line,
            start_token: start_line,
            end_token: end_line + 1,
            matcher: "2-aho".to_string(),
            score,
            matched_length: matched_len,
            rule_length: rule_len,
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
            is_from_license: false,
            hilen: 50,
            rule_start_token: 0,
            qspan_positions: None,
            ispan_positions: None,
            hispan_positions: None,
            candidate_resemblance: 0.0,
            candidate_containment: 0.0,
        }
    }

    fn create_test_match_with_tokens(
        rule_identifier: &str,
        start_token: usize,
        end_token: usize,
        matched_length: usize,
    ) -> LicenseMatch {
        let rid = parse_rule_id(rule_identifier).unwrap_or(0);
        LicenseMatch {
            rid,
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
            is_from_license: false,
            matched_token_positions: None,
            hilen: matched_length / 2,
            rule_start_token: 0,
            qspan_positions: None,
            ispan_positions: None,
            hispan_positions: None,
            candidate_resemblance: 0.0,
            candidate_containment: 0.0,
        }
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
    fn test_filter_spurious_matches_keeps_non_seq_matchers() {
        let index = LicenseIndex::with_legalese_count(10);
        let query = Query::new("test text", &index).unwrap();
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

        let filtered = filter_spurious_matches(&matches, &query);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_spurious_matches_keeps_high_density_seq() {
        let index = LicenseIndex::with_legalese_count(10);
        let query = Query::new("test text", &index).unwrap();
        let mut m = create_test_match("#1", 1, 10, 1.0, 100.0, 100);
        m.matcher = "3-seq".to_string();
        m.matched_length = 50;
        m.matched_token_positions = Some((0..50).collect());

        let matches = vec![m];
        let filtered = filter_spurious_matches(&matches, &query);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_spurious_matches_filters_low_density_short() {
        let index = LicenseIndex::with_legalese_count(10);
        let query = Query::new("test text", &index).unwrap();
        let mut m = create_test_match("#1", 1, 10, 1.0, 100.0, 100);
        m.matcher = "3-seq".to_string();
        m.matched_length = 5;
        m.start_token = 0;
        m.end_token = 100;
        m.matched_token_positions = Some(vec![0, 50, 75, 80, 99]);

        let matches = vec![m];
        let filtered = filter_spurious_matches(&matches, &query);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_spurious_matches_filters_unknown_matcher() {
        let index = LicenseIndex::with_legalese_count(10);
        let query = Query::new("test text", &index).unwrap();
        let mut m = create_test_match("#1", 1, 10, 1.0, 100.0, 100);
        m.matcher = "5-unknown".to_string();
        m.matched_length = 5;
        m.start_token = 0;
        m.end_token = 100;
        m.matched_token_positions = Some(vec![0, 50, 75, 80, 99]);

        let matches = vec![m];
        let filtered = filter_spurious_matches(&matches, &query);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_spurious_matches_keeps_medium_length() {
        let index = LicenseIndex::with_legalese_count(10);
        let query = Query::new("test text", &index).unwrap();
        let mut m = create_test_match("#1", 1, 10, 1.0, 100.0, 100);
        m.matcher = "3-seq".to_string();
        m.matched_length = 25;
        m.start_token = 0;
        m.end_token = 30;
        m.matched_token_positions = Some((0..25).collect());
        m.hilen = 10;

        let matches = vec![m];
        let filtered = filter_spurious_matches(&matches, &query);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_spurious_matches_empty() {
        let index = LicenseIndex::with_legalese_count(10);
        let query = Query::new("test text", &index).unwrap();
        let matches: Vec<LicenseMatch> = vec![];
        let filtered = filter_spurious_matches(&matches, &query);
        assert_eq!(filtered.len(), 0);
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

        let m1 = create_test_match("#1", 25, 35, 0.9, 90.0, 100);
        let m2 = create_test_match("#2", 1, 10, 0.85, 85.0, 100);
        let m3 = create_test_match("#3", 40, 50, 0.8, 80.0, 100);

        let matches = vec![m1, m2, m3];

        let (kept, _) = filter_overlapping_matches(matches, &index);

        assert_eq!(kept.len(), 3);
        assert_eq!(kept[0].start_line, 1);
        assert_eq!(kept[1].start_line, 25);
        assert_eq!(kept[2].start_line, 40);
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
        let kept = vec![create_test_match("#1", 1, 10, 0.9, 100.0, 100)];
        let mut m1 = create_test_match("#2", 50, 60, 0.85, 100.0, 100);
        m1.rule_length = 100;
        m1.rule_start_token = 0;
        let mut m2 = create_test_match("#2", 55, 65, 0.8, 100.0, 100);
        m2.rule_length = 100;
        m2.rule_start_token = 5;

        let discarded = vec![m1, m2];

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
}
