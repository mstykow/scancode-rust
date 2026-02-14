//! Core data structures for license detection.

use serde::{Deserialize, Serialize};

/// License metadata loaded from .LICENSE files.
#[derive(Debug, Clone, PartialEq)]
pub struct License {
    /// Unique lowercase ASCII identifier for this license
    pub key: String,

    /// Full name of the license
    pub name: String,

    /// SPDX license identifier if available
    pub spdx_license_key: Option<String>,

    /// License category (e.g., "Permissive", "Copyleft")
    pub category: Option<String>,

    /// Full license text
    pub text: String,

    /// Reference URLs for this license
    pub reference_urls: Vec<String>,

    /// Free text notes
    pub notes: Option<String>,

    /// Whether this license is deprecated
    pub is_deprecated: bool,

    /// List of license keys that replace this deprecated license
    pub replaced_by: Vec<String>,

    /// Minimum match coverage percentage (0-100) if specified
    pub minimum_coverage: Option<u8>,

    /// Copyrights that should be ignored when found in this license text
    pub ignorable_copyrights: Option<Vec<String>>,

    /// Holder names that should be ignored when found in this license text
    pub ignorable_holders: Option<Vec<String>>,

    /// Author names that should be ignored when found in this license text
    pub ignorable_authors: Option<Vec<String>>,

    /// URLs that should be ignored when found in this license text
    pub ignorable_urls: Option<Vec<String>>,

    /// Emails that should be ignored when found in this license text
    pub ignorable_emails: Option<Vec<String>>,
}

/// Rule metadata loaded from .LICENSE and .RULE files.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
}

/// License match result from a matching strategy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LicenseMatch {
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

    /// Name of the matching strategy used
    pub matcher: String,

    /// Match score 0.0-1.0
    pub score: f32,

    /// Length of matched text in characters
    pub matched_length: usize,

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_license_basic() {
        let license = License {
            key: "mit".to_string(),
            name: "MIT License".to_string(),
            spdx_license_key: Some("MIT".to_string()),
            category: Some("Permissive".to_string()),
            text: "MIT License text here...".to_string(),
            reference_urls: vec!["https://opensource.org/licenses/MIT".to_string()],
            notes: None,
            is_deprecated: false,
            replaced_by: vec![],
            minimum_coverage: None,
            ignorable_copyrights: None,
            ignorable_holders: None,
            ignorable_authors: None,
            ignorable_urls: None,
            ignorable_emails: None,
        };

        assert_eq!(license.key, "mit");
        assert_eq!(license.name, "MIT License");
    }

    #[test]
    fn test_rule_basic() {
        let rule = Rule {
            identifier: "mit.LICENSE".to_string(),
            license_expression: "mit".to_string(),
            text: "MIT License".to_string(),
            tokens: vec![],
            is_license_text: false,
            is_license_notice: true,
            is_license_reference: false,
            is_license_tag: false,
            is_license_intro: false,
            is_license_clue: false,
            is_false_positive: false,
            is_required_phrase: false,
            is_from_license: false,
            relevance: 90,
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

        assert_eq!(rule.license_expression, "mit");
        assert!(rule.is_license_notice);
        assert_eq!(rule.relevance, 90);
    }

    #[test]
    fn test_license_match_basic() {
        let match_result = LicenseMatch {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: Some("README.md".to_string()),
            start_line: 1,
            end_line: 5,
            matcher: "1-hash".to_string(),
            score: 0.95,
            matched_length: 100,
            match_coverage: 95.0,
            rule_relevance: 100,
            rule_identifier: "mit.LICENSE".to_string(),
            rule_url: "https://scancode-licensedb.aboutcode.org/mit".to_string(),
            matched_text: Some("MIT License text...".to_string()),
            referenced_filenames: None,
        };

        assert_eq!(match_result.license_expression, "mit");
        assert_eq!(match_result.start_line, 1);
        assert_eq!(match_result.end_line, 5);
        assert!((match_result.score - 0.95).abs() < 0.001);
    }
}
