//! Sequence matching algorithms for finding matching blocks.

use crate::license_detection::index::LicenseIndex;
use crate::license_detection::models::LicenseMatch;
use crate::license_detection::query::QueryRun;
use std::collections::{HashMap, HashSet};

use super::{Candidate, MATCH_SEQ};

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
pub(super) fn find_longest_match(
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
pub(super) fn match_blocks(
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

/// Sequence matching against pre-selected candidates.
///
/// Used by Phase 2 (near-duplicate detection) to match the whole file
/// against a small set of high-resemblance candidates.
///
/// # Arguments
///
/// * `index` - License index
/// * `query_run` - Query run to match (typically the whole file)
/// * `candidates` - Pre-selected candidates from `compute_candidates()`
///
/// # Returns
///
/// Vector of LicenseMatch results
pub fn seq_match_with_candidates(
    index: &LicenseIndex,
    query_run: &QueryRun,
    candidates: &[Candidate],
) -> Vec<LicenseMatch> {
    let mut matches = Vec::new();

    for candidate in candidates {
        let rid = candidate.rid;
        let rule_tokens = index.tids_by_rid.get(rid);
        let high_postings: HashMap<u16, Vec<usize>> = index
            .high_postings_by_rid
            .get(&rid)
            .map(|hp| {
                hp.iter()
                    .filter(|(tid, _)| candidate.high_set_intersection.contains(tid))
                    .map(|(&tid, postings)| (tid, postings.clone()))
                    .collect()
            })
            .unwrap_or_default();

        if let Some(rule_tokens) = rule_tokens {
            let query_tokens = query_run.tokens();
            let len_legalese = index.len_legalese;

            let qbegin = 0usize;
            let qfinish = query_tokens.len().saturating_sub(1);

            let matchables: HashSet<usize> = query_run
                .matchables(true)
                .into_iter()
                .map(|pos| pos - query_run.start)
                .collect();

            let mut qstart = qbegin;

            while qstart <= qfinish {
                let has_remaining_matchables = matchables.iter().any(|&pos| pos >= qstart);
                if !has_remaining_matchables {
                    break;
                }
                let blocks = match_blocks(
                    query_tokens,
                    rule_tokens,
                    qstart,
                    qfinish + 1,
                    &high_postings,
                    len_legalese,
                    &matchables,
                );

                if blocks.is_empty() {
                    break;
                }

                let mut max_qend = qstart;

                for (qpos, ipos, mlen) in blocks {
                    if mlen < 1 {
                        continue;
                    }

                    let qspan_end = qpos + mlen;
                    max_qend = max_qend.max(qspan_end);

                    if mlen == 1 && query_tokens[qpos] >= len_legalese as u16 {
                        continue;
                    }

                    let rule_length = rule_tokens.len();
                    if rule_length == 0 {
                        continue;
                    }

                    let qspan_positions: Vec<usize> = (qpos..qpos + mlen)
                        .map(|pos| pos + query_run.start)
                        .collect();
                    let ispan_positions: Vec<usize> = (ipos..ipos + mlen).collect();
                    let hispan_positions: Vec<usize> = (ipos..ipos + mlen)
                        .filter(|&p| rule_tokens.get(p).is_some_and(|t| *t < len_legalese as u16))
                        .collect();
                    let hispan_count = hispan_positions.len();

                    let qend = qpos + mlen - 1;
                    let abs_qpos = qpos + query_run.start;
                    let abs_qend = qend + query_run.start;
                    let start_line = query_run.line_for_pos(abs_qpos).unwrap_or(1);
                    let end_line = query_run.line_for_pos(abs_qend).unwrap_or(start_line);

                    let rule_coverage = mlen as f32 / rule_length as f32;
                    let match_coverage = rule_coverage * 100.0;

                    let score = match_coverage * candidate.rule.relevance as f32 / 100.0;

                    let matched_text = query_run.matched_text(start_line, end_line);

                    let license_match = LicenseMatch {
                        license_expression: candidate.rule.license_expression.clone(),
                        license_expression_spdx: candidate.rule.license_expression.clone(),
                        from_file: None,
                        start_line,
                        end_line,
                        start_token: abs_qpos,
                        end_token: abs_qend + 1,
                        matcher: MATCH_SEQ,
                        score,
                        matched_length: mlen,
                        rule_length,
                        match_coverage,
                        rule_relevance: candidate.rule.relevance,
                        rid,
                        rule_identifier: candidate.rule.identifier.clone(),
                        rule_url: String::new(),
                        matched_text: Some(matched_text),
                        referenced_filenames: candidate.rule.referenced_filenames.clone(),
                        is_license_intro: candidate.rule.is_license_intro,
                        is_license_clue: candidate.rule.is_license_clue,
                        is_license_reference: candidate.rule.is_license_reference,
                        is_license_tag: candidate.rule.is_license_tag,
                        is_license_text: candidate.rule.is_license_text,
                        is_from_license: candidate.rule.is_from_license,
                        matched_token_positions: None,
                        hilen: hispan_count,
                        rule_start_token: ipos,
                        qspan_positions: Some(qspan_positions),
                        ispan_positions: Some(ispan_positions),
                        hispan_positions: Some(hispan_positions),
                        candidate_resemblance: candidate.score_vec_full.resemblance,
                        candidate_containment: candidate.score_vec_full.containment,
                    };

                    matches.push(license_match);
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
