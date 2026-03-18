//! Candidate selection using set and multiset similarity.

use crate::license_detection::index::LicenseIndex;
use crate::license_detection::index::token_sets::{
    build_set_and_mset, high_multiset_subset, high_tids_set_subset, tids_set_counter,
};
use crate::license_detection::models::Rule;
use crate::license_detection::query::QueryRun;
use std::collections::{HashMap, HashSet};

use super::HIGH_RESEMBLANCE_THRESHOLD;

/// Score vector for ranking candidates using set similarity.
///
/// Contains metrics computed from set/multiset intersections.
///
/// Corresponds to Python: `ScoresVector` namedtuple in match_set.py (line 458)
#[derive(Debug, Clone, PartialEq)]
pub struct ScoresVector {
    /// True if the sets are highly similar (resemblance >= threshold)
    pub is_highly_resemblant: bool,
    /// Containment ratio (how much of rule is in query)
    pub containment: f32,
    /// Amplified resemblance (squared to boost high values)
    pub resemblance: f32,
    /// Number of matched tokens (normalized for ranking)
    pub matched_length: f32,
    /// Rule ID for tie-breaking
    pub rid: usize,
}

impl PartialOrd for ScoresVector {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for ScoresVector {}

impl Ord for ScoresVector {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Python sorts ScoresVector namedtuple with reverse=True:
        // 1. is_highly_resemblant (True > False)
        // 2. containment (higher is better)
        // 3. resemblance (higher is better)
        // 4. matched_length (higher is better)
        // Note: Python does NOT use rid for tie-breaking in ScoresVector
        self.is_highly_resemblant
            .cmp(&other.is_highly_resemblant)
            .then_with(|| {
                self.containment
                    .partial_cmp(&other.containment)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
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
    }
}

/// Candidate with its score vector and metadata.
///
/// Corresponds to the tuple structure used in Python: (scores_vectors, rid, rule, high_set_intersection)
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// Rounded score vector for display/grouping
    pub score_vec_rounded: ScoresVector,
    /// Full score vector for sorting
    pub score_vec_full: ScoresVector,
    /// Rule ID
    pub rid: usize,
    /// Reference to the rule
    pub rule: Rule,
    /// Set of high-value (legalese) tokens in the intersection
    pub high_set_intersection: HashSet<u16>,
}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Candidate {}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Python sorts the tuple ((svr, svf), rid, rule, ...) with reverse=True
        // So it compares (svr, svf) tuple first, which means:
        // 1. Compare rounded (svr) first
        // 2. Then compare full (svf) if rounded is equal
        // 3. Then compare rid if scores are still equal.
        compare_candidate_rank(
            &self.score_vec_rounded,
            &self.score_vec_full,
            self.rid,
            &other.score_vec_rounded,
            &other.score_vec_full,
            other.rid,
        )
    }
}

fn compare_candidate_rank(
    rounded: &ScoresVector,
    full: &ScoresVector,
    rid: usize,
    other_rounded: &ScoresVector,
    other_full: &ScoresVector,
    other_rid: usize,
) -> std::cmp::Ordering {
    rounded
        .cmp(other_rounded)
        .then_with(|| full.cmp(other_full))
        .then_with(|| rid.cmp(&other_rid))
}

fn python_round_tenths(value: f64) -> f32 {
    let rendered = format!("{value:.20}");
    let (whole, frac) = rendered.split_once('.').unwrap_or((rendered.as_str(), "0"));

    let whole_part: i64 = whole.parse().unwrap_or(0);
    let mut frac_chars = frac.chars();
    let tenths = frac_chars
        .next()
        .and_then(|c| c.to_digit(10))
        .unwrap_or(0) as i64;
    let rest: String = frac_chars.collect();

    let threshold = format!("5{}", "0".repeat(rest.len().saturating_sub(1)));
    let should_round_up = if rest > threshold {
        true
    } else if rest == threshold {
        tenths % 2 == 1
    } else {
        false
    };

    let mut scaled = whole_part * 10 + tenths;
    if should_round_up {
        scaled += 1;
    }

    scaled as f32 / 10.0
}

fn quantize_tenths(value: f32) -> i32 {
    format!("{value:.1}")
        .chars()
        .filter(|c| *c != '.')
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

fn build_score_vectors(
    resemblance: f64,
    containment: f64,
    matched_length: usize,
    rid: usize,
) -> (ScoresVector, ScoresVector) {
    let amplified_resemblance = resemblance * resemblance;

    let score_vec_rounded = ScoresVector {
        is_highly_resemblant: python_round_tenths(resemblance) >= HIGH_RESEMBLANCE_THRESHOLD,
        containment: python_round_tenths(containment),
        resemblance: python_round_tenths(amplified_resemblance),
        matched_length: python_round_tenths(matched_length as f64 / 20.0),
        rid,
    };

    let score_vec_full = ScoresVector {
        is_highly_resemblant: resemblance >= f64::from(HIGH_RESEMBLANCE_THRESHOLD),
        containment: containment as f32,
        resemblance: amplified_resemblance as f32,
        matched_length: matched_length as f32,
        rid,
    };

    (score_vec_rounded, score_vec_full)
}

/// Key for grouping duplicate candidates.
///
/// Candidates with the same DupeGroupKey are considered duplicates,
/// and only the best one is kept.
///
/// Corresponds to Python: `filter_dupes.group_key()` in match_set.py (line 467-476)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct DupeGroupKey {
    license_expression: String,
    is_highly_resemblant: bool,
    containment: i32,
    resemblance: i32,
    matched_length: i32,
    rule_length: usize,
}

/// Filter duplicate candidates, keeping only the best from each group.
///
/// Candidates are grouped by (license_expression, is_highly_resemblant, containment,
/// resemblance, matched_length, rule_length). Within each group, candidates are
/// ranked by (score_vec_full, rule.identifier) and only the best is kept.
///
/// This matches Python's filter_dupes behavior where matched_length uses 1-decimal
/// precision (e.g., 6.9 and 6.7 are different, but 7 and 7 would be same).
///
/// Corresponds to Python: `filter_dupes()` in match_set.py (line 461-498)
pub(super) fn filter_dupes(candidates: Vec<Candidate>) -> Vec<Candidate> {
    let mut groups: HashMap<DupeGroupKey, Vec<Candidate>> = HashMap::new();

    for candidate in candidates {
        let key = DupeGroupKey {
            license_expression: candidate.rule.license_expression.clone(),
            is_highly_resemblant: candidate.score_vec_rounded.is_highly_resemblant,
            containment: quantize_tenths(candidate.score_vec_rounded.containment),
            resemblance: quantize_tenths(candidate.score_vec_rounded.resemblance),
            matched_length: quantize_tenths(candidate.score_vec_rounded.matched_length),
            rule_length: candidate.rule.tokens.len(),
        };
        groups.entry(key).or_default().push(candidate);
    }

    let mut result: Vec<Candidate> = Vec::new();
    for mut group in groups.into_values() {
        // Python: duplicates = sorted(duplicates, reverse=True, key=lambda x: (sv_full, rule.identifier))
        // Higher sv_full wins, then HIGHER identifier alphabetically (reverse=True)
        group.sort_by(|a, b| {
            b.score_vec_full
                .cmp(&a.score_vec_full)
                .then_with(|| b.rule.identifier.cmp(&a.rule.identifier))
        });
        if let Some(best) = group.into_iter().next() {
            result.push(best);
        }
    }

    result
}

/// Compute intersection of two multisets.
///
/// For each token ID present in both multisets, the intersection value is the
/// smaller of the occurrence counts.
///
/// Corresponds to Python: `multisets_intersector()` in match_set.py (line 119)
pub fn multisets_intersector(
    qmset: &HashMap<u16, usize>,
    imset: &HashMap<u16, usize>,
) -> HashMap<u16, usize> {
    let (set1, set2) = if qmset.len() < imset.len() {
        (qmset, imset)
    } else {
        (imset, qmset)
    };

    set1.iter()
        .filter_map(|(&tid, &count1)| set2.get(&tid).map(|&count2| (tid, count1.min(count2))))
        .collect()
}

/// Compute multiset-based candidates (Phase 2 refinement).
///
/// After selecting candidates using sets, this refines the ranking using multisets.
///
/// Corresponds to Python: `compute_candidates()` step 2 in match_set.py (line 311-350)
pub fn compute_candidates_with_msets(
    index: &LicenseIndex,
    query_run: &QueryRun,
    high_resemblance: bool,
    top_n: usize,
) -> Vec<Candidate> {
    let query_tokens = query_run.matchable_tokens();
    if query_tokens.is_empty() {
        return Vec::new();
    }

    let query_token_ids: Vec<u16> = query_tokens
        .iter()
        .filter_map(|&tid| if tid >= 0 { Some(tid as u16) } else { None })
        .collect();

    if query_token_ids.is_empty() {
        return Vec::new();
    }

    let (query_set, query_mset) = build_set_and_mset(&query_token_ids);
    let len_legalese = index.len_legalese;

    let mut step1_candidates: Vec<(ScoresVector, ScoresVector, usize, Rule, HashSet<u16>)> =
        Vec::new();

    for (rid, rule) in index.rules_by_rid.iter().enumerate() {
        if !index.approx_matchable_rids.contains(&rid) {
            continue;
        }

        let Some(rule_set) = index.sets_by_rid.get(&rid) else {
            continue;
        };
        let Some(_rule_mset) = index.msets_by_rid.get(&rid) else {
            continue;
        };

        let intersection: HashSet<u16> = query_set.intersection(rule_set).copied().collect();
        if intersection.is_empty() {
            continue;
        }

        let high_set_intersection = high_tids_set_subset(&intersection, len_legalese);
        if high_set_intersection.is_empty() {
            continue;
        }

        // Check high token threshold (this is separate from matched_length!)
        let high_matched_length = tids_set_counter(&high_set_intersection);
        if high_matched_length < rule.min_high_matched_length_unique {
            continue;
        }

        // Check total intersection threshold
        let matched_length = tids_set_counter(&intersection);
        if matched_length < rule.min_matched_length_unique {
            continue;
        }

        // Compute resemblance using TOTAL intersection, not just high
        let qset_len = query_set.len();
        let iset_len = rule.length_unique;
        if qset_len == 0 || iset_len == 0 {
            continue;
        }

        let union_len = qset_len + iset_len - matched_length;
        let resemblance = matched_length as f64 / union_len as f64;
        let containment = matched_length as f64 / iset_len as f64;

        // Check minimum_containment (Python: match_set.py:429-433)
        // Rules with minimum_coverage require a minimum containment ratio
        let minimum_containment = rule.minimum_coverage.map(|mc| mc as f64 / 100.0);
        if let Some(min_cont) = minimum_containment
            && containment < min_cont
        {
            continue;
        }

        let (svr, svf) = build_score_vectors(resemblance, containment, matched_length, rid);

        if high_resemblance && (!svr.is_highly_resemblant || !svf.is_highly_resemblant) {
            continue;
        }

        step1_candidates.push((svr, svf, rid, rule.clone(), high_set_intersection));
    }

    if step1_candidates.is_empty() {
        return Vec::new();
    }

    step1_candidates.sort_by(|a, b| {
        compare_candidate_rank(&b.0, &b.1, b.2, &a.0, &a.1, a.2)
    });

    step1_candidates.truncate(top_n * 10);

    let mut sortable_candidates: Vec<Candidate> = Vec::new();

    for (_svr, _svf, rid, rule, high_set_intersection) in step1_candidates {
        let Some(rule_mset) = index.msets_by_rid.get(&rid) else {
            continue;
        };

        // Filter using HIGH multisets (Python: high_intersection check)
        let query_high_mset = high_multiset_subset(&query_mset, len_legalese);
        let rule_high_mset = high_multiset_subset(rule_mset, len_legalese);
        let high_intersection_mset = multisets_intersector(&query_high_mset, &rule_high_mset);
        if high_intersection_mset.is_empty() {
            continue;
        }

        let high_matched_length: usize = high_intersection_mset.values().sum();
        if high_matched_length < rule.min_high_matched_length {
            continue;
        }

        // Compute scores using FULL multisets (Python: matched_length = counter(intersection))
        let full_intersection_mset = multisets_intersector(&query_mset, rule_mset);
        let matched_length: usize = full_intersection_mset.values().sum();
        if matched_length < rule.min_matched_length {
            continue;
        }
        let qset_len: usize = query_mset.values().sum();
        let iset_len: usize = rule_mset.values().sum();

        if qset_len == 0 || iset_len == 0 {
            continue;
        }

        let union_len = qset_len + iset_len - matched_length;
        let resemblance = matched_length as f64 / union_len as f64;
        let containment = matched_length as f64 / iset_len as f64;

        // Check minimum_containment (Python: match_set.py:429-433)
        // Rules with minimum_coverage require a minimum containment ratio
        let minimum_containment = rule.minimum_coverage.map(|mc| mc as f64 / 100.0);
        if let Some(min_cont) = minimum_containment
            && containment < min_cont
        {
            continue;
        }

        let (score_vec_rounded, score_vec_full) =
            build_score_vectors(resemblance, containment, matched_length, rid);

        if high_resemblance
            && (!score_vec_rounded.is_highly_resemblant || !score_vec_full.is_highly_resemblant)
        {
            continue;
        }

        sortable_candidates.push(Candidate {
            score_vec_rounded,
            score_vec_full,
            rid,
            rule,
            high_set_intersection,
        });
    }

    sortable_candidates = filter_dupes(sortable_candidates);

    sortable_candidates.sort_by(|a, b| b.cmp(a));
    sortable_candidates.truncate(top_n);

    sortable_candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scores_vector_comparison() {
        let sv1 = ScoresVector {
            is_highly_resemblant: true,
            containment: 0.9,
            resemblance: 0.8,
            matched_length: 10.0,
            rid: 0,
        };

        let sv2 = ScoresVector {
            is_highly_resemblant: false,
            containment: 0.8,
            resemblance: 0.6,
            matched_length: 5.0,
            rid: 1,
        };

        assert!(sv1 > sv2);
    }

    #[test]
    fn test_python_round_tenths_matches_python_half_even_behavior() {
        assert_eq!(python_round_tenths(0.05), 0.1);
        assert_eq!(python_round_tenths(0.15), 0.1);
        assert_eq!(python_round_tenths(0.25), 0.2);
        assert_eq!(python_round_tenths(2.25), 2.2);
        assert_eq!(python_round_tenths(4.35), 4.3);
        assert_eq!(python_round_tenths(6.65), 6.7);
    }

    #[test]
    fn test_candidate_ordering() {
        let candidate1 = Candidate {
            score_vec_rounded: ScoresVector {
                is_highly_resemblant: true,
                containment: 0.9,
                resemblance: 0.8,
                matched_length: 10.0,
                rid: 0,
            },
            score_vec_full: ScoresVector {
                is_highly_resemblant: true,
                containment: 0.9,
                resemblance: 0.8,
                matched_length: 10.0,
                rid: 0,
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
                has_stored_minimum_coverage: false,
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
                starts_with_license: false,
                ends_with_license: false,
                is_deprecated: false,
                spdx_license_key: None,
                other_spdx_license_keys: vec![],
                required_phrase_spans: vec![],
                stopwords_by_pos: std::collections::HashMap::new(),
            },
            high_set_intersection: HashSet::new(),
        };

        let candidate2 = Candidate {
            score_vec_rounded: ScoresVector {
                is_highly_resemblant: false,
                containment: 0.5,
                resemblance: 0.3,
                matched_length: 5.0,
                rid: 1,
            },
            score_vec_full: ScoresVector {
                is_highly_resemblant: false,
                containment: 0.5,
                resemblance: 0.3,
                matched_length: 5.0,
                rid: 1,
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
                has_stored_minimum_coverage: false,
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
                starts_with_license: false,
                ends_with_license: false,
                is_deprecated: false,
                spdx_license_key: None,
                other_spdx_license_keys: vec![],
                required_phrase_spans: vec![],
                stopwords_by_pos: std::collections::HashMap::new(),
            },
            high_set_intersection: HashSet::new(),
        };

        assert!(
            candidate1 > candidate2,
            "Higher containment candidate should rank higher"
        );
    }

    #[test]
    fn test_filter_dupes_matched_length_precision() {
        let rule1 = Rule {
            identifier: "x11-dec1.RULE".to_string(),
            license_expression: "x11-dec1".to_string(),
            text: String::new(),
            tokens: vec![0; 138],
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
            has_stored_minimum_coverage: false,
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
            starts_with_license: false,
            ends_with_license: false,
            is_deprecated: false,
            spdx_license_key: None,
            other_spdx_license_keys: vec![],
            required_phrase_spans: vec![],
            stopwords_by_pos: std::collections::HashMap::new(),
        };

        let rule2 = Rule {
            identifier: "cmu-uc.RULE".to_string(),
            license_expression: "cmu-uc".to_string(),
            text: String::new(),
            tokens: vec![0; 133],
            ..rule1.clone()
        };

        let candidate1 = Candidate {
            score_vec_rounded: ScoresVector {
                is_highly_resemblant: false,
                containment: 0.5,
                resemblance: 0.25,
                matched_length: 7.0,
                rid: 1,
            },
            score_vec_full: ScoresVector {
                is_highly_resemblant: false,
                containment: 0.5,
                resemblance: 0.25,
                matched_length: 138.0,
                rid: 1,
            },
            rid: 1,
            rule: rule1,
            high_set_intersection: HashSet::new(),
        };

        let candidate2 = Candidate {
            score_vec_rounded: ScoresVector {
                is_highly_resemblant: false,
                containment: 0.5,
                resemblance: 0.25,
                matched_length: 7.0,
                rid: 2,
            },
            score_vec_full: ScoresVector {
                is_highly_resemblant: false,
                containment: 0.5,
                resemblance: 0.25,
                matched_length: 133.0,
                rid: 2,
            },
            rid: 2,
            rule: rule2,
            high_set_intersection: HashSet::new(),
        };

        let candidates = vec![candidate1, candidate2];
        let filtered = filter_dupes(candidates);

        assert_eq!(
            filtered.len(),
            2,
            "Should keep both candidates when matched_length differs at 1-decimal precision: 138/20=6.9 vs 133/20=6.7"
        );
    }

    #[test]
    fn test_filter_dupes_same_group() {
        let rule1 = Rule {
            identifier: "mit.RULE".to_string(),
            license_expression: "mit".to_string(),
            text: String::new(),
            tokens: vec![0; 100],
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
            has_stored_minimum_coverage: false,
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
            starts_with_license: false,
            ends_with_license: false,
            is_deprecated: false,
            spdx_license_key: None,
            other_spdx_license_keys: vec![],
            required_phrase_spans: vec![],
            stopwords_by_pos: std::collections::HashMap::new(),
        };

        let rule2 = Rule {
            identifier: "mit_2.RULE".to_string(),
            license_expression: "mit".to_string(),
            text: String::new(),
            tokens: vec![0; 100],
            ..rule1.clone()
        };

        let candidate1 = Candidate {
            score_vec_rounded: ScoresVector {
                is_highly_resemblant: false,
                containment: 0.5,
                resemblance: 0.25,
                matched_length: 5.0,
                rid: 1,
            },
            score_vec_full: ScoresVector {
                is_highly_resemblant: false,
                containment: 0.5,
                resemblance: 0.25,
                matched_length: 100.0,
                rid: 1,
            },
            rid: 1,
            rule: rule1,
            high_set_intersection: HashSet::new(),
        };

        let candidate2 = Candidate {
            score_vec_rounded: ScoresVector {
                is_highly_resemblant: false,
                containment: 0.5,
                resemblance: 0.25,
                matched_length: 5.0,
                rid: 2,
            },
            score_vec_full: ScoresVector {
                is_highly_resemblant: false,
                containment: 0.5,
                resemblance: 0.25,
                matched_length: 100.0,
                rid: 2,
            },
            rid: 2,
            rule: rule2,
            high_set_intersection: HashSet::new(),
        };

        let candidates = vec![candidate1, candidate2];
        let filtered = filter_dupes(candidates);

        assert_eq!(
            filtered.len(),
            1,
            "Should keep only one candidate when all group keys match"
        );
    }

    #[test]
    fn test_filter_dupes_prefers_higher_identifier_when_full_scores_tie() {
        let rule_sa = Rule {
            identifier: "cc-by-sa-1.0.RULE".to_string(),
            license_expression: "cc-by-sa-1.0".to_string(),
            text: String::new(),
            tokens: vec![0; 1960],
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
            has_stored_minimum_coverage: false,
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
            starts_with_license: false,
            ends_with_license: false,
            is_deprecated: false,
            spdx_license_key: None,
            other_spdx_license_keys: vec![],
            required_phrase_spans: vec![],
            stopwords_by_pos: std::collections::HashMap::new(),
        };

        let rule_nc_sa = Rule {
            identifier: "cc-by-nc-sa-1.0.RULE".to_string(),
            license_expression: "cc-by-nc-sa-1.0".to_string(),
            text: String::new(),
            tokens: vec![0; 1829],
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
            has_stored_minimum_coverage: false,
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
            starts_with_license: false,
            ends_with_license: false,
            is_deprecated: false,
            spdx_license_key: None,
            other_spdx_license_keys: vec![],
            required_phrase_spans: vec![],
            stopwords_by_pos: std::collections::HashMap::new(),
        };

        let candidate_sa = Candidate {
            score_vec_rounded: ScoresVector {
                is_highly_resemblant: true,
                containment: 0.9,
                resemblance: 0.8,
                matched_length: 100.0,
                rid: 1,
            },
            score_vec_full: ScoresVector {
                is_highly_resemblant: true,
                containment: 0.9,
                resemblance: 0.8,
                matched_length: 100.0,
                rid: 1,
            },
            rid: 1,
            rule: rule_sa,
            high_set_intersection: HashSet::new(),
        };

        let candidate_nc_sa = Candidate {
            score_vec_rounded: ScoresVector {
                is_highly_resemblant: true,
                containment: 0.9,
                resemblance: 0.8,
                matched_length: 100.0,
                rid: 2,
            },
            score_vec_full: ScoresVector {
                is_highly_resemblant: true,
                containment: 0.9,
                resemblance: 0.8,
                matched_length: 100.0,
                rid: 2,
            },
            rid: 2,
            rule: rule_nc_sa,
            high_set_intersection: HashSet::new(),
        };

        let candidates = vec![candidate_nc_sa, candidate_sa];
        let filtered = filter_dupes(candidates);

        assert_eq!(
            filtered.len(),
            2,
            "Different license expressions should create different groups"
        );

        let mut same_group_candidates = vec![filtered[0].clone(), filtered[1].clone()];
        for candidate in &mut same_group_candidates {
            candidate.rule.license_expression = "same".to_string();
            candidate.rule.tokens = vec![0; 100];
        }

        let deduped = filter_dupes(same_group_candidates);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].rule.identifier, "cc-by-sa-1.0.RULE");
    }

    #[test]
    fn test_candidate_ordering_uses_rid_after_equal_scores() {
        let rule_a = Rule {
            identifier: "a.RULE".to_string(),
            license_expression: "a".to_string(),
            text: String::new(),
            tokens: vec![0; 10],
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
            has_stored_minimum_coverage: false,
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
            starts_with_license: false,
            ends_with_license: false,
            is_deprecated: false,
            spdx_license_key: None,
            other_spdx_license_keys: vec![],
            required_phrase_spans: vec![],
            stopwords_by_pos: std::collections::HashMap::new(),
        };

        let candidate_low_rid = Candidate {
            score_vec_rounded: ScoresVector {
                is_highly_resemblant: true,
                containment: 0.9,
                resemblance: 0.8,
                matched_length: 10.0,
                rid: 1,
            },
            score_vec_full: ScoresVector {
                is_highly_resemblant: true,
                containment: 0.9,
                resemblance: 0.8,
                matched_length: 10.0,
                rid: 1,
            },
            rid: 1,
            rule: Rule {
                identifier: "z.RULE".to_string(),
                ..rule_a.clone()
            },
            high_set_intersection: HashSet::new(),
        };

        let candidate_high_rid = Candidate {
            score_vec_rounded: ScoresVector {
                rid: 2,
                ..candidate_low_rid.score_vec_rounded.clone()
            },
            score_vec_full: ScoresVector {
                rid: 2,
                ..candidate_low_rid.score_vec_full.clone()
            },
            rid: 2,
            rule: Rule {
                identifier: "a.RULE".to_string(),
                ..rule_a
            },
            high_set_intersection: HashSet::new(),
        };

        let mut sorted = [candidate_low_rid, candidate_high_rid];
        sorted.sort_by(|a, b| b.cmp(a));
        assert_eq!(
            sorted[0].rid, 2,
            "Python final candidate tuple ordering falls back to higher rid after equal scores"
        );
    }
}
