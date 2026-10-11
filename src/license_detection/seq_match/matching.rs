// SPDX-FileCopyrightText: nexB Inc. and others
// ScanCode is a trademark of nexB Inc.
// SPDX-FileCopyrightText: Provenant contributors
// SPDX-License-Identifier: Apache-2.0
// Derived from ScanCode Toolkit (Apache-2.0); modified. See NOTICE.

//! Sequence matching algorithms for finding matching blocks.
//!
//! The longest-match routine here is derived from Python's `difflib` (Python
//! Software Foundation License, PSF-2.0; Copyright (c) Python Software
//! Foundation and others) via ScanCode's `seq.py`, retained per Apache-2.0
//! section 4(c). The header above covers the ScanCode/nexB derivation.

use crate::cancel::CancelToken;
use crate::license_detection::LicenseDetectionError;
use crate::license_detection::index::LicenseIndex;
use crate::license_detection::index::dictionary::TokenId;
use crate::license_detection::models::position_span::PositionSpan;
use crate::license_detection::models::{LicenseMatch, MatchCoordinates};
use crate::license_detection::query::QueryRun;
use crate::models::LineNumber;
use crate::models::MatchScore;
use bit_set::BitSet;
use rustc_hash::{FxHashMap, FxHashSet};

use super::MATCH_SEQ;
use super::candidates::Candidate;

type HighPostings<'a> = FxHashMap<TokenId, &'a [usize]>;
type TokenOccurrences = FxHashMap<TokenId, Vec<usize>>;

/// Sorted matchable legalese positions per query token, so a search can visit
/// only the positions whose token occurs in the rule range being matched.
fn token_occurrences(
    query_tokens: &[TokenId],
    matchables: &BitSet,
    len_legalese: usize,
) -> TokenOccurrences {
    let mut occurrences = TokenOccurrences::default();
    for (i, &token) in query_tokens.iter().enumerate() {
        if token.as_usize() < len_legalese && matchables.contains(i) {
            occurrences.entry(token).or_default().push(i);
        }
    }
    occurrences
}

const MAX_SPARSE_RULE_TOKENS: usize = 8;

/// Ascending merge of the occurrence lists of a few tokens.
struct MergedPositions<'a> {
    lists: Vec<&'a [usize]>,
}

impl Iterator for MergedPositions<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        let list = self
            .lists
            .iter_mut()
            .filter(|list| !list.is_empty())
            .min_by_key(|list| list[0])?;
        let position = list[0];
        *list = &list[1..];
        Some(position)
    }
}

/// The query positions in `query_lo..query_hi` that can contribute to a match
/// against `rule_lo..rule_hi`, or `None` when a dense scan is cheaper. A dense
/// scan visits every position; any other position leaves the rolling run map
/// empty, which a gap between returned positions reproduces.
fn sparse_positions<'a>(
    context: &MatchSearchContext<'a>,
    query_lo: usize,
    query_hi: usize,
    rule_lo: usize,
    rule_hi: usize,
) -> Option<MergedPositions<'a>> {
    let mut rule_set: Vec<TokenId> = Vec::new();
    for &token in &context.rule_tokens[rule_lo..rule_hi] {
        if context.high_postings.contains_key(&token) && !rule_set.contains(&token) {
            if rule_set.len() == MAX_SPARSE_RULE_TOKENS {
                return None;
            }
            rule_set.push(token);
        }
    }

    let budget = (query_hi - query_lo) / 4;
    let mut total = 0;
    let mut lists = Vec::with_capacity(rule_set.len());
    for token in rule_set {
        let Some(all) = context.occurrences.get(&token) else {
            continue;
        };
        let start = all.partition_point(|&p| p < query_lo);
        let end = all.partition_point(|&p| p < query_hi);
        total += end - start;
        if total > budget {
            return None;
        }
        lists.push(&all[start..end]);
    }
    Some(MergedPositions { lists })
}

struct MatchSearchContext<'a> {
    query_tokens: &'a [TokenId],
    rule_tokens: &'a [TokenId],
    high_postings: &'a HighPostings<'a>,
    len_legalese: usize,
    matchables: &'a BitSet,
    run_potential: &'a [usize],
    occurrences: &'a TokenOccurrences,
    cancel: Option<&'a CancelToken>,
}

/// `potential[i]` bounds the longest diagonal run that can end at or after `i`:
/// each position must be matchable, legalese and in the rule's postings, and
/// each adjacent query pair must also be adjacent somewhere in the rule.
fn run_potential(
    query_tokens: &[TokenId],
    rule_tokens: &[TokenId],
    matchables: &BitSet,
    high_postings: &HighPostings<'_>,
    len_legalese: usize,
) -> Vec<usize> {
    let rule_pairs: FxHashSet<(TokenId, TokenId)> =
        rule_tokens.windows(2).map(|w| (w[0], w[1])).collect();
    let mut potential = vec![0usize; query_tokens.len() + 1];
    let mut run = 0usize;
    for (i, &token) in query_tokens.iter().enumerate() {
        let active = token.as_usize() < len_legalese
            && matchables.contains(i)
            && high_postings.contains_key(&token);
        run = if !active {
            0
        } else if i > 0 && run > 0 && rule_pairs.contains(&(query_tokens[i - 1], token)) {
            run + 1
        } else {
            1
        };
        potential[i] = run;
    }
    for i in (0..query_tokens.len()).rev() {
        potential[i] = potential[i].max(potential[i + 1]);
    }
    potential
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
// Index-based loop indexes parallel query/rule sequences in lockstep; an iterator rewrite would obscure the diagonal walk.
#[allow(clippy::needless_range_loop)]
fn find_longest_match_impl(
    context: &MatchSearchContext<'_>,
    query_lo: usize,
    query_hi: usize,
    rule_lo: usize,
    rule_hi: usize,
) -> Result<(usize, usize, usize), LicenseDetectionError> {
    let mut best_i = query_lo;
    let mut best_j = rule_lo;
    let mut best_size = 0;

    // `j2len` is a pure keyed lookup table (difflib's rolling longest-run map),
    // never iterated, so FxHash (vs SipHash on `usize` keys) is byte-identical
    // and much cheaper. The two maps are reused across iterations via clear+swap
    // to avoid a per-query-position allocation. This loop dominates license
    // detection on large-file repos (e.g. uv's big lockfiles); see #1061.
    let mut j2len: FxHashMap<usize, usize> = FxHashMap::default();
    let mut new_j2len: FxHashMap<usize, usize> = FxHashMap::default();

    let visited: Box<dyn Iterator<Item = usize> + '_> =
        match sparse_positions(context, query_lo, query_hi, rule_lo, rule_hi) {
            Some(positions) => Box::new(positions),
            None => Box::new(query_lo..query_hi),
        };
    let mut prev_i = None;

    for (offset, i) in visited.enumerate() {
        if offset.is_multiple_of(256) {
            crate::license_detection::ensure_not_cancelled(context.cancel)?;
        }

        if prev_i.is_none_or(|prev| prev + 1 != i) {
            j2len.clear();
        }
        prev_i = Some(i);

        if best_size >= (rule_hi - rule_lo).min(context.run_potential[i]) {
            break;
        }

        new_j2len.clear();
        let cur_a = context.query_tokens[i];

        if cur_a.as_usize() < context.len_legalese
            && context.matchables.contains(i)
            && let Some(positions) = context.high_postings.get(&cur_a)
        {
            for &j in *positions {
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
        // `new_j2len` holds this position's runs; it becomes `j2len` for the next
        // iteration, and the old `j2len` becomes the reused (cleared) buffer.
        std::mem::swap(&mut j2len, &mut new_j2len);
    }

    if best_size > 0 {
        while best_i > query_lo
            && best_j > rule_lo
            && context.query_tokens[best_i - 1] == context.rule_tokens[best_j - 1]
            && context.matchables.contains(best_i - 1)
        {
            best_i -= 1;
            best_j -= 1;
            best_size += 1;
        }

        while best_i + best_size < query_hi
            && best_j + best_size < rule_hi
            && context.query_tokens[best_i + best_size] == context.rule_tokens[best_j + best_size]
            && context.matchables.contains(best_i + best_size)
        {
            best_size += 1;
        }
    }

    Ok((best_i, best_j, best_size))
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
fn match_blocks_impl(
    context: &MatchSearchContext<'_>,
    query_start: usize,
    query_end: usize,
) -> Result<Vec<(usize, usize, usize)>, LicenseDetectionError> {
    if context.query_tokens.is_empty() || context.rule_tokens.is_empty() {
        return Ok(Vec::new());
    }

    let mut queue: Vec<(usize, usize, usize, usize)> =
        vec![(query_start, query_end, 0, context.rule_tokens.len())];
    let mut matching_blocks: Vec<(usize, usize, usize)> = Vec::new();

    let mut loop_count = 0usize;
    while let Some((alo, ahi, blo, bhi)) = queue.pop() {
        if loop_count.is_multiple_of(32) {
            crate::license_detection::ensure_not_cancelled(context.cancel)?;
        }
        loop_count += 1;

        let (i, j, k) = find_longest_match_impl(context, alo, ahi, blo, bhi)?;

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

    Ok(non_adjacent)
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
pub(crate) fn seq_match_with_candidates(
    index: &LicenseIndex,
    query_run: &QueryRun,
    candidates: &[Candidate<'_>],
) -> Vec<LicenseMatch> {
    seq_match_with_candidates_and_cancel(index, query_run, candidates, None)
        .expect("Sequence matching without cancel should not time out")
}

pub(crate) fn seq_match_with_candidates_and_cancel(
    index: &LicenseIndex,
    query_run: &QueryRun,
    candidates: &[Candidate<'_>],
    cancel: Option<&CancelToken>,
) -> Result<Vec<LicenseMatch>, LicenseDetectionError> {
    let mut matches = Vec::new();

    // Candidate-independent query-run setup, hoisted out of the per-candidate
    // loop: uv-class inputs produce many candidates against one huge query run,
    // and none of this depends on the candidate.
    let query_tokens = query_run.tokens();
    let len_legalese = index.len_legalese;
    let qfinish = query_tokens.len().saturating_sub(1);
    let matchables: BitSet = query_run
        .matchables(true)
        .iter()
        .map(|pos| pos - query_run.start)
        .collect();
    // `qstart <= max_matchable` is an O(1) equivalent of
    // `matchables.iter().any(|p| p >= qstart)`, avoiding an O(N) scan per round.
    let max_matchable = matchables.iter().max();
    let occurrences = token_occurrences(query_tokens, &matchables, len_legalese);

    for (candidate_index, candidate) in candidates.iter().enumerate() {
        if candidate_index.is_multiple_of(8) {
            crate::license_detection::ensure_not_cancelled(cancel)?;
        }

        let rid = candidate.rid;
        let rule_tokens = index.rule_tokens(rid);
        let high_postings: HighPostings<'_> = index
            .high_postings_by_rid
            .get(&rid)
            .map(|hp| {
                hp.iter()
                    .filter(|(tid, _)| candidate.high_set_intersection.contains(&tid.raw()))
                    .map(|(&tid, postings)| (tid, postings.as_slice()))
                    .collect()
            })
            .unwrap_or_default();

        if let Some(rule_tokens) = rule_tokens {
            let run_potential = run_potential(
                query_tokens,
                rule_tokens,
                &matchables,
                &high_postings,
                len_legalese,
            );
            let context = MatchSearchContext {
                query_tokens,
                rule_tokens,
                high_postings: &high_postings,
                len_legalese,
                matchables: &matchables,
                run_potential: &run_potential,
                occurrences: &occurrences,
                cancel,
            };

            let mut qstart = 0usize;
            let mut loop_count = 0usize;

            while qstart <= qfinish {
                if loop_count.is_multiple_of(32) {
                    crate::license_detection::ensure_not_cancelled(cancel)?;
                }
                loop_count += 1;

                let has_remaining_matchables = max_matchable.is_some_and(|m| m >= qstart);
                if !has_remaining_matchables {
                    break;
                }
                let blocks = match_blocks_impl(&context, qstart, qfinish + 1)?;

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

                    if mlen == 1 && query_tokens[qpos].as_usize() >= len_legalese {
                        continue;
                    }

                    let rule_length = rule_tokens.len();
                    if rule_length == 0 {
                        continue;
                    }

                    let qend = qpos + mlen - 1;
                    let abs_qpos = qpos + query_run.start;
                    let abs_qend = qend + query_run.start;
                    let start_line = query_run
                        .line_for_pos(abs_qpos)
                        .and_then(LineNumber::new)
                        .unwrap_or(LineNumber::ONE);
                    let end_line = query_run
                        .line_for_pos(abs_qend)
                        .and_then(LineNumber::new)
                        .unwrap_or(start_line);

                    let qspan =
                        PositionSpan::range(qpos + query_run.start, qpos + mlen + query_run.start);
                    let ispan = PositionSpan::range(ipos, ipos + mlen);
                    let hispan = PositionSpan::from_positions((ipos..ipos + mlen).filter(|&p| {
                        rule_tokens
                            .get(p)
                            .is_some_and(|t| t.as_usize() < len_legalese)
                    }));

                    let rule_coverage = mlen as f32 / rule_length as f32;
                    let match_coverage = LicenseMatch::round_metric(rule_coverage * 100.0);

                    let score = MatchScore::from_percentage(
                        f64::from(match_coverage) * f64::from(candidate.rule.relevance) / 100.0,
                    );

                    let license_match = LicenseMatch {
                        license_expression: candidate.rule.license_expression.clone(),
                        license_expression_spdx: index
                            .rule_metadata_by_identifier
                            .get(&candidate.rule.identifier)
                            .and_then(|metadata| metadata.license_expression_spdx.clone()),
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
                        rule_url: candidate.rule.rule_url().unwrap_or_default(),
                        matched_text: None,
                        referenced_filenames: candidate.rule.referenced_filenames.clone(),
                        rule_kind: candidate.rule.kind(),
                        is_from_license: candidate.rule.is_from_license,
                        rule_start_token: ipos,
                        coordinates: MatchCoordinates::rule_aligned(qspan, ispan, hispan),
                    };

                    matches.push(license_match);
                }

                qstart = max_qend;
            }
        }
    }

    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::license_detection::index::dictionary::{TokenId, tid};
    use std::collections::HashMap;
    use std::ops::Range;

    fn tids(values: &[u16]) -> Vec<TokenId> {
        values.iter().copied().map(tid).collect()
    }

    fn borrowed_postings(postings: &HashMap<TokenId, Vec<usize>>) -> super::HighPostings<'_> {
        postings
            .iter()
            .map(|(&tid, v)| (tid, v.as_slice()))
            .collect()
    }

    fn find_longest_match_for_test(
        query_tokens: &[TokenId],
        rule_tokens: &[TokenId],
        query_range: Range<usize>,
        rule_range: Range<usize>,
        high_postings: &HashMap<TokenId, Vec<usize>>,
        len_legalese: usize,
        matchables: &BitSet,
    ) -> (usize, usize, usize) {
        let high_postings = borrowed_postings(high_postings);
        let occurrences = token_occurrences(query_tokens, matchables, len_legalese);
        let potential = run_potential(
            query_tokens,
            rule_tokens,
            matchables,
            &high_postings,
            len_legalese,
        );
        let context = MatchSearchContext {
            query_tokens,
            rule_tokens,
            high_postings: &high_postings,
            len_legalese,
            matchables,
            run_potential: &potential,
            occurrences: &occurrences,
            cancel: None,
        };
        find_longest_match_impl(
            &context,
            query_range.start,
            query_range.end,
            rule_range.start,
            rule_range.end,
        )
        .expect("Sequence matching without cancel should not time out")
    }

    fn match_blocks_for_test(
        query_tokens: &[TokenId],
        rule_tokens: &[TokenId],
        query_range: Range<usize>,
        high_postings: &HashMap<TokenId, Vec<usize>>,
        len_legalese: usize,
        matchables: &BitSet,
    ) -> Vec<(usize, usize, usize)> {
        let high_postings = borrowed_postings(high_postings);
        let occurrences = token_occurrences(query_tokens, matchables, len_legalese);
        let potential = run_potential(
            query_tokens,
            rule_tokens,
            matchables,
            &high_postings,
            len_legalese,
        );
        let context = MatchSearchContext {
            query_tokens,
            rule_tokens,
            high_postings: &high_postings,
            len_legalese,
            matchables,
            run_potential: &potential,
            occurrences: &occurrences,
            cancel: None,
        };
        match_blocks_impl(&context, query_range.start, query_range.end)
            .expect("Sequence block matching without cancel should not time out")
    }

    /// Exhaustive longest-match search with none of the production shortcuts.
    fn reference_longest_match(
        query: &[TokenId],
        rule: &[TokenId],
        query_range: Range<usize>,
        rule_range: Range<usize>,
        len_legalese: usize,
        matchables: &BitSet,
    ) -> (usize, usize, usize) {
        let active = |i: usize, j: usize| {
            query[i] == rule[j] && query[i].as_usize() < len_legalese && matchables.contains(i)
        };
        let mut run = vec![vec![0usize; rule.len()]; query.len()];
        let (mut best_i, mut best_j, mut best_size) = (query_range.start, rule_range.start, 0);
        for i in query_range.clone() {
            for j in rule_range.clone() {
                if !active(i, j) {
                    continue;
                }
                let previous = if i > query_range.start && j > rule_range.start {
                    run[i - 1][j - 1]
                } else {
                    0
                };
                run[i][j] = previous + 1;
                if run[i][j] > best_size {
                    best_size = run[i][j];
                    (best_i, best_j) = (i + 1 - best_size, j + 1 - best_size);
                }
            }
        }
        if best_size > 0 {
            while best_i > query_range.start
                && best_j > rule_range.start
                && query[best_i - 1] == rule[best_j - 1]
                && matchables.contains(best_i - 1)
            {
                best_i -= 1;
                best_j -= 1;
                best_size += 1;
            }
            while best_i + best_size < query_range.end
                && best_j + best_size < rule_range.end
                && query[best_i + best_size] == rule[best_j + best_size]
                && matchables.contains(best_i + best_size)
            {
                best_size += 1;
            }
        }
        (best_i, best_j, best_size)
    }

    #[test]
    fn test_sparse_and_dense_scans_match_an_exhaustive_search() {
        // Appending more than `MAX_SPARSE_RULE_TOKENS` tokens that never occur in
        // the query forces the dense scan without changing any match.
        let len_legalese = 60;
        let dummies: Vec<u16> = (50..51 + MAX_SPARSE_RULE_TOKENS as u16).collect();
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut next = |bound: usize| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % bound as u64) as usize
        };

        for _ in 0..300 {
            let query: Vec<u16> = (0..300 + next(200)).map(|_| next(40) as u16).collect();
            let rule: Vec<u16> = (0..4 + next(5)).map(|_| next(24) as u16).collect();
            let mut dense_rule = rule.clone();
            dense_rule.extend(&dummies);

            let query_lo = next(query.len() / 2);
            let query_range = query_lo..query_lo + 1 + next(query.len() - query_lo);
            let rule_lo = next(rule.len() / 2);
            let rule_range = rule_lo..rule_lo + 1 + next(rule.len() - rule_lo);

            let query_tokens = tids(&query);
            let matchables: BitSet = (0..query.len()).filter(|_| next(8) != 0).collect();
            let search = |rule: &[u16], rule_range: Range<usize>| {
                let rule_tokens = tids(rule);
                let mut postings: HashMap<TokenId, Vec<usize>> = HashMap::new();
                for (j, token) in rule_tokens.iter().enumerate() {
                    if token.as_usize() < len_legalese {
                        postings.entry(*token).or_default().push(j);
                    }
                }
                find_longest_match_for_test(
                    &query_tokens,
                    &rule_tokens,
                    query_range.clone(),
                    rule_range,
                    &postings,
                    len_legalese,
                    &matchables,
                )
            };
            let reference = |rule_range: Range<usize>| {
                reference_longest_match(
                    &query_tokens,
                    &tids(&rule),
                    query_range.clone(),
                    rule_range,
                    len_legalese,
                    &matchables,
                )
            };

            assert_eq!(
                search(&rule, rule_range.clone()),
                reference(rule_range.clone()),
                "sparse: query {query:?} {query_range:?} rule {rule:?} {rule_range:?}"
            );
            assert_eq!(
                search(&dense_rule, 0..dense_rule.len()),
                reference(0..rule.len()),
                "dense: query {query:?} {query_range:?} rule {rule:?}"
            );
        }
    }

    #[test]
    fn test_find_longest_match_basic() {
        let query_tokens = tids(&[0, 1, 2, 3]);
        let rule_tokens = tids(&[0, 1, 2, 3]);
        let mut high_postings: HashMap<TokenId, Vec<usize>> = HashMap::new();
        high_postings.insert(tid(0), vec![0]);
        high_postings.insert(tid(1), vec![1]);
        high_postings.insert(tid(2), vec![2]);
        high_postings.insert(tid(3), vec![3]);

        let matchables: BitSet = (0..query_tokens.len()).collect();

        let result = find_longest_match_for_test(
            &query_tokens,
            &rule_tokens,
            0..query_tokens.len(),
            0..rule_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert_eq!(result, (0, 0, 4), "Should find full match");
    }

    #[test]
    fn test_find_longest_match_with_gap() {
        let query_tokens = tids(&[0, 1, 99, 2, 3]);
        let rule_tokens = tids(&[0, 1, 2, 3]);
        let mut high_postings: HashMap<TokenId, Vec<usize>> = HashMap::new();
        high_postings.insert(tid(0), vec![0]);
        high_postings.insert(tid(1), vec![1]);
        high_postings.insert(tid(2), vec![2]);
        high_postings.insert(tid(3), vec![3]);

        let matchables: BitSet = (0..query_tokens.len()).collect();

        let result = find_longest_match_for_test(
            &query_tokens,
            &rule_tokens,
            0..query_tokens.len(),
            0..rule_tokens.len(),
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
        let query_tokens = tids(&[0, 10, 2]);
        let rule_tokens = tids(&[0, 1, 2]);
        let mut high_postings: HashMap<TokenId, Vec<usize>> = HashMap::new();
        high_postings.insert(tid(0), vec![0]);
        high_postings.insert(tid(2), vec![2]);

        let matchables: BitSet = (0..query_tokens.len()).collect();

        let result = find_longest_match_for_test(
            &query_tokens,
            &rule_tokens,
            0..query_tokens.len(),
            0..rule_tokens.len(),
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
        let query_tokens = tids(&[10, 11, 12]);
        let rule_tokens = tids(&[0, 1, 2]);
        let high_postings: HashMap<TokenId, Vec<usize>> = HashMap::new();

        let matchables: BitSet = (0..query_tokens.len()).collect();

        let result = find_longest_match_for_test(
            &query_tokens,
            &rule_tokens,
            0..query_tokens.len(),
            0..rule_tokens.len(),
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
        let query_tokens = tids(&[0, 1, 2, 0, 1, 2, 0, 1, 2]);
        let rule_tokens = tids(&[0, 1, 2]);
        let mut high_postings: HashMap<TokenId, Vec<usize>> = HashMap::new();
        high_postings.insert(tid(0), vec![0]);
        high_postings.insert(tid(1), vec![1]);
        high_postings.insert(tid(2), vec![2]);

        let matchables: BitSet = (0..query_tokens.len()).collect();

        let result = find_longest_match_for_test(
            &query_tokens,
            &rule_tokens,
            3..6,
            0..rule_tokens.len(),
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
        let query_tokens = tids(&[0, 1, 2]);
        let rule_tokens = tids(&[0, 1, 2]);
        let mut high_postings: HashMap<TokenId, Vec<usize>> = HashMap::new();
        high_postings.insert(tid(0), vec![0]);
        high_postings.insert(tid(1), vec![1]);
        high_postings.insert(tid(2), vec![2]);

        let matchables: BitSet = [0, 2].into_iter().collect();

        let result = find_longest_match_for_test(
            &query_tokens,
            &rule_tokens,
            0..query_tokens.len(),
            0..rule_tokens.len(),
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
        let query_tokens = tids(&[0, 1, 2, 3]);
        let rule_tokens = tids(&[0, 1, 2, 3]);
        let mut high_postings: HashMap<TokenId, Vec<usize>> = HashMap::new();
        high_postings.insert(tid(0), vec![0]);
        high_postings.insert(tid(1), vec![1]);
        high_postings.insert(tid(2), vec![2]);
        high_postings.insert(tid(3), vec![3]);

        let matchables: BitSet = (0..query_tokens.len()).collect();

        let blocks = match_blocks_for_test(
            &query_tokens,
            &rule_tokens,
            0..query_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert_eq!(blocks.len(), 1, "Should find single full match");
        assert_eq!(blocks[0], (0, 0, 4), "Should match entire sequence");
    }

    #[test]
    fn test_match_blocks_collapse_adjacent() {
        let query_tokens = tids(&[0, 1, 2, 3, 4]);
        let rule_tokens = tids(&[0, 1, 2, 3, 4]);
        let mut high_postings: HashMap<TokenId, Vec<usize>> = HashMap::new();
        for (i, &tid) in query_tokens.iter().enumerate() {
            high_postings.entry(tid).or_default().push(i);
        }

        let matchables: BitSet = (0..query_tokens.len()).collect();

        let blocks = match_blocks_for_test(
            &query_tokens,
            &rule_tokens,
            0..query_tokens.len(),
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
        let query_tokens = tids(&[10, 11, 12]);
        let rule_tokens = tids(&[0, 1, 2]);
        let high_postings: HashMap<TokenId, Vec<usize>> = HashMap::new();

        let matchables: BitSet = (0..query_tokens.len()).collect();

        let blocks = match_blocks_for_test(
            &query_tokens,
            &rule_tokens,
            0..query_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert!(blocks.is_empty(), "Should return empty when no matches");
    }

    #[test]
    fn test_match_blocks_empty_query() {
        let query_tokens = tids(&[]);
        let rule_tokens = tids(&[0, 1, 2]);
        let high_postings: HashMap<TokenId, Vec<usize>> = HashMap::new();
        let matchables: BitSet = BitSet::new();

        let blocks = match_blocks_for_test(
            &query_tokens,
            &rule_tokens,
            0..query_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert!(blocks.is_empty());
    }

    #[test]
    fn test_match_blocks_with_gap() {
        let query_tokens = tids(&[0, 1, 99, 2, 3]);
        let rule_tokens = tids(&[0, 1, 2, 3]);
        let mut high_postings: HashMap<TokenId, Vec<usize>> = HashMap::new();
        high_postings.insert(tid(0), vec![0]);
        high_postings.insert(tid(1), vec![1]);
        high_postings.insert(tid(2), vec![2]);
        high_postings.insert(tid(3), vec![3]);

        let matchables: BitSet = (0..query_tokens.len()).collect();

        let blocks = match_blocks_for_test(
            &query_tokens,
            &rule_tokens,
            0..query_tokens.len(),
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
        let query_tokens = tids(&[0, 1, 2]);
        let rule_tokens = tids(&[]);
        let high_postings: HashMap<TokenId, Vec<usize>> = HashMap::new();
        let matchables: BitSet = (0..query_tokens.len()).collect();

        let blocks = match_blocks_for_test(
            &query_tokens,
            &rule_tokens,
            0..query_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );

        assert!(blocks.is_empty());
    }

    #[test]
    fn test_match_blocks_multiple_regions() {
        let query_tokens = tids(&[0, 1, 99, 2, 3, 88, 0, 1]);
        let rule_tokens = tids(&[0, 1, 2, 3]);
        let mut high_postings: HashMap<TokenId, Vec<usize>> = HashMap::new();
        high_postings.insert(tid(0), vec![0]);
        high_postings.insert(tid(1), vec![1]);
        high_postings.insert(tid(2), vec![2]);
        high_postings.insert(tid(3), vec![3]);

        let matchables: BitSet = (0..query_tokens.len()).collect();

        let blocks = match_blocks_for_test(
            &query_tokens,
            &rule_tokens,
            0..query_tokens.len(),
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
        let query_tokens = tids(&[0, 1, 2, 99, 0, 1, 2]);
        let rule_tokens = tids(&[0, 1, 2]);
        let mut high_postings: HashMap<TokenId, Vec<usize>> = HashMap::new();
        high_postings.insert(tid(0), vec![0]);
        high_postings.insert(tid(1), vec![1]);
        high_postings.insert(tid(2), vec![2]);

        let matchables: BitSet = (0..query_tokens.len()).collect();

        let blocks = match_blocks_for_test(
            &query_tokens,
            &rule_tokens,
            0..3,
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

        let blocks2 = match_blocks_for_test(
            &query_tokens,
            &rule_tokens,
            4..query_tokens.len(),
            &high_postings,
            5,
            &matchables,
        );
        assert_eq!(blocks2.len(), 1, "Should find the second occurrence");
        assert_eq!(blocks2[0], (4, 0, 3));
    }

    #[test]
    fn test_extend_match_into_low_tokens() {
        let query_tokens = tids(&[0, 1, 2, 10, 11]);
        let rule_tokens = tids(&[0, 1, 2, 10, 11]);
        let mut high_postings: HashMap<TokenId, Vec<usize>> = HashMap::new();
        high_postings.insert(tid(0), vec![0]);
        high_postings.insert(tid(1), vec![1]);
        high_postings.insert(tid(2), vec![2]);

        let matchables: BitSet = (0..query_tokens.len()).collect();

        let blocks = match_blocks_for_test(
            &query_tokens,
            &rule_tokens,
            0..query_tokens.len(),
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
        let query_tokens = tids(&[0, 1, 2, 10, 11]);
        let rule_tokens = tids(&[0, 1, 2, 10, 11]);
        let mut high_postings: HashMap<TokenId, Vec<usize>> = HashMap::new();
        high_postings.insert(tid(0), vec![0]);
        high_postings.insert(tid(1), vec![1]);
        high_postings.insert(tid(2), vec![2]);

        let matchables: BitSet = [0, 1, 2].into_iter().collect();

        let blocks = match_blocks_for_test(
            &query_tokens,
            &rule_tokens,
            0..query_tokens.len(),
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
