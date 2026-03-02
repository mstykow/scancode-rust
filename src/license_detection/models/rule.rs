//! Rule metadata loaded from .LICENSE and .RULE files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Range;

/// Rule metadata loaded from .LICENSE and .RULE files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// Unique identifier for this rule (e.g., "mit.LICENSE", "gpl-2.0_12.RULE")
    /// Used for sorting to match Python's attr.s field order.
    /// This is the primary sort key after rid (which is None at sort time in Python).
    pub identifier: String,

    /// License expression string using SPDX syntax and ScanCode license keys
    pub license_expression: String,

    /// Pattern text to match
    pub text: String,

    /// Token IDs for the text (assigned during indexing)
    pub tokens: Vec<u16>,

    /// True if this is a full license text (highest confidence)
    pub is_license_text: bool,

    /// True if this is an explicit notice like "Licensed under the MIT license"
    pub is_license_notice: bool,

    /// True if this is a reference like a bare name or URL
    pub is_license_reference: bool,

    /// True if this is a structured licensing tag (e.g., SPDX identifier in package manifest)
    pub is_license_tag: bool,

    /// True if this is an introductory statement before actual license text
    pub is_license_intro: bool,

    /// True if this is a clue but not a proper license detection
    pub is_license_clue: bool,

    /// True if exact matches to this rule are false positives
    pub is_false_positive: bool,

    /// True if this rule text is a required phrase.
    /// A required phrase is an essential section of the rule text which must be
    /// present in the case of partial matches.
    pub is_required_phrase: bool,

    /// True if this rule was created from a license file (not a .RULE file)
    pub is_from_license: bool,

    /// Relevance score 0-100 (100 is most relevant)
    pub relevance: u8,

    /// Minimum match coverage percentage (0-100) if specified
    pub minimum_coverage: Option<u8>,

    /// Tokens must appear in order if true
    pub is_continuous: bool,

    /// Token position spans for required phrases parsed from {{...}} markers.
    /// Each span represents positions in the rule text that MUST be matched.
    pub required_phrase_spans: Vec<Range<usize>>,

    /// Mapping from token position to count of stopwords at that position.
    /// Used for required phrase validation.
    pub stopwords_by_pos: HashMap<usize, usize>,

    /// Filenames where this rule should be considered
    pub referenced_filenames: Option<Vec<String>>,

    /// URLs that should be ignored when found in this rule text
    pub ignorable_urls: Option<Vec<String>>,

    /// Emails that should be ignored when found in this rule text
    pub ignorable_emails: Option<Vec<String>>,

    /// Copyrights that should be ignored when found in this rule text
    pub ignorable_copyrights: Option<Vec<String>>,

    /// Holder names that should be ignored when found in this rule text
    pub ignorable_holders: Option<Vec<String>>,

    /// Author names that should be ignored when found in this rule text
    pub ignorable_authors: Option<Vec<String>>,

    /// Programming language for the rule if specified
    pub language: Option<String>,

    /// Free text notes
    pub notes: Option<String>,

    /// Count of unique token IDs in the rule (computed during indexing)
    pub length_unique: usize,

    /// Count of unique legalese token IDs (tokens with ID < len_legalese)
    pub high_length_unique: usize,

    /// Total count of legalese token occurrences (with duplicates)
    pub high_length: usize,

    /// Minimum matched length threshold (occurrences-based)
    pub min_matched_length: usize,

    /// Minimum high-value token matched length threshold (occurrences-based)
    pub min_high_matched_length: usize,

    /// Minimum matched length threshold (unique tokens)
    pub min_matched_length_unique: usize,

    /// Minimum high-value token matched length threshold (unique tokens)
    pub min_high_matched_length_unique: usize,

    /// True if rule length < SMALL_RULE (15 tokens)
    pub is_small: bool,

    /// True if rule length < TINY_RULE (6 tokens)
    pub is_tiny: bool,

    /// True if the rule's first token is "license", "licence", or "licensed"
    pub starts_with_license: bool,

    /// True if the rule's last token is "license", "licence", or "licensed"
    pub ends_with_license: bool,

    /// Whether this rule is deprecated
    pub is_deprecated: bool,

    /// SPDX license identifier if available
    pub spdx_license_key: Option<String>,

    /// Alternative SPDX license identifiers (aliases)
    pub other_spdx_license_keys: Vec<String>,
}

impl PartialOrd for Rule {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rule {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.identifier.cmp(&other.identifier)
    }
}

