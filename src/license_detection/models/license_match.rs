//! License match result from a matching strategy.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

fn default_rule_length() -> usize {
    0
}

/// License match result from a matching strategy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LicenseMatch {
    /// Internal rule ID for fast lookups (index into rules_by_rid).
    /// Not serialized to JSON output.
    #[serde(skip)]
    pub rid: usize,

    /// License expression string using ScanCode license keys
    pub license_expression: String,

    /// License expression with SPDX-only keys
    pub license_expression_spdx: String,

    /// File where match was found (if applicable)
    pub from_file: Option<String>,

    /// Start line number (1-indexed)
    pub start_line: usize,

    /// End line number (1-indexed)
    pub end_line: usize,

    /// Start token position (0-indexed in query token stream)
    /// Used for dual-criteria match grouping with token gap threshold.
    #[serde(default)]
    pub start_token: usize,

    /// End token position (0-indexed, exclusive)
    /// Used for dual-criteria match grouping with token gap threshold.
    #[serde(default)]
    pub end_token: usize,

    /// Name of the matching strategy used
    pub matcher: String,

    /// Match score 0.0-1.0
    pub score: f32,

    /// Length of matched text in characters
    pub matched_length: usize,

    /// Token count of the matched rule (from rule.tokens.len())
    /// Used for false positive detection instead of matched_length.
    #[serde(default = "default_rule_length")]
    pub rule_length: usize,

    /// Match coverage as percentage 0.0-100.0
    pub match_coverage: f32,

    /// Relevance of the matched rule (0-100)
    pub rule_relevance: u8,

    /// Unique identifier for the matched rule
    pub rule_identifier: String,

    /// URL for the matched rule
    pub rule_url: String,

    /// Matched text snippet (optional for privacy/performance)
    pub matched_text: Option<String>,

    /// Filenames referenced by this match (e.g., ["LICENSE"] for "See LICENSE file")
    /// Populated from rule.referenced_filenames when rule matches
    pub referenced_filenames: Option<Vec<String>>,

    /// True if this match is from a license intro rule
    pub is_license_intro: bool,

    /// True if this match is from a license clue rule
    pub is_license_clue: bool,

    /// True if this match is from a license reference rule
    #[serde(default)]
    pub is_license_reference: bool,

    /// True if this match is from a license tag rule
    #[serde(default)]
    pub is_license_tag: bool,

    /// True if this match is from a license text rule (full license text, not notice)
    #[serde(default)]
    pub is_license_text: bool,

    /// True if this match is from a rule created from a license file (not a .RULE file)
    /// Rules from LICENSE files have relevance=100 and should take priority over decomposed expressions.
    #[serde(default)]
    pub is_from_license: bool,

    /// Token positions matched by this license (for span subtraction).
    ///
    /// Populated during matching to enable double-match prevention.
    /// None means contiguous range [start_token, end_token).
    /// Some(positions) contains the exact positions for non-contiguous matches.
    #[serde(skip)]
    pub matched_token_positions: Option<Vec<usize>>,

    /// Count of matched high-value legalese tokens (token IDs < len_legalese).
    ///
    /// Corresponds to Python's `len(self.hispan)` - the number of matched positions
    /// where the token ID is a high-value legalese token.
    #[serde(default)]
    pub hilen: usize,

    /// Rule-side start position (where in the rule text the match starts).
    ///
    /// This is Python's "istart" - the position in the rule, not the query.
    /// Used by `ispan()` to return rule-side positions for required phrase checking.
    ///
    /// For exact matches (hash, aho), this is always 0.
    /// For approximate matches (seq), this is the position in the rule where alignment begins.
    #[serde(default)]
    pub rule_start_token: usize,

    /// Token positions matched in the query text.
    /// None means contiguous range [start_token, end_token).
    /// Some(positions) contains exact positions for non-contiguous matches (after merge).
    #[serde(skip)]
    pub qspan_positions: Option<Vec<usize>>,

    /// Token positions matched in the rule text.
    /// None means contiguous range [rule_start_token, rule_start_token + matched_length).
    /// Some(positions) contains exact positions for non-contiguous matches (after merge).
    #[serde(skip)]
    pub ispan_positions: Option<Vec<usize>>,

    /// Token positions in the rule that are high-value legalese tokens.
    /// None means hispan can be computed from rule_start_token (contiguous case).
    /// Some(positions) contains exact positions for non-contiguous hispans (after merge).
    #[serde(skip)]
    pub hispan_positions: Option<Vec<usize>>,

    /// Candidate resemblance score from set similarity.
    /// Used for cross-license tie-breaking when matches overlap.
    /// Higher resemblance means better candidate quality.
    #[serde(default)]
    pub candidate_resemblance: f32,

    /// Candidate containment score from set similarity.
    /// Used for cross-license tie-breaking when matches overlap.
    /// Higher containment means more of the rule is matched.
    #[serde(default)]
    pub candidate_containment: f32,
}

impl Default for LicenseMatch {
    fn default() -> Self {
        LicenseMatch {
            rid: 0,
            license_expression: String::new(),
            license_expression_spdx: String::new(),
            from_file: None,
            start_line: 0,
            end_line: 0,
            start_token: 0,
            end_token: 0,
            matcher: String::new(),
            score: 0.0,
            matched_length: 0,
            rule_length: 0,
            match_coverage: 0.0,
            rule_relevance: 0,
            rule_identifier: String::new(),
            rule_url: String::new(),
            matched_text: None,
            referenced_filenames: None,
            is_license_intro: false,
            is_license_clue: false,
            is_license_reference: false,
            is_license_tag: false,
            is_license_text: false,
            is_from_license: false,
            matched_token_positions: None,
            hilen: 0,
            rule_start_token: 0,
            qspan_positions: None,
            ispan_positions: None,
            hispan_positions: None,
            candidate_resemblance: 0.0,
            candidate_containment: 0.0,
        }
    }
}

impl LicenseMatch {
    pub fn matcher_order(&self) -> u8 {
        match self.matcher.as_str() {
            "1-hash" => 0,
            "1-spdx-id" => 2,
            "2-aho" => 1,
            "3-seq" => 3,
            "3-spdx" => 3,
            "4-seq" => 4,
            "6-unknown" => 6,
            _ => 9,
        }
    }

    pub fn hilen(&self) -> usize {
        self.hilen
    }

    pub fn qstart(&self) -> usize {
        if let Some(positions) = &self.qspan_positions {
            positions.iter().copied().min().unwrap_or(self.start_token)
        } else {
            self.start_token
        }
    }

    pub fn is_small(
        &self,
        min_matched_len: usize,
        min_high_matched_len: usize,
        rule_is_small: bool,
    ) -> bool {
        if self.matched_length < min_matched_len || self.hilen() < min_high_matched_len {
            return true;
        }
        if rule_is_small && self.match_coverage < 80.0 {
            return true;
        }
        false
    }

    pub(crate) fn len(&self) -> usize {
        if let Some(positions) = &self.qspan_positions {
            positions.len()
        } else if let Some(positions) = &self.matched_token_positions {
            positions.len()
        } else {
            self.end_token.saturating_sub(self.start_token)
        }
    }

    fn qregion_len(&self) -> usize {
        if let Some(positions) = &self.qspan_positions {
            if positions.is_empty() {
                return 0;
            }
            let min_pos = *positions.iter().min().unwrap_or(&0);
            let max_pos = *positions.iter().max().unwrap_or(&0);
            max_pos - min_pos + 1
        } else if let Some(positions) = &self.matched_token_positions {
            if positions.is_empty() {
                return 0;
            }
            let min_pos = *positions.iter().min().unwrap_or(&0);
            let max_pos = *positions.iter().max().unwrap_or(&0);
            max_pos - min_pos + 1
        } else {
            self.end_token.saturating_sub(self.start_token)
        }
    }

    pub fn qmagnitude(&self, query: &crate::license_detection::query::Query) -> usize {
        let qregion_len = self.qregion_len();
        let positions: Vec<usize> = if let Some(qspan_positions) = &self.qspan_positions {
            qspan_positions.clone()
        } else {
            (self.start_token..self.end_token).collect()
        };
        if positions.is_empty() {
            return qregion_len;
        }
        let max_pos = *positions.iter().max().unwrap_or(&0);
        let unknowns_in_match: usize = positions
            .iter()
            .filter(|&&pos| pos != max_pos)
            .filter_map(|&pos| query.unknowns_by_pos.get(&Some(pos as i32)))
            .sum();
        qregion_len + unknowns_in_match
    }

    pub fn qdensity(&self, query: &crate::license_detection::query::Query) -> f32 {
        let mlen = self.len();
        if mlen == 0 {
            return 0.0;
        }
        let qmag = self.qmagnitude(query);
        if qmag == 0 {
            return 0.0;
        }
        mlen as f32 / qmag as f32
    }

    pub fn idensity(&self) -> f32 {
        let ispan_len = if let Some(positions) = &self.ispan_positions {
            positions.len()
        } else {
            self.matched_length
        };
        if ispan_len == 0 {
            return 0.0;
        }
        let ispan_magnitude = if let Some(positions) = &self.ispan_positions {
            if positions.is_empty() {
                return 0.0;
            }
            let min_pos = *positions.iter().min().unwrap();
            let max_pos = *positions.iter().max().unwrap();
            max_pos - min_pos + 1
        } else {
            self.matched_length
        };
        if ispan_magnitude == 0 {
            return 0.0;
        }
        ispan_len as f32 / ispan_magnitude as f32
    }

    pub fn icoverage(&self) -> f32 {
        if self.rule_length == 0 {
            return 0.0;
        }
        self.len() as f32 / self.rule_length as f32
    }

    pub fn surround(&self, other: &LicenseMatch) -> bool {
        let (self_qstart, self_qend) = self.qspan_bounds();
        let (other_qstart, other_qend) = other.qspan_bounds();
        self_qstart <= other_qstart && self_qend >= other_qend
    }

    pub fn qcontains(&self, other: &LicenseMatch) -> bool {
        if let (Some(self_positions), Some(other_positions)) =
            (&self.qspan_positions, &other.qspan_positions)
        {
            let self_set: HashSet<usize> = self_positions.iter().copied().collect();
            return other_positions.iter().all(|p| self_set.contains(p));
        }

        if let (Some(self_positions), None) = (&self.qspan_positions, &other.qspan_positions) {
            let self_set: HashSet<usize> = self_positions.iter().copied().collect();
            return (other.start_token..other.end_token).all(|p| self_set.contains(&p));
        }

        if let (None, Some(other_positions)) = (&self.qspan_positions, &other.qspan_positions) {
            return other_positions
                .iter()
                .all(|&p| p >= self.start_token && p < self.end_token);
        }

        if self.start_token == 0
            && self.end_token == 0
            && other.start_token == 0
            && other.end_token == 0
        {
            return self.start_line <= other.start_line && self.end_line >= other.end_line;
        }
        self.start_token <= other.start_token && self.end_token >= other.end_token
    }

    pub fn qoverlap(&self, other: &LicenseMatch) -> usize {
        if let (Some(self_positions), Some(other_positions)) =
            (&self.qspan_positions, &other.qspan_positions)
        {
            let self_set: HashSet<usize> = self_positions.iter().copied().collect();
            return other_positions
                .iter()
                .filter(|p| self_set.contains(p))
                .count();
        }

        if let (Some(self_positions), None) = (&self.qspan_positions, &other.qspan_positions) {
            let self_set: HashSet<usize> = self_positions.iter().copied().collect();
            return (other.start_token..other.end_token)
                .filter(|p| self_set.contains(p))
                .count();
        }

        if let (None, Some(other_positions)) = (&self.qspan_positions, &other.qspan_positions) {
            return other_positions
                .iter()
                .filter(|&&p| p >= self.start_token && p < self.end_token)
                .count();
        }

        if self.start_token == 0
            && self.end_token == 0
            && other.start_token == 0
            && other.end_token == 0
        {
            let start = self.start_line.max(other.start_line);
            let end = self.end_line.min(other.end_line);
            return if start <= end { end - start + 1 } else { 0 };
        }
        let start = self.start_token.max(other.start_token);
        let end = self.end_token.min(other.end_token);
        end.saturating_sub(start)
    }

    pub fn qspan_overlap(&self, other: &LicenseMatch) -> usize {
        let self_qspan: HashSet<usize> = self.qspan().into_iter().collect();
        let other_qspan: HashSet<usize> = other.qspan().into_iter().collect();
        self_qspan.intersection(&other_qspan).count()
    }

    /// Return true if all matched tokens are continuous without gaps or unknowns.
    /// Python: len() == qregion_len() == qmagnitude()
    pub fn is_continuous(&self, query: &crate::license_detection::query::Query) -> bool {
        if self.matched_token_positions.is_some() {
            return false;
        }
        let len = self.len();
        let qregion_len = self.qregion_len();
        let qmagnitude = self.qmagnitude(query);
        len == qregion_len && qregion_len == qmagnitude
    }

    pub fn ispan(&self) -> Vec<usize> {
        if let Some(positions) = &self.ispan_positions {
            positions.clone()
        } else {
            (self.rule_start_token..self.rule_start_token + self.matched_length).collect()
        }
    }

    pub fn hispan(&self) -> Vec<usize> {
        if let Some(positions) = &self.hispan_positions {
            positions.clone()
        } else {
            (self.rule_start_token..self.rule_start_token + self.hilen).collect()
        }
    }

    pub fn qspan(&self) -> Vec<usize> {
        if let Some(positions) = &self.qspan_positions {
            positions.clone()
        } else {
            (self.start_token..self.end_token).collect()
        }
    }

    pub fn qspan_eq(&self, other: &LicenseMatch) -> bool {
        match (&self.qspan_positions, &other.qspan_positions) {
            (Some(self_positions), Some(other_positions)) => {
                self_positions.len() == other_positions.len()
                    && self_positions.iter().collect::<HashSet<_>>()
                        == other_positions.iter().collect::<HashSet<_>>()
            }
            (Some(self_positions), None) => {
                let range_len = other.end_token.saturating_sub(other.start_token);
                self_positions.len() == range_len
                    && self_positions
                        .iter()
                        .all(|&p| p >= other.start_token && p < other.end_token)
            }
            (None, Some(other_positions)) => {
                let range_len = self.end_token.saturating_sub(self.start_token);
                other_positions.len() == range_len
                    && other_positions
                        .iter()
                        .all(|&p| p >= self.start_token && p < self.end_token)
            }
            (None, None) => {
                if self.start_token == 0
                    && self.end_token == 0
                    && other.start_token == 0
                    && other.end_token == 0
                {
                    self.start_line == other.start_line && self.end_line == other.end_line
                } else {
                    self.start_token == other.start_token && self.end_token == other.end_token
                }
            }
        }
    }

    pub fn qdistance_to(&self, other: &LicenseMatch) -> usize {
        if self.qoverlap(other) > 0 {
            return 0;
        }

        let (self_start, self_end_exclusive) = self.qspan_bounds();
        let (other_start, other_end_exclusive) = other.qspan_bounds();
        let self_end = self_end_exclusive.saturating_sub(1);
        let other_end = other_end_exclusive.saturating_sub(1);

        if self_end + 1 == other_start || other_end + 1 == self_start {
            return 1;
        }

        if self_end < other_start {
            other_start.saturating_sub(self_end)
        } else {
            self_start.saturating_sub(other_end)
        }
    }

    pub fn qspan_bounds(&self) -> (usize, usize) {
        if let Some(positions) = &self.qspan_positions {
            if positions.is_empty() {
                return (0, 0);
            }
            (
                *positions.iter().min().unwrap(),
                *positions.iter().max().unwrap() + 1,
            )
        } else {
            (self.start_token, self.end_token)
        }
    }

    pub fn qspan_magnitude(&self) -> usize {
        let (start, end) = self.qspan_bounds();
        end.saturating_sub(start)
    }

    pub fn ispan_bounds(&self) -> (usize, usize) {
        if let Some(positions) = &self.ispan_positions {
            if positions.is_empty() {
                return (0, 0);
            }
            (
                *positions.iter().min().unwrap(),
                *positions.iter().max().unwrap() + 1,
            )
        } else {
            (
                self.rule_start_token,
                self.rule_start_token + self.matched_length,
            )
        }
    }

    pub fn idistance_to(&self, other: &LicenseMatch) -> usize {
        let (self_start, self_end) = self.ispan_bounds();
        let (other_start, other_end) = other.ispan_bounds();

        if self_start < other_end && other_start < self_end {
            return 0;
        }

        if self_end == other_start || other_end == self_start {
            return 1;
        }

        if self_end <= other_start {
            other_start - self_end
        } else {
            self_start - other_end
        }
    }

    pub fn is_after(&self, other: &LicenseMatch) -> bool {
        let (self_qstart, _self_qend) = self.qspan_bounds();
        let (_other_qstart, other_qend) = other.qspan_bounds();

        let q_after = self_qstart >= other_qend;

        let (self_istart, _self_iend) = self.ispan_bounds();
        let (_other_istart, other_iend) = other.ispan_bounds();

        let i_after = self_istart >= other_iend;

        q_after && i_after
    }

    pub fn ispan_overlap(&self, other: &LicenseMatch) -> usize {
        if let (Some(self_positions), Some(other_positions)) =
            (&self.ispan_positions, &other.ispan_positions)
        {
            let self_set: HashSet<usize> = self_positions.iter().copied().collect();
            return other_positions
                .iter()
                .filter(|p| self_set.contains(p))
                .count();
        }

        if let (Some(self_positions), None) = (&self.ispan_positions, &other.ispan_positions) {
            let self_set: HashSet<usize> = self_positions.iter().copied().collect();
            return (other.rule_start_token..other.rule_start_token + other.matched_length)
                .filter(|p| self_set.contains(p))
                .count();
        }

        if let (None, Some(other_positions)) = (&self.ispan_positions, &other.ispan_positions) {
            return other_positions
                .iter()
                .filter(|&&p| {
                    p >= self.rule_start_token && p < self.rule_start_token + self.matched_length
                })
                .count();
        }

        let (self_start, self_end) = self.ispan_bounds();
        let (other_start, other_end) = other.ispan_bounds();

        let overlap_start = self_start.max(other_start);
        let overlap_end = self_end.min(other_end);

        overlap_end.saturating_sub(overlap_start)
    }

    pub fn has_unknown(&self) -> bool {
        self.license_expression.contains("unknown")
    }
}
