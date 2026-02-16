//! Approximate sequence matching for license detection.
//!
//! This module implements sequence-based matching using set similarity for
//! candidate selection, followed by sequence alignment to find matching blocks.
//!
//! Based on Python ScanCode Toolkit implementation at:
//! reference/scancode-toolkit/src/licensedcode/match_seq.py

use crate::license_detection::index::LicenseIndex;
use crate::license_detection::index::token_sets::{
    build_set_and_mset, high_tids_set_subset, multiset_counter, tids_set_counter,
};
use crate::license_detection::models::{LicenseMatch, Rule};
use crate::license_detection::query::QueryRun;
use std::collections::{HashMap, HashSet};

pub const MATCH_SEQ: &str = "3-seq";
#[allow(dead_code)]
pub const MATCH_SEQ_ORDER: u8 = 3;

/// Score vector for ranking candidates using set similarity.
///
/// Contains metrics computed from set/multiset intersections.
///
/// Corresponds to Python: `ScoresVector` namedtuple in match_set.py (line 458)
#[derive(Debug, Clone, PartialEq)]
struct ScoresVector {
    /// True if the sets are highly similar (resemblance >= threshold)
    is_highly_resemblant: bool,
    /// Containment ratio (how much of rule is in query)
    containment: f32,
    /// Amplified resemblance (squared to boost high values)
    resemblance: f32,
    /// Number of matched tokens (normalized for ranking)
    matched_length: f32,
}

impl PartialOrd for ScoresVector {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for ScoresVector {}

impl Ord for ScoresVector {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.containment
            .partial_cmp(&other.containment)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                self.resemblance
                    .partial_cmp(&other.resemblance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                self.matched_length
                    .partial_cmp(&other.matched_length)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| self.is_highly_resemblant.cmp(&other.is_highly_resemblant))
    }
}

/// Candidate with its score vector and metadata.
///
/// Corresponds to the tuple structure used in Python: (scores_vectors, rid, rule, high_set_intersection)
#[derive(Debug, Clone, PartialEq)]
struct Candidate {
    /// Rounded score vector for display/grouping
    score_vec_rounded: ScoresVector,
    /// Full score vector for sorting
    score_vec_full: ScoresVector,
    /// Rule ID
    rid: usize,
    /// Reference to the rule
    rule: Rule,
    /// Set of high-value (legalese) tokens in the intersection
    high_set_intersection: HashSet<u16>,
}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Candidate {}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score_vec_full.cmp(&other.score_vec_full)
    }
}

/// Compute set similarity between query and rule token sets.
///
/// Returns a score vector with containment, resemblance, and matched length metrics.
///
/// Corresponds to Python: `compare_token_sets()` in match_set.py (line 370)
///
/// # Arguments
///
/// * `query_set` - Query token set (unique tokens)
/// * `query_mset` - Query token multiset (with frequencies)
/// * `rule_set` - Rule token set (unique tokens)
/// * `rule_mset` - Rule token multiset (with frequencies)
/// * `len_legalese` - Number of legalese tokens (IDs < this are high-value)
///
/// # Returns
///
/// Option containing (rounded ScoresVector, full ScoresVector) or None if intersection is too small
fn compute_set_similarity(
    query_set: &HashSet<u16>,
    query_mset: &HashMap<u16, usize>,
    rule_set: &HashSet<u16>,
    rule_mset: &HashMap<u16, usize>,
    len_legalese: usize,
) -> Option<(ScoresVector, ScoresVector)> {
    let intersection: HashSet<u16> = query_set.intersection(rule_set).copied().collect();

    if intersection.is_empty() {
        return None;
    }

    let high_intersection = high_tids_set_subset(&intersection, len_legalese);

    if high_intersection.is_empty() {
        return None;
    }

    let _high_matched_length = tids_set_counter(&high_intersection);

    let matched_length = multiset_counter(query_mset);
    let rule_length = multiset_counter(rule_mset);
    let query_length = multiset_counter(query_mset);

    if matched_length == 0 || rule_length == 0 {
        return None;
    }

    let union_length = query_length + rule_length - matched_length;
    let resemblance = matched_length as f32 / union_length as f32;
    let containment = matched_length as f32 / rule_length as f32;
    let amplified_resemblance = resemblance.powi(2);

    let score_vec_rounded = ScoresVector {
        is_highly_resemblant: resemblance >= 0.8,
        containment: (containment * 10.0).round() / 10.0,
        resemblance: (amplified_resemblance * 10.0).round() / 10.0,
        matched_length: (matched_length as f32 / 20.0),
    };

    let score_vec_full = ScoresVector {
        is_highly_resemblant: resemblance >= 0.8,
        containment,
        resemblance: amplified_resemblance,
        matched_length: matched_length as f32,
    };

    Some((score_vec_rounded, score_vec_full))
}

/// Select top-N candidate rules for sequence matching.
///
/// Uses set similarity to rank candidates and returns the top-N.
///
/// Corresponds to Python: `compute_candidates()` in match_set.py (line 244)
///
/// # Arguments
///
/// * `index` - License index containing rule token sets
/// * `query_run` - Query run to match
/// * `top_n` - Number of top candidates to return
///
/// # Returns
///
/// Vector of top-N candidates sorted by similarity score
fn select_candidates(index: &LicenseIndex, query_run: &QueryRun, top_n: usize) -> Vec<Candidate> {
    let mut candidates = Vec::new();

    let query_tokens = query_run.matchable_tokens();
    if query_tokens.is_empty() {
        return candidates;
    }

    let query_token_ids: Vec<u16> = query_tokens
        .iter()
        .filter_map(|&tid| if tid >= 0 { Some(tid as u16) } else { None })
        .collect();

    if query_token_ids.is_empty() {
        return candidates;
    }

    let (query_set, query_mset) = build_set_and_mset(&query_token_ids);

    let len_legalese = index.len_legalese;

    for (rid, rule) in index.rules_by_rid.iter().enumerate() {
        let rule_set = index.sets_by_rid.get(&rid);
        let rule_mset = index.msets_by_rid.get(&rid);

        if let (Some(rule_set), Some(rule_mset)) = (rule_set, rule_mset)
            && let Some((score_vec_rounded, score_vec_full)) =
                compute_set_similarity(&query_set, &query_mset, rule_set, rule_mset, len_legalese)
        {
            let intersection: HashSet<u16> = query_set.intersection(rule_set).copied().collect();
            let high_set_intersection = high_tids_set_subset(&intersection, len_legalese);

            candidates.push(Candidate {
                score_vec_rounded,
                score_vec_full,
                rid,
                rule: rule.clone(),
                high_set_intersection,
            });
        }
    }

    candidates.sort_by(|a, b| b.cmp(a));
    candidates.truncate(top_n);
    candidates
}

/// Find the longest matching block between query and rule token sequences.
///
/// Uses dynamic programming to find the longest contiguous matching subsequence.
///
/// Corresponds to Python: `find_longest_match()` in seq.py (line 19)
///
/// # Arguments
///
/// * `query_tokens` - Query token sequence (called `a` in Python)
/// * `rule_tokens` - Rule token sequence (called `b` in Python)
/// * `query_lo` - Start position in query (inclusive)
/// * `query_hi` - End position in query (exclusive)
/// * `rule_lo` - Start position in rule (inclusive)
/// * `rule_hi` - End position in rule (exclusive)
/// * `high_postings` - Mapping of rule token IDs to their positions (b2j in Python)
/// * `len_legalese` - Token IDs below this are "good" tokens
/// * `matchables` - Set of matchable positions in query
///
/// # Returns
///
/// Tuple of (query_start, rule_start, match_length)
#[allow(clippy::too_many_arguments, clippy::needless_range_loop)]
#[allow(dead_code)]
fn find_longest_match(
    query_tokens: &[u16],
    rule_tokens: &[u16],
    query_lo: usize,
    query_hi: usize,
    rule_lo: usize,
    rule_hi: usize,
    high_postings: &HashMap<u16, Vec<usize>>,
    len_legalese: usize,
    matchables: &HashSet<usize>,
) -> (usize, usize, usize) {
    let mut best_i = query_lo;
    let mut best_j = rule_lo;
    let mut best_size = 0;

    let mut j2len: HashMap<usize, usize> = HashMap::new();

    for i in query_lo..query_hi {
        let mut new_j2len: HashMap<usize, usize> = HashMap::new();
        let cur_a = query_tokens[i];

        if (cur_a as usize) < len_legalese
            && matchables.contains(&i)
            && let Some(positions) = high_postings.get(&cur_a)
        {
            for &j in positions {
                if j < rule_lo {
                    continue;
                }
                if j >= rule_hi {
                    break;
                }

                let prev_len = if j > 0 {
                    j2len.get(&(j - 1)).copied().unwrap_or(0)
                } else {
                    0
                };
                let k = prev_len + 1;
                new_j2len.insert(j, k);

                if k > best_size {
                    best_i = i + 1 - k;
                    best_j = j + 1 - k;
                    best_size = k;
                }
            }
        }
        j2len = new_j2len;
    }

    if best_size > 0 {
        while best_i > query_lo
            && best_j > rule_lo
            && query_tokens[best_i - 1] == rule_tokens[best_j - 1]
            && matchables.contains(&(best_i - 1))
        {
            best_i -= 1;
            best_j -= 1;
            best_size += 1;
        }

        while best_i + best_size < query_hi
            && best_j + best_size < rule_hi
            && query_tokens[best_i + best_size] == rule_tokens[best_j + best_size]
            && matchables.contains(&(best_i + best_size))
        {
            best_size += 1;
        }
    }

    (best_i, best_j, best_size)
}

/// Find all matching blocks between query and rule token sequences using divide-and-conquer.
///
/// Uses a queue-based algorithm to find longest match, then recursively processes
/// left and right regions to find all matches.
///
/// Corresponds to Python: `match_blocks()` in seq.py (line 107)
///
/// # Arguments
///
/// * `query_tokens` - Query token sequence (called `a` in Python)
/// * `rule_tokens` - Rule token sequence (called `b` in Python)
/// * `query_start` - Start position in query (inclusive)
/// * `query_end` - End position in query (exclusive)
/// * `high_postings` - Mapping of rule token IDs to their positions (b2j in Python)
/// * `len_legalese` - Token IDs below this are "good" tokens
/// * `matchables` - Set of matchable positions in query
///
/// # Returns
///
/// Vector of matching blocks as (query_pos, rule_pos, length)
#[allow(dead_code)]
fn match_blocks(
    query_tokens: &[u16],
    rule_tokens: &[u16],
    query_start: usize,
    query_end: usize,
    high_postings: &HashMap<u16, Vec<usize>>,
    len_legalese: usize,
    matchables: &HashSet<usize>,
) -> Vec<(usize, usize, usize)> {
    if query_tokens.is_empty() || rule_tokens.is_empty() {
        return Vec::new();
    }

    let mut queue: Vec<(usize, usize, usize, usize)> =
        vec![(query_start, query_end, 0, rule_tokens.len())];
    let mut matching_blocks: Vec<(usize, usize, usize)> = Vec::new();

    while let Some((alo, ahi, blo, bhi)) = queue.pop() {
        let (i, j, k) = find_longest_match(
            query_tokens,
            rule_tokens,
            alo,
            ahi,
            blo,
            bhi,
            high_postings,
            len_legalese,
            matchables,
        );

        if k > 0 {
            matching_blocks.push((i, j, k));

            if alo < i && blo < j {
                queue.push((alo, i, blo, j));
            }
            if i + k < ahi && j + k < bhi {
                queue.push((i + k, ahi, j + k, bhi));
            }
        }
    }

    matching_blocks.sort();

    let mut non_adjacent: Vec<(usize, usize, usize)> = Vec::new();
    let mut i1 = 0usize;
    let mut j1 = 0usize;
    let mut k1 = 0usize;

    for (i2, j2, k2) in matching_blocks {
        if i1 + k1 == i2 && j1 + k1 == j2 {
            k1 += k2;
        } else {
            if k1 > 0 {
                non_adjacent.push((i1, j1, k1));
            }
            i1 = i2;
            j1 = j2;
            k1 = k2;
        }
    }

    if k1 > 0 {
        non_adjacent.push((i1, j1, k1));
    }

    non_adjacent
}

/// Main sequence matching function.
///
/// Performs approximate sequence matching using set similarity for candidate
/// selection followed by sequence alignment.
///
/// Corresponds to Python: `match_sequence()` in match_seq.py (line 48)
///
/// # Arguments
///
/// * `index` - License index
/// * `query_run` - Query run to match
///
/// # Returns
///
/// Vector of LicenseMatch results
pub fn seq_match(index: &LicenseIndex, query_run: &QueryRun) -> Vec<LicenseMatch> {
    let mut matches = Vec::new();

    let candidates = select_candidates(index, query_run, 50);

    for candidate in candidates {
        let rid = candidate.rid;
        let rule_tokens = index.tids_by_rid.get(rid);
        let high_postings = index.high_postings_by_rid.get(&rid);

        if let (Some(rule_tokens), Some(high_postings)) = (rule_tokens, high_postings) {
            let query_tokens = query_run.tokens();
            let len_legalese = index.len_legalese;

            let qbegin = query_run.start;
            let qfinish = query_run.end.unwrap_or(qbegin);

            let matchables = query_run.matchables(true);

            let mut qstart = qbegin;

            while qstart <= qfinish {
                let blocks = match_blocks(
                    query_tokens,
                    rule_tokens,
                    qstart,
                    qfinish + 1,
                    high_postings,
                    len_legalese,
                    &matchables,
                );

                if blocks.is_empty() {
                    break;
                }

                let mut max_qend = qstart;

                for (qpos, _ipos, mlen) in blocks {
                    if mlen < 1 {
                        continue;
                    }

                    if mlen == 1 && query_tokens[qpos] >= len_legalese as u16 {
                        continue;
                    }

                    let rule_length = rule_tokens.len();
                    if rule_length == 0 {
                        continue;
                    }

                    let match_coverage = (mlen as f32 / rule_length as f32) * 100.0;

                    let qend = qpos + mlen - 1;
                    let start_line = query_run.line_for_pos(qpos).unwrap_or(1);
                    let end_line = query_run.line_for_pos(qend).unwrap_or(start_line);

                    let score = (match_coverage * candidate.rule.relevance as f32) / 100.0;

                    let matched_text = query_run.matched_text(start_line, end_line);

                    let license_match = LicenseMatch {
                        license_expression: candidate.rule.license_expression.clone(),
                        license_expression_spdx: candidate.rule.license_expression.clone(),
                        from_file: None,
                        start_line,
                        end_line,
                        matcher: MATCH_SEQ.to_string(),
                        score,
                        matched_length: mlen,
                        match_coverage,
                        rule_relevance: candidate.rule.relevance,
                        rule_identifier: format!("#{}", rid),
                        rule_url: String::new(),
                        matched_text: Some(matched_text),
                        referenced_filenames: candidate.rule.referenced_filenames.clone(),
                        is_license_intro: candidate.rule.is_license_intro,
                        is_license_clue: candidate.rule.is_license_clue,
                    };

                    matches.push(license_match);

                    max_qend = max_qend.max(qend + 1);
                }

                qstart = max_qend;
            }
        }
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::license_detection::query::Query;
    use crate::license_detection::test_utils::create_test_index;

    fn create_seq_match_test_index() -> LicenseIndex {
        create_test_index(
            &[
                ("license", 0),
                ("copyright", 1),
                ("permission", 2),
                ("redistribute", 3),
                ("granted", 4),
            ],
            5,
        )
    }

    fn add_test_rule(index: &mut LicenseIndex, text: &str, expression: &str) -> usize {
        let rid = index.rules_by_rid.len();
        let tokens: Vec<u16> = text
            .split_whitespace()
            .filter_map(|word| index.dictionary.get(word))
            .collect();

        let (set, mset) = build_set_and_mset(&tokens);
        let _ = index.sets_by_rid.insert(rid, set);
        let _ = index.msets_by_rid.insert(rid, mset);

        let mut high_postings: HashMap<u16, Vec<usize>> = HashMap::new();
        for (pos, &tid) in tokens.iter().enumerate() {
            if (tid as usize) < index.len_legalese {
                high_postings.entry(tid).or_default().push(pos);
            }
        }
        let _ = index.high_postings_by_rid.insert(rid, high_postings);

        let rule = Rule {
            identifier: format!("{}.test", expression),
            license_expression: expression.to_string(),
            text: text.to_string(),
            tokens: tokens.clone(),
            is_license_text: true,
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
            min_matched_length: 0,
            min_high_matched_length: 0,
            min_matched_length_unique: 0,
            min_high_matched_length_unique: 0,
            is_small: false,
            is_tiny: false,
            is_deprecated: false,
            spdx_license_key: None,
            other_spdx_license_keys: vec![],
        };

        index.rules_by_rid.push(rule.clone());
        index.tids_by_rid.push(tokens);

        rid
    }

    #[test]
    fn test_scores_vector_comparison() {
        let sv1 = ScoresVector {
            is_highly_resemblant: true,
            containment: 0.9,
            resemblance: 0.8,
            matched_length: 10.0,
        };

        let sv2 = ScoresVector {
            is_highly_resemblant: false,
            containment: 0.8,
            resemblance: 0.6,
            matched_length: 5.0,
        };

        assert!(sv1 > sv2);
    }

    #[test]
    fn test_compute_set_similarity_identical() {
        let mut set1 = HashSet::new();
        set1.insert(0);
        set1.insert(1);
        set1.insert(2);

        let mut mset1 = HashMap::new();
        mset1.insert(0, 1);
        mset1.insert(1, 1);
        mset1.insert(2, 1);

        let set2 = set1.clone();
        let mset2 = mset1.clone();

        let result = compute_set_similarity(&set1, &mset1, &set2, &mset2, 5);

        assert!(result.is_some());
        let (rounded, full) = result.unwrap();
        assert!(rounded.is_highly_resemblant);
        assert!((full.containment - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_compute_set_similarity_no_intersection() {
        let mut set1 = HashSet::new();
        set1.insert(0);
        set1.insert(1);

        let mut mset1 = HashMap::new();
        mset1.insert(0, 1);
        mset1.insert(1, 1);

        let mut set2 = HashSet::new();
        set2.insert(10);
        set2.insert(11);

        let mut mset2 = HashMap::new();
        mset2.insert(10, 1);
        mset2.insert(11, 1);

        let result = compute_set_similarity(&set1, &mset1, &set2, &mset2, 5);

        assert!(result.is_none());
    }

    #[test]
    fn test_compute_set_similarity_no_legalese() {
        let mut set1 = HashSet::new();
        set1.insert(10);
        set1.insert(11);

        let mut mset1 = HashMap::new();
        mset1.insert(10, 1);
        mset1.insert(11, 1);

        let mut set2 = HashSet::new();
        set2.insert(10);
        set2.insert(11);

        let mut mset2 = HashMap::new();
        mset2.insert(10, 1);
        mset2.insert(11, 1);

        let result = compute_set_similarity(&set1, &mset1, &set2, &mset2, 5);

        assert!(result.is_none());
    }

    #[test]
    fn test_select_candidates() {
        let mut index = create_seq_match_test_index();

        add_test_rule(&mut index, "license copyright granted", "test-license-1");
        add_test_rule(&mut index, "permission redistribute", "test-license-2");

        let text = "license copyright granted here";
        let query = Query::new(text, &index).unwrap();
        let query_run = query.whole_query_run();

        let candidates = select_candidates(&index, &query_run, 10);

        assert!(!candidates.is_empty());
    }

    #[test]
    fn test_find_longest_match_basic() {
        let query_tokens = vec![0, 1, 2, 3];
        let rule_tokens = vec![0, 1, 2, 3];
        let mut high_postings: HashMap<u16, Vec<usize>> = HashMap::new();
        high_postings.insert(0, vec![0]);
        high_postings.insert(1, vec![1]);
        high_postings.insert(2, vec![2]);
        high_postings.insert(3, vec![3]);

        let matchables: HashSet<usize> = (0..query_tokens.len()).collect();

        let result = find_longest_match(
            &query_tokens,
            &rule_tokens,
            0,
            query_tokens.len(),
            0,
            rule_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert_eq!(result, (0, 0, 4), "Should find full match");
    }

    #[test]
    fn test_find_longest_match_with_gap() {
        let query_tokens = vec![0, 1, 99, 2, 3];
        let rule_tokens = vec![0, 1, 2, 3];
        let mut high_postings: HashMap<u16, Vec<usize>> = HashMap::new();
        high_postings.insert(0, vec![0]);
        high_postings.insert(1, vec![1]);
        high_postings.insert(2, vec![2]);
        high_postings.insert(3, vec![3]);

        let matchables: HashSet<usize> = (0..query_tokens.len()).collect();

        let result = find_longest_match(
            &query_tokens,
            &rule_tokens,
            0,
            query_tokens.len(),
            0,
            rule_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert_eq!(
            result.2, 2,
            "Should find longest contiguous match (length 2)"
        );
        assert!(
            result == (0, 0, 2) || result == (3, 2, 2),
            "Should find either [0,1] or [2,3] match, got {:?}",
            result
        );
    }

    #[test]
    fn test_find_longest_match_uses_high_postings() {
        let query_tokens = vec![0, 10, 2];
        let rule_tokens = vec![0, 1, 2];
        let mut high_postings: HashMap<u16, Vec<usize>> = HashMap::new();
        high_postings.insert(0, vec![0]);
        high_postings.insert(2, vec![2]);

        let matchables: HashSet<usize> = (0..query_tokens.len()).collect();

        let result = find_longest_match(
            &query_tokens,
            &rule_tokens,
            0,
            query_tokens.len(),
            0,
            rule_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert_eq!(
            result.2, 1,
            "Token 10 is not in high_postings and doesn't match token 1, so LCS finds separate matches"
        );
    }

    #[test]
    fn test_find_longest_match_no_match() {
        let query_tokens = vec![10, 11, 12];
        let rule_tokens = vec![0, 1, 2];
        let high_postings: HashMap<u16, Vec<usize>> = HashMap::new();

        let matchables: HashSet<usize> = (0..query_tokens.len()).collect();

        let result = find_longest_match(
            &query_tokens,
            &rule_tokens,
            0,
            query_tokens.len(),
            0,
            rule_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert_eq!(
            result,
            (0, 0, 0),
            "Should return (alo, blo, 0) for no match"
        );
    }

    #[test]
    fn test_find_longest_match_respects_bounds() {
        let query_tokens = vec![0, 1, 2, 0, 1, 2, 0, 1, 2];
        let rule_tokens = vec![0, 1, 2];
        let mut high_postings: HashMap<u16, Vec<usize>> = HashMap::new();
        high_postings.insert(0, vec![0]);
        high_postings.insert(1, vec![1]);
        high_postings.insert(2, vec![2]);

        let matchables: HashSet<usize> = (0..query_tokens.len()).collect();

        let result = find_longest_match(
            &query_tokens,
            &rule_tokens,
            3,
            6,
            0,
            rule_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert_eq!(
            result,
            (3, 0, 3),
            "Should find match within query bounds [3,6)"
        );
    }

    #[test]
    fn test_find_longest_match_non_matchable_position() {
        let query_tokens = vec![0, 1, 2];
        let rule_tokens = vec![0, 1, 2];
        let mut high_postings: HashMap<u16, Vec<usize>> = HashMap::new();
        high_postings.insert(0, vec![0]);
        high_postings.insert(1, vec![1]);
        high_postings.insert(2, vec![2]);

        let matchables: HashSet<usize> = [0, 2].into_iter().collect();

        let result = find_longest_match(
            &query_tokens,
            &rule_tokens,
            0,
            query_tokens.len(),
            0,
            rule_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert_eq!(
            result.2, 1,
            "Position 1 is not matchable, so longest match should be 1"
        );
    }

    #[test]
    fn test_match_blocks_divide_conquer() {
        let query_tokens = vec![0, 1, 2, 3];
        let rule_tokens = vec![0, 1, 2, 3];
        let mut high_postings: HashMap<u16, Vec<usize>> = HashMap::new();
        high_postings.insert(0, vec![0]);
        high_postings.insert(1, vec![1]);
        high_postings.insert(2, vec![2]);
        high_postings.insert(3, vec![3]);

        let matchables: HashSet<usize> = (0..query_tokens.len()).collect();

        let blocks = match_blocks(
            &query_tokens,
            &rule_tokens,
            0,
            query_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert_eq!(blocks.len(), 1, "Should find single full match");
        assert_eq!(blocks[0], (0, 0, 4), "Should match entire sequence");
    }

    #[test]
    fn test_match_blocks_collapse_adjacent() {
        let query_tokens = vec![0, 1, 2, 3, 4];
        let rule_tokens = vec![0, 1, 2, 3, 4];
        let mut high_postings: HashMap<u16, Vec<usize>> = HashMap::new();
        for (i, &tid) in query_tokens.iter().enumerate() {
            high_postings.entry(tid).or_default().push(i);
        }

        let matchables: HashSet<usize> = (0..query_tokens.len()).collect();

        let blocks = match_blocks(
            &query_tokens,
            &rule_tokens,
            0,
            query_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert_eq!(
            blocks.len(),
            1,
            "Adjacent blocks should be collapsed into one"
        );
        assert_eq!(blocks[0].2, 5, "Collapsed block should have full length");
    }

    #[test]
    fn test_match_blocks_no_match() {
        let query_tokens = vec![10, 11, 12];
        let rule_tokens = vec![0, 1, 2];
        let high_postings: HashMap<u16, Vec<usize>> = HashMap::new();

        let matchables: HashSet<usize> = (0..query_tokens.len()).collect();

        let blocks = match_blocks(
            &query_tokens,
            &rule_tokens,
            0,
            query_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert!(blocks.is_empty(), "Should return empty when no matches");
    }

    #[test]
    fn test_match_blocks_empty_query() {
        let query_tokens: Vec<u16> = vec![];
        let rule_tokens = vec![0, 1, 2];
        let high_postings: HashMap<u16, Vec<usize>> = HashMap::new();
        let matchables: HashSet<usize> = HashSet::new();

        let blocks = match_blocks(
            &query_tokens,
            &rule_tokens,
            0,
            query_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert!(blocks.is_empty());
    }

    #[test]
    fn test_match_blocks_with_gap() {
        let query_tokens = vec![0, 1, 99, 2, 3];
        let rule_tokens = vec![0, 1, 2, 3];
        let mut high_postings: HashMap<u16, Vec<usize>> = HashMap::new();
        high_postings.insert(0, vec![0]);
        high_postings.insert(1, vec![1]);
        high_postings.insert(2, vec![2]);
        high_postings.insert(3, vec![3]);

        let matchables: HashSet<usize> = (0..query_tokens.len()).collect();

        let blocks = match_blocks(
            &query_tokens,
            &rule_tokens,
            0,
            query_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert!(!blocks.is_empty(), "Should find matches despite gap");
        assert!(
            blocks.iter().any(|b| b.2 >= 2),
            "Should find at least one block of length >= 2"
        );
    }

    #[test]
    fn test_match_blocks_empty_rule() {
        let query_tokens = vec![0, 1, 2];
        let rule_tokens: Vec<u16> = vec![];
        let high_postings: HashMap<u16, Vec<usize>> = HashMap::new();
        let matchables: HashSet<usize> = (0..query_tokens.len()).collect();

        let blocks = match_blocks(
            &query_tokens,
            &rule_tokens,
            0,
            query_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert!(blocks.is_empty());
    }

    #[test]
    fn test_match_blocks_multiple_regions() {
        let query_tokens = vec![0, 1, 99, 2, 3, 88, 0, 1];
        let rule_tokens = vec![0, 1, 2, 3];
        let mut high_postings: HashMap<u16, Vec<usize>> = HashMap::new();
        high_postings.insert(0, vec![0]);
        high_postings.insert(1, vec![1]);
        high_postings.insert(2, vec![2]);
        high_postings.insert(3, vec![3]);

        let matchables: HashSet<usize> = (0..query_tokens.len()).collect();

        let blocks = match_blocks(
            &query_tokens,
            &rule_tokens,
            0,
            query_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert!(
            blocks.len() >= 2,
            "Should find multiple match regions, got {:?}",
            blocks
        );
    }

    #[test]
    fn test_match_blocks_with_range() {
        let query_tokens = vec![0, 1, 2, 99, 0, 1, 2];
        let rule_tokens = vec![0, 1, 2];
        let mut high_postings: HashMap<u16, Vec<usize>> = HashMap::new();
        high_postings.insert(0, vec![0]);
        high_postings.insert(1, vec![1]);
        high_postings.insert(2, vec![2]);

        let matchables: HashSet<usize> = (0..query_tokens.len()).collect();

        let blocks = match_blocks(
            &query_tokens,
            &rule_tokens,
            0,
            3,
            &high_postings,
            5,
            &matchables,
        );

        assert_eq!(
            blocks.len(),
            1,
            "Should only find one match in the restricted range"
        );
        assert_eq!(blocks[0], (0, 0, 3));

        let blocks2 = match_blocks(
            &query_tokens,
            &rule_tokens,
            4,
            query_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );
        assert_eq!(blocks2.len(), 1, "Should find the second occurrence");
        assert_eq!(blocks2[0], (4, 0, 3));
    }

    #[test]
    fn test_seq_match_basic() {
        let mut index = create_seq_match_test_index();

        add_test_rule(&mut index, "license copyright granted", "test-license");

        let text = "license copyright granted here";
        let query = Query::new(text, &index).unwrap();
        let query_run = query.whole_query_run();

        let matches = seq_match(&index, &query_run);

        assert!(!matches.is_empty());
        assert_eq!(matches[0].matcher, MATCH_SEQ);
    }

    #[test]
    fn test_seq_match_partial_coverage_not_filtered() {
        let mut index = create_seq_match_test_index();

        add_test_rule(
            &mut index,
            "license copyright granted permission redistribute",
            "test-long-license",
        );

        let text = "license copyright";
        let query = Query::new(text, &index).unwrap();
        let query_run = query.whole_query_run();

        let matches = seq_match(&index, &query_run);

        assert!(
            !matches.is_empty(),
            "Partial coverage matches should NOT be filtered (Python has no 50% coverage filter)"
        );
        assert!(matches[0].match_coverage < 50.0);
    }

    #[test]
    fn test_seq_match_empty_query() {
        let mut index = create_seq_match_test_index();

        add_test_rule(&mut index, "license copyright", "test-license");

        let text = "";
        let query = Query::new(text, &index).unwrap();
        let query_run = query.whole_query_run();

        let matches = seq_match(&index, &query_run);

        assert!(matches.is_empty());
    }

    #[test]
    fn test_seq_match_constants() {
        assert_eq!(MATCH_SEQ, "3-seq");
        assert_eq!(MATCH_SEQ_ORDER, 3);
    }

    #[test]
    fn test_compute_set_similarity_partial() {
        let mut set1 = HashSet::new();
        set1.insert(0);
        set1.insert(1);
        set1.insert(2);

        let mut mset1 = HashMap::new();
        mset1.insert(0, 1);
        mset1.insert(1, 1);
        mset1.insert(2, 1);

        let mut set2 = HashSet::new();
        set2.insert(0);
        set2.insert(1);
        set2.insert(10);

        let mut mset2 = HashMap::new();
        mset2.insert(0, 1);
        mset2.insert(1, 1);
        mset2.insert(10, 1);

        let result = compute_set_similarity(&set1, &mset1, &set2, &mset2, 5);

        assert!(result.is_some());
        let (_rounded, full) = result.unwrap();
        assert!(full.containment > 0.0 && full.containment <= 1.0);
    }

    #[test]
    fn test_select_candidates_empty_tokens() {
        let index = create_seq_match_test_index();

        let text = "";
        let query = Query::new(text, &index).unwrap();
        let query_run = query.whole_query_run();

        let candidates = select_candidates(&index, &query_run, 10);

        assert!(
            candidates.is_empty(),
            "Should return empty candidates for empty query"
        );
    }

    #[test]
    fn test_seq_match_with_no_legalese_intersection() {
        let mut index = create_test_index(&[("word1", 10), ("word2", 11), ("word3", 12)], 5);

        add_test_rule(&mut index, "word1 word2 word3", "test-license");

        let text = "word1 word2 word3";
        let query = Query::new(text, &index).unwrap();
        let query_run = query.whole_query_run();

        let matches = seq_match(&index, &query_run);

        assert!(
            matches.is_empty(),
            "Should not match when tokens are not legalese (above len_legalese)"
        );
    }

    #[test]
    fn test_candidate_ordering() {
        let candidate1 = Candidate {
            score_vec_rounded: ScoresVector {
                is_highly_resemblant: true,
                containment: 0.9,
                resemblance: 0.8,
                matched_length: 10.0,
            },
            score_vec_full: ScoresVector {
                is_highly_resemblant: true,
                containment: 0.9,
                resemblance: 0.8,
                matched_length: 10.0,
            },
            rid: 0,
            rule: Rule {
                identifier: "test1".to_string(),
                license_expression: "mit".to_string(),
                text: String::new(),
                tokens: vec![],
                is_license_text: true,
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
                min_matched_length: 0,
                min_high_matched_length: 0,
                min_matched_length_unique: 0,
                min_high_matched_length_unique: 0,
                is_small: false,
                is_tiny: false,
                is_deprecated: false,
                spdx_license_key: None,
                other_spdx_license_keys: vec![],
            },
            high_set_intersection: HashSet::new(),
        };

        let candidate2 = Candidate {
            score_vec_rounded: ScoresVector {
                is_highly_resemblant: false,
                containment: 0.5,
                resemblance: 0.3,
                matched_length: 5.0,
            },
            score_vec_full: ScoresVector {
                is_highly_resemblant: false,
                containment: 0.5,
                resemblance: 0.3,
                matched_length: 5.0,
            },
            rid: 1,
            rule: Rule {
                identifier: "test2".to_string(),
                license_expression: "apache".to_string(),
                text: String::new(),
                tokens: vec![],
                is_license_text: true,
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
                min_matched_length: 0,
                min_high_matched_length: 0,
                min_matched_length_unique: 0,
                min_high_matched_length_unique: 0,
                is_small: false,
                is_tiny: false,
                is_deprecated: false,
                spdx_license_key: None,
                other_spdx_license_keys: vec![],
            },
            high_set_intersection: HashSet::new(),
        };

        assert!(
            candidate1 > candidate2,
            "Higher containment candidate should rank higher"
        );
    }

    #[test]
    fn test_seq_match_multiple_occurrences() {
        let mut index = create_seq_match_test_index();

        add_test_rule(&mut index, "license copyright granted", "test-license");

        let text = "license copyright granted some text license copyright granted more text";
        let query = Query::new(text, &index).unwrap();
        let query_run = query.whole_query_run();

        let matches = seq_match(&index, &query_run);

        assert!(
            matches.len() >= 2,
            "Should find multiple matches for the same rule appearing multiple times in query, got {} matches",
            matches.len()
        );

        let license_expressions: Vec<&str> = matches
            .iter()
            .map(|m| m.license_expression.as_str())
            .collect();
        assert!(
            license_expressions.iter().all(|&e| e == "test-license"),
            "All matches should be for test-license"
        );

        let start_lines: Vec<usize> = matches.iter().map(|m| m.start_line).collect();
        let end_lines: Vec<usize> = matches.iter().map(|m| m.end_line).collect();

        assert!(
            start_lines.iter().all(|&l| l >= 1),
            "Start lines should be valid"
        );
        assert!(
            end_lines.iter().all(|&l| l >= 1),
            "End lines should be valid"
        );
    }

    #[test]
    fn test_seq_match_line_numbers_accurate() {
        let mut index = create_seq_match_test_index();

        add_test_rule(&mut index, "license copyright granted", "test-license");

        let text = "line one\nlicense copyright granted\nline three";
        let query = Query::new(text, &index).unwrap();
        let query_run = query.whole_query_run();

        let matches = seq_match(&index, &query_run);

        assert!(!matches.is_empty(), "Should find matches");

        let first_match = &matches[0];

        assert_eq!(
            first_match.start_line, 2,
            "Match should start on line 2 (where license tokens are), not line 1"
        );
        assert_eq!(
            first_match.end_line, 2,
            "Match should end on line 2 (where license tokens are), not line 3"
        );

        assert!(
            first_match
                .matched_text
                .as_ref()
                .is_some_and(|t| t.contains("license")),
            "Matched text should contain 'license'"
        );
    }

    #[test]
    fn test_seq_match_line_numbers_partial_match() {
        let mut index = create_seq_match_test_index();

        add_test_rule(
            &mut index,
            "license copyright granted permission",
            "test-license",
        );

        let text = "line one\nlicense copyright\nline three";
        let query = Query::new(text, &index).unwrap();
        let query_run = query.whole_query_run();

        let matches = seq_match(&index, &query_run);

        assert!(!matches.is_empty(), "Should find partial matches");

        let first_match = &matches[0];

        assert_eq!(
            first_match.start_line, 2,
            "Partial match should start on line 2"
        );
        assert_eq!(
            first_match.end_line, 2,
            "Partial match should end on line 2"
        );

        assert!(
            first_match.match_coverage < 100.0,
            "Should be partial coverage"
        );
    }

    #[test]
    fn test_extend_match_into_low_tokens() {
        let query_tokens = vec![0, 1, 2, 10, 11];
        let rule_tokens = vec![0, 1, 2, 10, 11];
        let mut high_postings: HashMap<u16, Vec<usize>> = HashMap::new();
        high_postings.insert(0, vec![0]);
        high_postings.insert(1, vec![1]);
        high_postings.insert(2, vec![2]);

        let matchables: HashSet<usize> = (0..query_tokens.len()).collect();

        let blocks = match_blocks(
            &query_tokens,
            &rule_tokens,
            0,
            query_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert_eq!(blocks.len(), 1, "Should find single extended match");
        assert_eq!(
            blocks[0].2, 5,
            "Match should extend into low-token areas (positions 3,4) when they are in matchables"
        );
    }

    #[test]
    fn test_extend_match_blocked_by_non_matchable() {
        let query_tokens = vec![0, 1, 2, 10, 11];
        let rule_tokens = vec![0, 1, 2, 10, 11];
        let mut high_postings: HashMap<u16, Vec<usize>> = HashMap::new();
        high_postings.insert(0, vec![0]);
        high_postings.insert(1, vec![1]);
        high_postings.insert(2, vec![2]);

        let matchables: HashSet<usize> = [0, 1, 2].into_iter().collect();

        let blocks = match_blocks(
            &query_tokens,
            &rule_tokens,
            0,
            query_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert_eq!(blocks.len(), 1, "Should find one match block");
        assert_eq!(
            blocks[0].2, 3,
            "Match should stop at position 3 because positions 3,4 are not in matchables"
        );
    }
}
