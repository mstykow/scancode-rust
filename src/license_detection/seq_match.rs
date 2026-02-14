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

/// Find matching blocks between query and rule token sequences.
///
/// Returns list of blocks as (query_start, query_end, rule_start, rule_end).
///
/// For Phase 4.4, this is a simplified version that finds consecutive exact token matches.
///
/// Corresponds to Python: `match_blocks()` in seq.py (used by match_seq.py line 100)
///
/// # Arguments
///
/// * `query_tokens` - Query token sequence
/// * `rule_tokens` - Rule token sequence
/// * `len_legalese` - Number of legalese tokens
/// * `high_postings` - Positions of high-value tokens in the rule (used for optimization)
///
/// # Returns
///
/// Vector of matching blocks as (query_start, query_end, rule_start, rule_end)
fn align_sequences(
    query_tokens: &[u16],
    rule_tokens: &[u16],
    len_legalese: usize,
    _high_postings: &HashMap<u16, Vec<usize>>,
) -> Vec<(usize, usize, usize, usize)> {
    let mut blocks = Vec::new();

    if query_tokens.is_empty() || rule_tokens.is_empty() {
        return blocks;
    }

    let mut i = 0;
    while i < query_tokens.len() {
        let qtoken = query_tokens[i];

        let j_opt = rule_tokens.iter().position(|&rtoken| rtoken == qtoken);

        if let Some(j) = j_opt {
            let mut block_len = 1;

            while i + block_len < query_tokens.len()
                && j + block_len < rule_tokens.len()
                && query_tokens[i + block_len] == rule_tokens[j + block_len]
            {
                block_len += 1;
            }

            let is_multi_token = block_len > 1;
            let is_high_token = qtoken < len_legalese as u16;

            if is_multi_token || is_high_token {
                blocks.push((i, i + block_len - 1, j, j + block_len - 1));
            }

            i += block_len;
        } else {
            i += 1;
        }
    }

    blocks
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

            let blocks = align_sequences(query_tokens, rule_tokens, len_legalese, high_postings);

            for (q_start, q_end, _r_start, _r_end) in blocks {
                let matched_length = q_end - q_start + 1;

                let rule_length = rule_tokens.len();
                if rule_length == 0 {
                    continue;
                }

                let match_coverage = (matched_length as f32 / rule_length as f32) * 100.0;

                if match_coverage < 50.0 {
                    continue;
                }

                let start_line = query_run.start_line().unwrap_or(1);
                let end_line = query_run.end_line().unwrap_or(start_line);

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
                    matched_length,
                    match_coverage,
                    rule_relevance: candidate.rule.relevance,
                    rule_identifier: format!("#{}", rid),
                    rule_url: String::new(),
                    matched_text: Some(matched_text),
                    referenced_filenames: candidate.rule.referenced_filenames.clone(),
                };

                matches.push(license_match);
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
    fn test_align_sequences_exact_match() {
        let query_tokens = vec![0, 1, 2, 3];
        let rule_tokens = vec![0, 1, 2, 3];
        let mut high_postings: HashMap<u16, Vec<usize>> = HashMap::new();
        high_postings.insert(0, vec![0]);
        high_postings.insert(1, vec![1]);
        high_postings.insert(2, vec![2]);

        let blocks = align_sequences(&query_tokens, &rule_tokens, 5, &high_postings);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0], (0, 3, 0, 3));
    }

    #[test]
    fn test_align_sequences_partial_match() {
        let query_tokens = vec![0, 1, 2, 5, 6, 0, 1, 2];
        let rule_tokens = vec![0, 1, 2, 3, 4];
        let mut high_postings: HashMap<u16, Vec<usize>> = HashMap::new();
        high_postings.insert(0, vec![0]);
        high_postings.insert(1, vec![1]);
        high_postings.insert(2, vec![2]);

        let blocks = align_sequences(&query_tokens, &rule_tokens, 5, &high_postings);

        assert_eq!(blocks.len(), 2);
        assert!(blocks.contains(&(0, 2, 0, 2)));
        assert!(blocks.contains(&(5, 7, 0, 2)));
    }

    #[test]
    fn test_align_sequences_no_match() {
        let query_tokens = vec![10, 11, 12];
        let rule_tokens = vec![0, 1, 2];
        let high_postings: HashMap<u16, Vec<usize>> = HashMap::new();

        let blocks = align_sequences(&query_tokens, &rule_tokens, 5, &high_postings);

        assert!(blocks.is_empty());
    }

    #[test]
    fn test_align_sequences_empty_query() {
        let query_tokens: Vec<u16> = vec![];
        let rule_tokens = vec![0, 1, 2];
        let high_postings: HashMap<u16, Vec<usize>> = HashMap::new();

        let blocks = align_sequences(&query_tokens, &rule_tokens, 5, &high_postings);

        assert!(blocks.is_empty());
    }

    #[test]
    fn test_align_sequences_low_value_token() {
        let query_tokens = vec![10];
        let rule_tokens = vec![10];
        let high_postings: HashMap<u16, Vec<usize>> = HashMap::new();

        let blocks = align_sequences(&query_tokens, &rule_tokens, 5, &high_postings);

        assert!(blocks.is_empty());
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
    fn test_seq_match_low_coverage_filtered() {
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

        assert!(matches.is_empty());
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
}
