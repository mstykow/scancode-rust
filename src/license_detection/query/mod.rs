//! Query processing - tokenized input for license matching.

use crate::license_detection::index::LicenseIndex;
use crate::license_detection::tokenize::{STOPWORDS, tokenize_without_stopwords};
use std::collections::{HashMap, HashSet};

/// A span representing a range of token positions.
///
/// Used for tracking matched token positions and performing position arithmetic.
/// This is a single continuous range of token positions (start..=end, inclusive).
///
/// Distinct from `spans::Span` which tracks multiple byte ranges for coverage.
///
/// Based on Python Span class at:
/// reference/scancode-toolkit/src/licensedcode/spans.py
#[derive(Debug, Clone)]
pub struct PositionSpan {
    start: usize,
    end: usize,
}

impl PositionSpan {
    /// Create a new span from start and end positions (inclusive).
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Check if this span contains a position.
    pub fn contains(&self, pos: usize) -> bool {
        pos >= self.start && pos <= self.end
    }

    /// Get all positions in this span as a HashSet.
    pub fn positions(&self) -> HashSet<usize> {
        (self.start..=self.end).collect()
    }

    /// Subtract another span from this span.
    ///
    /// Returns positions in self that are not in other.
    pub fn difference(&self, other: &PositionSpan) -> HashSet<usize> {
        self.positions()
            .difference(&other.positions())
            .copied()
            .collect()
    }
}

///
/// Query holds:
/// - Known token IDs (tokens existing in the index dictionary)
/// - Token positions and their corresponding line numbers (line_by_pos)
/// - Unknown tokens (tokens not in dictionary) tracked per position
/// - Stopwords tracked per position
/// - Positions with short/digit-only tokens
/// - High and low matchable token positions (for tracking what's been matched)
///
/// Based on Python Query class at:
/// reference/scancode-toolkit/src/licensedcode/query.py (lines 155-295)
#[derive(Debug)]
pub struct Query<'a> {
    /// The original input text.
    ///
    /// Corresponds to Python: `self.query_string` (line 215)
    pub text: String,

    /// Token IDs for known tokens (tokens found in the index dictionary)
    ///
    /// Corresponds to Python: `self.tokens = []` (line 228)
    pub tokens: Vec<u16>,

    /// Mapping from token position to line number (1-based)
    ///
    /// Each token position in `self.tokens` maps to the line number where it appears.
    /// This is used for match position reporting.
    ///
    /// Corresponds to Python: `self.line_by_pos = []` (line 231)
    pub line_by_pos: Vec<usize>,

    /// Mapping from token position to count of unknown tokens after that position
    ///
    /// Unknown tokens are those not found in the dictionary. We track them by
    /// counting how many unknown tokens appear after each known position.
    /// Unknown tokens before the first known token are tracked at position -1
    /// (using the key `None` in Rust).
    ///
    /// Corresponds to Python: `self.unknowns_by_pos = {}` (line 236)
    pub unknowns_by_pos: HashMap<Option<i32>, usize>,

    /// Mapping from token position to count of stopwords after that position
    ///
    /// Similar to unknown_tokens, but for stopwords.
    ///
    /// Corresponds to Python: `self.stopwords_by_pos = {}` (line 244)
    pub stopwords_by_pos: HashMap<Option<i32>, usize>,

    /// Set of positions with single-character or digit-only tokens
    ///
    /// These tokens have special handling in matching.
    ///
    /// Corresponds to Python: `self.shorts_and_digits_pos = set()` (line 249)
    pub shorts_and_digits_pos: HashSet<usize>,

    /// High-value matchable token positions (legalese tokens)
    ///
    /// These are tokens with ID < len_legalese.
    ///
    /// Corresponds to Python: `self.high_matchables` (line 293)
    pub high_matchables: HashSet<usize>,

    /// Low-value matchable token positions (non-legalese tokens)
    ///
    /// These are tokens with ID >= len_legalese.
    ///
    /// Corresponds to Python: `self.low_matchables` (line 294)
    pub low_matchables: HashSet<usize>,

    /// True if the query contains very long lines (e.g., minified JS/CSS)
    ///
    /// Corresponds to Python: `self.has_long_lines = False` (line 222)
    pub has_long_lines: bool,

    /// True if the query is detected as binary content
    ///
    /// Corresponds to Python: `self.is_binary = False` (line 225)
    pub is_binary: bool,

    /// Raw query run ranges (start, end) computed during tokenization.
    ///
    /// QueryRuns are created on-demand from these ranges.
    ///
    /// Corresponds to Python: `self.query_runs = []` (line 274)
    pub(crate) query_run_ranges: Vec<(usize, Option<usize>)>,

    /// SPDX-License-Identifier lines found during tokenization.
    ///
    /// Each tuple is (spdx_text, start_token_pos, end_token_pos).
    /// Used for creating LicenseMatches with correct token positions.
    ///
    /// Corresponds to Python: `self.spdx_lines = []` (line 507)
    pub spdx_lines: Vec<(String, usize, usize)>,

    /// Reference to the license index for dictionary access and metadata
    pub index: &'a LicenseIndex,
}

impl<'a> Query<'a> {
    /// Create a new query from text string and license index.
    ///
    /// This tokenizes the input text, looks up each token in the index dictionary,
    /// and builds the query structures for matching.
    ///
    /// # Arguments
    /// * `text` - The input text to tokenize
    /// * `index` - The license index containing the token dictionary
    ///
    /// # Returns
    /// A Result containing the Query or an error if binary detection fails
    ///
    /// Corresponds to Python: `Query.__init__()` (lines 196-295)
    const TEXT_LINE_THRESHOLD: usize = 15;

    pub fn new(text: &str, index: &'a LicenseIndex) -> Result<Self, anyhow::Error> {
        Self::with_options(text, index, Self::TEXT_LINE_THRESHOLD)
    }

    /// Iterate over query runs.
    ///
    /// If query runs is empty (not yet computed), returns a single run
    /// covering the whole query.
    ///
    /// Corresponds to Python: `query.query_runs` property iteration
    pub fn query_runs(&self) -> Vec<QueryRun<'_>> {
        if self.query_run_ranges.is_empty() {
            vec![self.whole_query_run()]
        } else {
            self.query_run_ranges
                .iter()
                .map(|&(start, end)| QueryRun::new(self, start, end))
                .collect()
        }
    }

    /// Create a new query with custom line threshold.
    ///
    /// # Arguments
    /// * `text` - The input text to tokenize
    /// * `index` - The license index containing the token dictionary
    /// * `_line_threshold` - Number of empty/junk lines to break a new run (default 4)
    ///
    /// # Returns
    /// A Result containing the Query or an error if binary detection fails
    ///
    /// Corresponds to Python: `Query.__init__()` with line_threshold parameter
    pub fn with_options(
        text: &str,
        index: &'a LicenseIndex,
        _line_threshold: usize,
    ) -> Result<Self, anyhow::Error> {
        let is_binary = Self::detect_binary(text)?;
        let has_long_lines = Self::detect_long_lines(text);

        let stopwords_set = &*STOPWORDS;

        let mut tokens = Vec::new();
        let mut line_by_pos = Vec::new();
        let mut unknowns_by_pos: HashMap<Option<i32>, usize> = HashMap::new();
        let mut stopwords_by_pos: HashMap<Option<i32>, usize> = HashMap::new();
        let mut shorts_and_digits_pos = HashSet::new();
        let mut spdx_lines: Vec<(String, usize, usize)> = Vec::new();

        let len_legalese = index.len_legalese;

        let mut known_pos = -1i32;
        let mut started = false;
        let mut current_line = 1usize;

        let mut tokens_by_line: Vec<Vec<u16>> = Vec::new();

        for line in text.lines() {
            let line_trimmed = line.trim();
            let mut line_tokens: Vec<u16> = Vec::new();

            let line_first_known_pos = known_pos + 1;

            for token in tokenize_without_stopwords(line_trimmed) {
                let is_stopword = stopwords_set.contains(token.as_str());
                let tid_opt = index.dictionary.get(&token);

                if !is_stopword {
                    if let Some(tid) = tid_opt {
                        known_pos += 1;
                        started = true;
                        tokens.push(tid);
                        line_by_pos.push(current_line);
                        line_tokens.push(tid);

                        if token.len() == 1 || token.chars().all(|c| c.is_ascii_digit()) {
                            let _ = shorts_and_digits_pos.insert(known_pos as usize);
                        }
                    } else if !started {
                        *unknowns_by_pos.entry(None).or_insert(0) += 1;
                    } else {
                        *unknowns_by_pos.entry(Some(known_pos)).or_insert(0) += 1;
                    }
                } else if !started {
                    *stopwords_by_pos.entry(None).or_insert(0) += 1;
                } else {
                    *stopwords_by_pos.entry(Some(known_pos)).or_insert(0) += 1;
                }
            }

            let line_last_known_pos = known_pos;

            let tokens_lower: Vec<String> = tokenize_without_stopwords(line_trimmed)
                .into_iter()
                .map(|t| t.to_lowercase())
                .collect();

            let spdx_start_offset = if tokens_lower.len() >= 3 {
                let first_three: Vec<&str> =
                    tokens_lower.iter().take(3).map(|s| s.as_str()).collect();
                let is_spdx_prefix = first_three == ["spdx", "license", "identifier"]
                    || first_three == ["spdx", "licence", "identifier"]
                    || first_three == ["licenses", "nuget", "org"];
                if is_spdx_prefix {
                    Some(0)
                } else if tokens_lower.len() >= 4 {
                    let second_three: Vec<&str> = tokens_lower
                        .iter()
                        .skip(1)
                        .take(3)
                        .map(|s| s.as_str())
                        .collect();
                    let is_spdx_second = second_three == ["spdx", "license", "identifier"]
                        || second_three == ["spdx", "licence", "identifier"]
                        || second_three == ["licenses", "nuget", "org"];
                    if is_spdx_second {
                        Some(1)
                    } else if tokens_lower.len() >= 5 {
                        let third_three: Vec<&str> = tokens_lower
                            .iter()
                            .skip(2)
                            .take(3)
                            .map(|s| s.as_str())
                            .collect();
                        let is_spdx_third = third_three == ["spdx", "license", "identifier"]
                            || third_three == ["spdx", "licence", "identifier"]
                            || third_three == ["licenses", "nuget", "org"];
                        if is_spdx_third { Some(2) } else { None }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(offset) = spdx_start_offset {
                let spdx_start_known_pos = line_first_known_pos + offset;
                if spdx_start_known_pos <= line_last_known_pos {
                    let spdx_start = spdx_start_known_pos as usize;
                    let spdx_end = (line_last_known_pos + 1) as usize;
                    spdx_lines.push((line_trimmed.to_string(), spdx_start, spdx_end));
                }
            }

            tokens_by_line.push(line_tokens);
            current_line += 1;
        }

        let high_matchables: HashSet<usize> = tokens
            .iter()
            .enumerate()
            .filter(|(_pos, tid)| (**tid as usize) < len_legalese)
            .map(|(pos, _tid)| pos)
            .collect();

        let low_matchables: HashSet<usize> = tokens
            .iter()
            .enumerate()
            .filter(|(_pos, tid)| (**tid as usize) >= len_legalese)
            .map(|(pos, _tid)| pos)
            .collect();

        // Compute query runs by splitting on 4+ consecutive junk lines.
        // Double-matching prevention: is_matchable() with matched_qspans exclusion
        // in Phase 4 (mod.rs:286) handles this. query.subtract() is called after
        // near-dupe matches (mod.rs:252) to update high/low_matchables.
        //
        // TODO: QueryRun causes 37 regressions due to issues with multi-license
        // detection in files with large junk sections. The candidate selection
        // fix (preferring higher coverage) is correct and has been kept.
        // QueryRun needs further investigation before re-enabling.
        //
        // let query_runs = Self::compute_query_runs(
        //     &tokens,
        //     &tokens_by_line,
        //     _line_threshold,
        //     len_legalese,
        //     &index.digit_only_tids,
        // );
        let query_runs: Vec<(usize, Option<usize>)> = Vec::new();

        Ok(Query {
            text: text.to_string(),
            tokens,
            line_by_pos,
            unknowns_by_pos,
            stopwords_by_pos,
            shorts_and_digits_pos,
            high_matchables,
            low_matchables,
            has_long_lines,
            is_binary,
            query_run_ranges: query_runs,
            spdx_lines,
            index,
        })
    }

    /// Compute query runs by analyzing line-by-line tokenization.
    ///
    /// Breaks the query into runs when we encounter `line_threshold` consecutive
    /// "junk" lines. A junk line is one that:
    /// - Is empty (no known tokens)
    /// - Contains only unknown tokens
    /// - Contains only digit-only tokens
    /// - Contains no high-value legalese tokens
    ///
    /// Based on Python: `Query._tokenize_and_build_runs()` at lines 568-641
    fn compute_query_runs(
        tokens: &[u16],
        tokens_by_line: &[Vec<u16>],
        line_threshold: usize,
        len_legalese: usize,
        digit_only_tids: &HashSet<u16>,
    ) -> Vec<(usize, Option<usize>)> {
        if tokens.is_empty() {
            return Vec::new();
        }

        let mut query_runs: Vec<(usize, Option<usize>)> = Vec::new();
        let mut empty_lines = 0usize;
        let mut pos = 0usize;
        let mut run_start = 0usize;
        let mut run_end: Option<usize> = None;

        for line_tokens in tokens_by_line {
            if run_end.is_some() && empty_lines >= line_threshold {
                query_runs.push((run_start, run_end));
                run_start = pos;
                empty_lines = 0;
            }

            if line_tokens.is_empty() {
                empty_lines += 1;
                continue;
            }

            let line_is_all_digit = line_tokens.iter().all(|tid| digit_only_tids.contains(tid));
            let line_has_good_tokens = line_tokens.iter().any(|tid| (*tid as usize) < len_legalese);

            for _tid in line_tokens {
                run_end = Some(pos);
                pos += 1;
            }

            if line_is_all_digit {
                empty_lines += 1;
                continue;
            }

            if line_has_good_tokens {
                empty_lines = 0;
            } else {
                empty_lines += 1;
            }
        }

        if let Some(end) = run_end {
            let run_all_digits = tokens[run_start..=end]
                .iter()
                .all(|tid| digit_only_tids.contains(tid));
            if !run_all_digits {
                query_runs.push((run_start, run_end));
            }
        }

        query_runs
    }

    /// Detect if text is binary content.
    ///
    /// Binary detection checks for:
    /// - Null bytes (0x00)
    /// - High ratio of non-printable characters
    ///
    /// # Arguments
    /// * `text` - The text to analyze
    ///
    /// # Returns
    /// true if binary, false otherwise
    ///
    /// Corresponds to Python: `typecode.get_type().is_binary` usage (lines 123-135)
    fn detect_binary(text: &str) -> Result<bool, anyhow::Error> {
        let null_byte_count = text.bytes().filter(|&b| b == 0).count();

        if null_byte_count > 0 {
            return Ok(true);
        }

        let non_printable_ratio = text
            .chars()
            .filter(|&c| {
                !c.is_ascii() && !c.is_ascii_graphic() && c != '\n' && c != '\r' && c != '\t'
            })
            .count() as f64
            / text.len().max(1) as f64;

        Ok(non_printable_ratio > 0.3)
    }

    /// Detect if text has very long lines (for minified JS/CSS).
    ///
    /// # Arguments
    /// * `text` - The text to analyze
    ///
    /// # Returns
    /// true if there are lines with many tokens, false otherwise
    ///
    /// Corresponds to Python: `typecode.get_type().is_text_with_long_lines` usage
    fn detect_long_lines(text: &str) -> bool {
        text.lines()
            .any(|line| tokenize_without_stopwords(line).len() > 25)
    }

    /// Get the length of the query in tokens.
    ///
    /// # Arguments
    /// * `with_unknown` - If true, include unknown tokens in the count
    ///
    /// # Returns
    /// The number of tokens
    ///
    /// Corresponds to Python: `tokens_length()` method (lines 296-304)
    pub fn tokens_length(&self, with_unknown: bool) -> usize {
        let length = self.tokens.len();
        if with_unknown {
            length + self.unknowns_by_pos.values().sum::<usize>()
        } else {
            length
        }
    }

    /// Check if a token position has a short or digit-only token.
    ///
    /// # Arguments
    /// * `pos` - The token position
    ///
    /// # Returns
    /// true if the position has a short or digit-only token
    #[inline]
    pub fn is_short_or_digit(&self, pos: usize) -> bool {
        self.shorts_and_digits_pos.contains(&pos)
    }

    /// Get the number of unknown tokens after a given position.
    ///
    /// # Arguments
    /// * `pos` - The token position (None for before first token)
    ///
    /// # Returns
    /// The count of unknown tokens
    #[inline]
    pub fn unknown_count_after(&self, pos: Option<i32>) -> usize {
        self.unknowns_by_pos.get(&pos).copied().unwrap_or(0)
    }

    /// Get the number of stopwords after a given position.
    ///
    /// # Arguments
    /// * `pos` - The token position (None for before first token)
    ///
    /// # Returns
    /// The count of stopwords
    #[inline]
    pub fn stopword_count_after(&self, pos: Option<i32>) -> usize {
        self.stopwords_by_pos.get(&pos).copied().unwrap_or(0)
    }

    /// Get the line number for a token position.
    ///
    /// # Arguments
    /// * `pos` - The token position
    ///
    /// # Returns
    /// The line number (1-based)
    #[inline]
    pub fn line_for_pos(&self, pos: usize) -> Option<usize> {
        self.line_by_pos.get(pos).copied()
    }

    /// Get the token ID at a position.
    ///
    /// # Arguments
    /// * `pos` - The token position
    ///
    /// # Returns
    /// The token ID if position is valid
    #[inline]
    pub fn token_at(&self, pos: usize) -> Option<u16> {
        self.tokens.get(pos).copied()
    }

    /// Check if the query is empty (no known tokens).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Get the number of known tokens.
    #[inline]
    pub fn len(&self) -> usize {
        self.tokens.len()
    }
    /// Get a query run covering the entire query.
    ///
    /// Corresponds to Python: `whole_query_run()` method (lines 306-317)
    pub fn whole_query_run(&self) -> QueryRun<'_> {
        if self.is_empty() {
            return QueryRun::new(self, 0, None);
        }
        QueryRun::new(self, 0, Some(self.tokens.len() - 1))
    }

    /// Get all matchable token positions.
    ///
    /// Corresponds to Python: `matchables` property (lines 336-341)
    pub fn matchables(&self) -> HashSet<usize> {
        self.low_matchables
            .union(&self.high_matchables)
            .copied()
            .collect()
    }

    /// Get matched token positions (tokens that are not matchable).
    ///
    /// Corresponds to Python: `matched` property (lines 343-350)
    pub fn matched(&self) -> HashSet<usize> {
        let all_positions: HashSet<usize> = (0..self.tokens.len()).collect();
        all_positions
            .difference(&self.matchables())
            .copied()
            .collect()
    }

    /// Check if a position is a high-value legalese token.
    ///
    /// # Arguments
    /// * `pos` - The token position to check
    #[inline]
    pub fn is_high_matchable(&self, pos: usize) -> bool {
        self.high_matchables.contains(&pos)
    }

    /// Check if a position is a low-value token.
    ///
    /// # Arguments
    /// * `pos` - The token position to check
    #[inline]
    pub fn is_low_matchable(&self, pos: usize) -> bool {
        self.low_matchables.contains(&pos)
    }

    /// Get high-value matchable positions within a range.
    ///
    /// # Arguments
    /// * `start` - Start position (inclusive)
    /// * `end` - End position (inclusive, or None for unbounded)
    ///
    /// Corresponds to Python: `high_matchables` property in QueryRun (lines 851-861)
    pub fn high_matchables(&self, start: &usize, end: &Option<usize>) -> Option<HashSet<usize>> {
        if *start >= self.tokens.len() {
            return None;
        }

        let end_pos = end
            .unwrap_or(self.tokens.len() - 1)
            .min(self.tokens.len() - 1);

        Some(
            self.high_matchables
                .iter()
                .filter(|&&pos| pos >= *start && pos <= end_pos)
                .copied()
                .collect(),
        )
    }

    /// Get low-value matchable positions within a range.
    ///
    /// # Arguments
    /// * `start` - Start position (inclusive)
    /// * `end` - End position (inclusive, or None for unbounded)
    ///
    /// Corresponds to Python: `low_matchables` property in QueryRun (lines 839-849)
    pub fn low_matchables(&self, start: &usize, end: &Option<usize>) -> Option<HashSet<usize>> {
        if *start >= self.tokens.len() {
            return None;
        }

        let end_pos = end
            .unwrap_or(self.tokens.len() - 1)
            .min(self.tokens.len() - 1);

        Some(
            self.low_matchables
                .iter()
                .filter(|&&pos| pos >= *start && pos <= end_pos)
                .copied()
                .collect(),
        )
    }

    /// Subtract matched positions from matchables.
    ///
    /// This removes the positions from both high and low matchables.
    ///
    /// # Arguments
    /// * `span` - The span of positions to subtract
    ///
    /// Corresponds to Python: `subtract()` method (lines 328-334)
    pub fn subtract(&mut self, span: &PositionSpan) {
        let positions = span.positions();
        self.high_matchables = self
            .high_matchables
            .difference(&positions)
            .copied()
            .collect();
        self.low_matchables = self
            .low_matchables
            .difference(&positions)
            .copied()
            .collect();
    }

    /// Extract matched text for a given line range.
    ///
    /// Returns the text from the original input between start_line and end_line
    /// (both inclusive, 1-indexed).
    ///
    /// # Arguments
    /// * `start_line` - Starting line number (1-indexed)
    /// * `end_line` - Ending line number (1-indexed)
    ///
    /// # Returns
    /// The matched text, or empty string if lines are out of range
    ///
    /// Corresponds to Python: `matched_text()` method in match.py (lines 757-795)
    pub fn matched_text(&self, start_line: usize, end_line: usize) -> String {
        if start_line == 0 || end_line == 0 || start_line > end_line {
            return String::new();
        }

        self.text
            .lines()
            .enumerate()
            .filter_map(|(idx, line)| {
                let line_num = idx + 1;
                if line_num >= start_line && line_num <= end_line {
                    Some(line)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// A query run is a slice of query tokens identified by a start and end positions.
///
/// Query runs break a query into manageable chunks for efficient matching.
/// They track matchable token positions and support subtraction of matched spans.
///
/// Based on Python QueryRun class at:
/// reference/scancode-toolkit/src/licensedcode/query.py (lines 720-914)
#[derive(Debug, Clone)]
pub struct QueryRun<'a> {
    query: &'a Query<'a>,
    pub start: usize,
    pub end: Option<usize>,
}

impl<'a> QueryRun<'a> {
    /// Create a new query run from a query with start and end positions.
    ///
    /// # Arguments
    /// * `query` - The parent query
    /// * `start` - The start position (inclusive)
    /// * `end` - The end position (inclusive), or None for an empty run
    ///
    /// Corresponds to Python: `QueryRun.__init__()` (lines 735-749)
    pub fn new(query: &'a Query<'a>, start: usize, end: Option<usize>) -> Self {
        Self { query, start, end }
    }

    /// Get the license index used by this query run.
    pub fn get_index(&self) -> &LicenseIndex {
        self.query.index
    }

    /// Get the underlying query reference.
    pub fn query(&self) -> &Query<'a> {
        self.query
    }

    /// Get the start line number of this query run.
    ///
    /// Corresponds to Python: `start_line` property (lines 771-773)
    pub fn start_line(&self) -> Option<usize> {
        self.query.line_by_pos.get(self.start).copied()
    }

    /// Get the end line number of this query run.
    ///
    /// Corresponds to Python: `end_line` property (lines 775-777)
    pub fn end_line(&self) -> Option<usize> {
        self.end
            .and_then(|e| self.query.line_by_pos.get(e).copied())
    }

    /// Get the line number for a specific token position.
    ///
    /// # Arguments
    /// * `pos` - Absolute token position in the query
    ///
    /// # Returns
    /// The line number (1-based), or None if position is out of range
    pub fn line_for_pos(&self, pos: usize) -> Option<usize> {
        self.query.line_by_pos.get(pos).copied()
    }

    /// Get the sequence of token IDs for this run.
    ///
    /// Returns empty slice if end is None.
    ///
    /// Corresponds to Python: `tokens` property (lines 779-786)
    pub fn tokens(&self) -> &[u16] {
        match self.end {
            Some(end) => &self.query.tokens[self.start..=end],
            None => &[],
        }
    }

    /// Iterate over token IDs with their absolute positions.
    ///
    /// Corresponds to Python: `tokens_with_pos()` method (lines 788-789)
    pub fn tokens_with_pos(&self) -> impl Iterator<Item = (usize, u16)> + '_ {
        self.tokens()
            .iter()
            .copied()
            .enumerate()
            .map(|(i, tid)| (self.start + i, tid))
    }

    /// Check if this query run contains only digit tokens.
    ///
    /// Corresponds to Python: `is_digits_only()` method (lines 791-796)
    pub fn is_digits_only(&self) -> bool {
        self.tokens()
            .iter()
            .all(|tid| self.query.index.digit_only_tids.contains(tid))
    }

    /// Check if this query run has matchable tokens.
    ///
    /// # Arguments
    /// * `include_low` - If true, include low-value tokens in the check
    /// * `exclude_positions` - Optional set of spans containing positions to exclude
    ///
    /// Returns true if there are matchable tokens remaining
    ///
    /// Corresponds to Python: `is_matchable()` method (lines 798-818)
    pub fn is_matchable(&self, include_low: bool, exclude_positions: &[PositionSpan]) -> bool {
        if self.is_digits_only() {
            return false;
        }

        let matchables = self.matchables(include_low);

        if exclude_positions.is_empty() {
            return !matchables.is_empty();
        }

        let mut matchable_set = matchables;
        for span in exclude_positions {
            let span_positions = span.positions();
            matchable_set = matchable_set.difference(&span_positions).copied().collect();
        }

        !matchable_set.is_empty()
    }

    /// Get all matchable token positions for this query run.
    ///
    /// # Arguments
    /// * `include_low` - If true, include low-value tokens
    ///
    /// Corresponds to Python: `matchables` property (lines 820-825)
    pub fn matchables(&self, include_low: bool) -> HashSet<usize> {
        if include_low {
            self.low_matchables()
                .union(&self.high_matchables())
                .copied()
                .collect()
        } else {
            self.high_matchables()
        }
    }

    /// Get an iterator over matchable tokens.
    ///
    /// Returns -1 for positions with non-matchable tokens.
    /// Returns empty if there are no high matchable tokens.
    ///
    /// Corresponds to Python: `matchable_tokens()` method (lines 827-837)
    pub fn matchable_tokens(&self) -> Vec<i32> {
        let high_matchables = self.high_matchables();
        if high_matchables.is_empty() {
            return Vec::new();
        }

        // Use ALL matchables (high + low), not just high matchables.
        // This is critical for Phase 2 (near-duplicate detection) to find
        // combined rules like "cddl-1.0 OR gpl-2.0-glassfish".
        // Python: `self.matchables` includes both high and low.
        let matchables = self.matchables(true);
        self.tokens_with_pos()
            .map(|(pos, tid)| {
                if matchables.contains(&pos) {
                    tid as i32
                } else {
                    -1
                }
            })
            .collect()
    }

    /// Get high-value matchable token positions.
    ///
    /// High-value tokens are legalese (token ID < len_legalese).
    ///
    /// Corresponds to Python: `high_matchables` property (lines 851-861)
    pub fn high_matchables(&self) -> HashSet<usize> {
        self.query
            .high_matchables
            .iter()
            .filter(|&&pos| pos >= self.start && pos <= self.end.unwrap_or(usize::MAX))
            .copied()
            .collect()
    }

    /// Get low-value matchable token positions.
    ///
    /// Low-value tokens are non-legalese.
    ///
    /// Corresponds to Python: `low_matchables` property (lines 839-849)
    pub fn low_matchables(&self) -> HashSet<usize> {
        self.query
            .low_matchables
            .iter()
            .filter(|&&pos| pos >= self.start && pos <= self.end.unwrap_or(usize::MAX))
            .copied()
            .collect()
    }

    /// Extract matched text for a given line range.
    ///
    /// This delegates to the underlying Query's matched_text method.
    ///
    /// # Arguments
    /// * `start_line` - Starting line number (1-indexed)
    /// * `end_line` - Ending line number (1-indexed)
    ///
    /// # Returns
    /// The matched text, or empty string if lines are out of range
    pub fn matched_text(&self, start_line: usize, end_line: usize) -> String {
        self.query.matched_text(start_line, end_line)
    }
}

#[cfg(test)]
mod test;
