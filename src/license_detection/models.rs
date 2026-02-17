//! Core data structures for license detection.

use serde::{Deserialize, Serialize};

fn default_rule_length() -> usize {
    0
}

/// License metadata loaded from .LICENSE files.
#[derive(Debug, Clone, PartialEq)]
pub struct License {
    /// Unique lowercase ASCII identifier for this license
    pub key: String,

    /// Full name of the license
    pub name: String,

    /// SPDX license identifier if available
    pub spdx_license_key: Option<String>,

    /// Alternative SPDX license identifiers (aliases)
    pub other_spdx_license_keys: Vec<String>,

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

    /// Whether this rule is deprecated
    pub is_deprecated: bool,

    /// SPDX license identifier if available
    pub spdx_license_key: Option<String>,

    /// Alternative SPDX license identifiers (aliases)
    pub other_spdx_license_keys: Vec<String>,
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
}

impl Default for LicenseMatch {
    fn default() -> Self {
        LicenseMatch {
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
            matched_token_positions: None,
            hilen: 0,
        }
    }
}

impl LicenseMatch {
    pub fn matcher_order(&self) -> u8 {
        match self.matcher.as_str() {
            "1-hash" => 1,
            "1-spdx-id" => 1,
            "2-aho" => 2,
            "3-seq" => 3,
            "5-unknown" => 5,
            _ => 9,
        }
    }

    pub fn hilen(&self) -> usize {
        self.hilen
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        if let Some(positions) = &self.matched_token_positions {
            positions.len()
        } else {
            self.end_token.saturating_sub(self.start_token)
        }
    }

    #[allow(dead_code)]
    fn qregion_len(&self) -> usize {
        if let Some(positions) = &self.matched_token_positions {
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

    #[allow(dead_code)]
    pub fn qdensity(&self) -> f32 {
        let mlen = self.len();
        if mlen == 0 {
            return 0.0;
        }
        let qregion = self.qregion_len();
        if qregion == 0 {
            return 0.0;
        }
        mlen as f32 / qregion as f32
    }

    #[allow(dead_code)]
    pub fn idensity(&self) -> f32 {
        let mlen = self.len();
        if mlen == 0 {
            return 0.0;
        }
        let qregion = self.qregion_len();
        if qregion == 0 {
            return 1.0;
        }
        mlen as f32 / qregion as f32
    }

    pub fn surround(&self, other: &LicenseMatch) -> bool {
        self.start_line < other.start_line && self.end_line > other.end_line
    }

    pub fn qcontains(&self, other: &LicenseMatch) -> bool {
        if self.start_token == 0
            && self.end_token == 0
            && other.start_token == 0
            && other.end_token == 0
        {
            return self.start_line <= other.start_line && self.end_line >= other.end_line;
        }
        self.start_token <= other.start_token && self.end_token >= other.end_token
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_license() -> License {
        License {
            key: "mit".to_string(),
            name: "MIT License".to_string(),
            spdx_license_key: Some("MIT".to_string()),
            other_spdx_license_keys: vec![],
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
        }
    }

    fn create_rule() -> Rule {
        Rule {
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
            is_deprecated: false,
            spdx_license_key: None,
            other_spdx_license_keys: vec![],
        }
    }

    fn create_license_match() -> LicenseMatch {
        LicenseMatch {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: Some("README.md".to_string()),
            start_line: 1,
            end_line: 5,
            start_token: 0,
            end_token: 100,
            matcher: "1-hash".to_string(),
            score: 0.95,
            matched_length: 100,
            rule_length: 100,
            matched_token_positions: None,
            match_coverage: 95.0,
            rule_relevance: 100,
            rule_identifier: "mit.LICENSE".to_string(),
            rule_url: "https://scancode-licensedb.aboutcode.org/mit".to_string(),
            matched_text: Some("MIT License text...".to_string()),
            referenced_filenames: None,
            is_license_intro: false,
            is_license_clue: false,
            is_license_reference: false,
            is_license_tag: false,
            hilen: 50,
        }
    }

    #[test]
    fn test_license_creation_with_all_fields() {
        let license = create_license();

        assert_eq!(license.key, "mit");
        assert_eq!(license.name, "MIT License");
        assert_eq!(license.spdx_license_key, Some("MIT".to_string()));
        assert_eq!(license.category, Some("Permissive".to_string()));
        assert!(!license.is_deprecated);
        assert!(license.replaced_by.is_empty());
    }

    #[test]
    fn test_license_creation_with_minimal_fields() {
        let license = License {
            key: "unknown".to_string(),
            name: String::new(),
            spdx_license_key: None,
            other_spdx_license_keys: vec![],
            category: None,
            text: String::new(),
            reference_urls: vec![],
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

        assert_eq!(license.key, "unknown");
        assert!(license.name.is_empty());
        assert!(license.spdx_license_key.is_none());
        assert!(license.reference_urls.is_empty());
    }

    #[test]
    fn test_license_deprecated_with_replaced_by() {
        let license = License {
            key: "old-license".to_string(),
            name: "Old License".to_string(),
            spdx_license_key: None,
            other_spdx_license_keys: vec![],
            category: None,
            text: String::new(),
            reference_urls: vec![],
            notes: Some("Deprecated in favor of new-license".to_string()),
            is_deprecated: true,
            replaced_by: vec!["new-license".to_string()],
            minimum_coverage: None,
            ignorable_copyrights: None,
            ignorable_holders: None,
            ignorable_authors: None,
            ignorable_urls: None,
            ignorable_emails: None,
        };

        assert!(license.is_deprecated);
        assert_eq!(license.replaced_by, vec!["new-license"]);
    }

    #[test]
    fn test_license_clone_trait() {
        let license = create_license();
        let cloned = license.clone();

        assert_eq!(license, cloned);
    }

    #[test]
    fn test_license_debug_trait() {
        let license = create_license();
        let debug_str = format!("{:?}", license);

        assert!(debug_str.contains("License"));
        assert!(debug_str.contains("key: \"mit\""));
    }

    #[test]
    fn test_license_partial_eq_trait() {
        let license1 = create_license();
        let license2 = create_license();
        let mut license3 = create_license();
        license3.key = "different".to_string();

        assert_eq!(license1, license2);
        assert_ne!(license1, license3);
    }

    #[test]
    fn test_license_with_ignorable_fields() {
        let license = License {
            key: "apache-2.0".to_string(),
            name: "Apache 2.0".to_string(),
            spdx_license_key: Some("Apache-2.0".to_string()),
            other_spdx_license_keys: vec![],
            category: Some("Permissive".to_string()),
            text: "Apache License text...".to_string(),
            reference_urls: vec![],
            notes: None,
            is_deprecated: false,
            replaced_by: vec![],
            minimum_coverage: Some(95),
            ignorable_copyrights: Some(vec!["Copyright 2000 Apache".to_string()]),
            ignorable_holders: Some(vec!["Apache Software Foundation".to_string()]),
            ignorable_authors: Some(vec!["Apache".to_string()]),
            ignorable_urls: Some(vec!["https://apache.org".to_string()]),
            ignorable_emails: Some(vec!["legal@apache.org".to_string()]),
        };

        assert_eq!(license.minimum_coverage, Some(95));
        assert_eq!(
            license.ignorable_copyrights,
            Some(vec!["Copyright 2000 Apache".to_string()])
        );
        assert_eq!(
            license.ignorable_holders,
            Some(vec!["Apache Software Foundation".to_string()])
        );
    }

    #[test]
    fn test_rule_creation_with_all_fields() {
        let rule = create_rule();

        assert_eq!(rule.identifier, "mit.LICENSE");
        assert_eq!(rule.license_expression, "mit");
        assert!(rule.is_license_notice);
        assert!(!rule.is_license_text);
        assert!(!rule.is_license_reference);
        assert!(!rule.is_license_tag);
        assert_eq!(rule.relevance, 90);
    }

    #[test]
    fn test_rule_creation_with_minimal_fields() {
        let rule = Rule {
            identifier: String::new(),
            license_expression: String::new(),
            text: String::new(),
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
            relevance: 0,
            minimum_coverage: None,
            is_continuous: false,
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

        assert!(rule.identifier.is_empty());
        assert!(rule.license_expression.is_empty());
        assert_eq!(rule.relevance, 0);
    }

    #[test]
    fn test_rule_license_flags_mutually_exclusive() {
        let mut rule = create_rule();
        assert!(rule.is_license_notice);

        rule.is_license_notice = false;
        rule.is_license_text = true;
        assert!(rule.is_license_text);
        assert!(!rule.is_license_notice);

        let flag_count = [
            rule.is_license_text,
            rule.is_license_notice,
            rule.is_license_reference,
            rule.is_license_tag,
            rule.is_license_intro,
            rule.is_license_clue,
        ]
        .iter()
        .filter(|&&f| f)
        .count();
        assert_eq!(flag_count, 1);
    }

    #[test]
    fn test_rule_clone_trait() {
        let rule = create_rule();
        let cloned = rule.clone();

        assert_eq!(rule, cloned);
    }

    #[test]
    fn test_rule_debug_trait() {
        let rule = create_rule();
        let debug_str = format!("{:?}", rule);

        assert!(debug_str.contains("Rule"));
        assert!(debug_str.contains("identifier: \"mit.LICENSE\""));
    }

    #[test]
    fn test_rule_partial_eq_trait() {
        let rule1 = create_rule();
        let rule2 = create_rule();
        let mut rule3 = create_rule();
        rule3.identifier = "different".to_string();

        assert_eq!(rule1, rule2);
        assert_ne!(rule1, rule3);
    }

    #[test]
    fn test_rule_ord_trait() {
        let mut rule1 = create_rule();
        rule1.identifier = "aaa.LICENSE".to_string();
        let mut rule2 = create_rule();
        rule2.identifier = "bbb.LICENSE".to_string();

        assert!(rule1 < rule2);
        assert!(rule2 > rule1);
    }

    #[test]
    fn test_rule_with_tokens() {
        let rule = Rule {
            identifier: "test.RULE".to_string(),
            license_expression: "test".to_string(),
            text: "test text".to_string(),
            tokens: vec![1, 2, 3, 4, 5],
            is_license_text: false,
            is_license_notice: true,
            is_license_reference: false,
            is_license_tag: false,
            is_license_intro: false,
            is_license_clue: false,
            is_false_positive: false,
            is_required_phrase: false,
            is_from_license: false,
            relevance: 100,
            minimum_coverage: None,
            is_continuous: false,
            referenced_filenames: None,
            ignorable_urls: None,
            ignorable_emails: None,
            ignorable_copyrights: None,
            ignorable_holders: None,
            ignorable_authors: None,
            language: None,
            notes: None,
            length_unique: 5,
            high_length_unique: 0,
            high_length: 0,
            min_matched_length: 0,
            min_high_matched_length: 0,
            min_matched_length_unique: 0,
            min_high_matched_length_unique: 0,
            is_small: true,
            is_tiny: false,
            is_deprecated: false,
            spdx_license_key: None,
            other_spdx_license_keys: vec![],
        };

        assert_eq!(rule.tokens.len(), 5);
        assert!(rule.is_small);
    }

    #[test]
    fn test_rule_small_and_tiny_flags() {
        let mut rule = create_rule();

        rule.is_small = true;
        rule.is_tiny = true;
        assert!(rule.is_small);
        assert!(rule.is_tiny);

        rule.is_tiny = false;
        assert!(rule.is_small);
        assert!(!rule.is_tiny);
    }

    #[test]
    fn test_rule_threshold_fields() {
        let rule = Rule {
            identifier: "complex.RULE".to_string(),
            license_expression: "complex".to_string(),
            text: "complex text".to_string(),
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
            relevance: 100,
            minimum_coverage: Some(80),
            is_continuous: true,
            referenced_filenames: Some(vec!["LICENSE".to_string()]),
            ignorable_urls: Some(vec!["https://example.com".to_string()]),
            ignorable_emails: Some(vec!["test@example.com".to_string()]),
            ignorable_copyrights: None,
            ignorable_holders: None,
            ignorable_authors: None,
            language: Some("en".to_string()),
            notes: Some("Test rule".to_string()),
            length_unique: 10,
            high_length_unique: 5,
            high_length: 8,
            min_matched_length: 4,
            min_high_matched_length: 2,
            min_matched_length_unique: 3,
            min_high_matched_length_unique: 1,
            is_small: false,
            is_tiny: false,
            is_deprecated: false,
            spdx_license_key: None,
            other_spdx_license_keys: vec![],
        };

        assert_eq!(rule.minimum_coverage, Some(80));
        assert_eq!(rule.referenced_filenames, Some(vec!["LICENSE".to_string()]));
        assert_eq!(rule.length_unique, 10);
        assert_eq!(rule.high_length, 8);
        assert_eq!(rule.min_matched_length, 4);
    }

    #[test]
    fn test_license_match_creation_with_all_fields() {
        let match_result = create_license_match();

        assert_eq!(match_result.license_expression, "mit");
        assert_eq!(match_result.license_expression_spdx, "MIT");
        assert_eq!(match_result.from_file, Some("README.md".to_string()));
        assert_eq!(match_result.start_line, 1);
        assert_eq!(match_result.end_line, 5);
        assert_eq!(match_result.matcher, "1-hash");
        assert!((match_result.score - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_license_match_creation_with_minimal_fields() {
        let match_result = LicenseMatch {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
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
            matched_token_positions: None,
            hilen: 0,
        };

        assert!(match_result.from_file.is_none());
        assert_eq!(match_result.start_line, 0);
        assert_eq!(match_result.score, 0.0);
        assert!(match_result.matched_text.is_none());
    }

    #[test]
    fn test_license_match_score_boundaries() {
        let mut match_result = create_license_match();

        match_result.score = 0.0;
        assert_eq!(match_result.score, 0.0);

        match_result.score = 1.0;
        assert_eq!(match_result.score, 1.0);

        match_result.score = 0.5;
        assert!((match_result.score - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_license_match_coverage_boundaries() {
        let mut match_result = create_license_match();

        match_result.match_coverage = 0.0;
        assert_eq!(match_result.match_coverage, 0.0);

        match_result.match_coverage = 100.0;
        assert_eq!(match_result.match_coverage, 100.0);

        match_result.match_coverage = 50.0;
        assert_eq!(match_result.match_coverage, 50.0);
    }

    #[test]
    fn test_license_match_clone_trait() {
        let match_result = create_license_match();
        let cloned = match_result.clone();

        assert_eq!(match_result, cloned);
    }

    #[test]
    fn test_license_match_debug_trait() {
        let match_result = create_license_match();
        let debug_str = format!("{:?}", match_result);

        assert!(debug_str.contains("LicenseMatch"));
        assert!(debug_str.contains("license_expression: \"mit\""));
    }

    #[test]
    fn test_license_match_partial_eq_trait() {
        let match1 = create_license_match();
        let match2 = create_license_match();
        let mut match3 = create_license_match();
        match3.start_line = 99;

        assert_eq!(match1, match2);
        assert_ne!(match1, match3);
    }

    #[test]
    fn test_license_match_serialization() {
        let match_result = create_license_match();
        let json = serde_json::to_string(&match_result).unwrap();

        assert!(json.contains("\"license_expression\":\"mit\""));
        assert!(json.contains("\"license_expression_spdx\":\"MIT\""));
        assert!(json.contains("\"start_line\":1"));
    }

    #[test]
    fn test_license_match_deserialization() {
        let json = r#"{
            "license_expression": "apache-2.0",
            "license_expression_spdx": "Apache-2.0",
            "from_file": "LICENSE",
            "start_line": 10,
            "end_line": 20,
            "matcher": "2-hash",
            "score": 0.99,
            "matched_length": 500,
            "match_coverage": 99.0,
            "rule_relevance": 95,
            "rule_identifier": "apache-2.0.LICENSE",
            "rule_url": "https://example.org/apache-2.0",
            "matched_text": "Apache License",
            "referenced_filenames": ["NOTICE"],
            "is_license_intro": false,
            "is_license_clue": false
        }"#;

        let match_result: LicenseMatch = serde_json::from_str(json).unwrap();

        assert_eq!(match_result.license_expression, "apache-2.0");
        assert_eq!(match_result.start_line, 10);
        assert_eq!(match_result.end_line, 20);
        assert!((match_result.score - 0.99).abs() < 0.001);
        assert_eq!(
            match_result.referenced_filenames,
            Some(vec!["NOTICE".to_string()])
        );
    }

    #[test]
    fn test_license_match_roundtrip_serialization() {
        let original = create_license_match();
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: LicenseMatch = serde_json::from_str(&json).unwrap();

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_license_match_with_referenced_filenames() {
        let match_result = LicenseMatch {
            license_expression: "mit".to_string(),
            license_expression_spdx: "MIT".to_string(),
            from_file: Some("README.md".to_string()),
            start_line: 1,
            end_line: 5,
            start_token: 0,
            end_token: 100,
            matcher: "1-hash".to_string(),
            score: 0.95,
            matched_length: 100,
            rule_length: 100,
            matched_token_positions: None,
            match_coverage: 95.0,
            rule_relevance: 100,
            rule_identifier: "mit.LICENSE".to_string(),
            rule_url: "https://scancode-licensedb.aboutcode.org/mit".to_string(),
            matched_text: Some("MIT License text...".to_string()),
            referenced_filenames: Some(vec!["LICENSE".to_string(), "COPYING".to_string()]),
            is_license_intro: false,
            is_license_clue: false,
            is_license_reference: false,
            is_license_tag: false,
            hilen: 50,
        };

        assert_eq!(
            match_result.referenced_filenames,
            Some(vec!["LICENSE".to_string(), "COPYING".to_string()])
        );
    }

    #[test]
    fn test_matcher_order_hash() {
        let match_result = create_license_match();
        assert_eq!(match_result.matcher_order(), 1);
    }

    #[test]
    fn test_matcher_order_aho() {
        let mut match_result = create_license_match();
        match_result.matcher = "2-aho".to_string();
        assert_eq!(match_result.matcher_order(), 2);
    }

    #[test]
    fn test_matcher_order_spdx() {
        let mut match_result = create_license_match();
        match_result.matcher = "3-spdx".to_string();
        assert_eq!(match_result.matcher_order(), 3);
    }

    #[test]
    fn test_matcher_order_seq() {
        let mut match_result = create_license_match();
        match_result.matcher = "4-seq".to_string();
        assert_eq!(match_result.matcher_order(), 4);
    }

    #[test]
    fn test_matcher_order_unknown() {
        let mut match_result = create_license_match();
        match_result.matcher = "5-unknown".to_string();
        assert_eq!(match_result.matcher_order(), 5);
    }

    #[test]
    fn test_matcher_order_invalid() {
        let mut match_result = create_license_match();
        match_result.matcher = "invalid".to_string();
        assert_eq!(match_result.matcher_order(), 9);
    }

    #[test]
    fn test_hilen_basic() {
        let match_result = create_license_match();
        assert_eq!(match_result.hilen(), 50);
    }

    #[test]
    fn test_hilen_zero() {
        let mut match_result = create_license_match();
        match_result.hilen = 0;
        assert_eq!(match_result.hilen(), 0);
    }

    #[test]
    fn test_hilen_value() {
        let mut match_result = create_license_match();
        match_result.hilen = 25;
        assert_eq!(match_result.hilen(), 25);
    }

    #[test]
    fn test_len_contiguous() {
        let match_result = create_license_match();
        assert_eq!(match_result.len(), 100);
    }

    #[test]
    fn test_len_non_contiguous() {
        let mut match_result = create_license_match();
        match_result.matched_token_positions = Some(vec![0, 2, 5, 10]);
        assert_eq!(match_result.len(), 4);
    }

    #[test]
    fn test_len_zero() {
        let mut match_result = create_license_match();
        match_result.start_token = 0;
        match_result.end_token = 0;
        assert_eq!(match_result.len(), 0);
    }

    #[test]
    fn test_qdensity_contiguous() {
        let match_result = create_license_match();
        assert!((match_result.qdensity() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_qdensity_sparse() {
        let mut match_result = create_license_match();
        match_result.matched_token_positions = Some(vec![0, 10]);
        let expected = 2.0 / 11.0;
        assert!((match_result.qdensity() - expected).abs() < 0.001);
    }

    #[test]
    fn test_qdensity_zero() {
        let mut match_result = create_license_match();
        match_result.start_token = 0;
        match_result.end_token = 0;
        assert_eq!(match_result.qdensity(), 0.0);
    }

    #[test]
    fn test_idensity_contiguous() {
        let match_result = create_license_match();
        assert!((match_result.idensity() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_idensity_sparse() {
        let mut match_result = create_license_match();
        match_result.matched_token_positions = Some(vec![0, 10]);
        let expected = 2.0 / 11.0;
        assert!((match_result.idensity() - expected).abs() < 0.001);
    }

    #[test]
    fn test_idensity_zero() {
        let mut match_result = create_license_match();
        match_result.start_token = 0;
        match_result.end_token = 0;
        assert_eq!(match_result.idensity(), 0.0);
    }

    #[test]
    fn test_surround_true() {
        let outer = LicenseMatch {
            start_line: 1,
            end_line: 20,
            ..create_license_match()
        };
        let inner = LicenseMatch {
            start_line: 5,
            end_line: 15,
            ..create_license_match()
        };
        assert!(outer.surround(&inner));
    }

    #[test]
    fn test_surround_false_same_start() {
        let outer = LicenseMatch {
            start_line: 1,
            end_line: 20,
            ..create_license_match()
        };
        let inner = LicenseMatch {
            start_line: 1,
            end_line: 15,
            ..create_license_match()
        };
        assert!(!outer.surround(&inner));
    }

    #[test]
    fn test_surround_false_same_end() {
        let outer = LicenseMatch {
            start_line: 1,
            end_line: 20,
            ..create_license_match()
        };
        let inner = LicenseMatch {
            start_line: 5,
            end_line: 20,
            ..create_license_match()
        };
        assert!(!outer.surround(&inner));
    }

    #[test]
    fn test_surround_false_reversed() {
        let outer = LicenseMatch {
            start_line: 5,
            end_line: 15,
            ..create_license_match()
        };
        let inner = LicenseMatch {
            start_line: 1,
            end_line: 20,
            ..create_license_match()
        };
        assert!(!outer.surround(&inner));
    }

    #[test]
    fn test_surround_false_adjacent() {
        let first = LicenseMatch {
            start_line: 1,
            end_line: 10,
            ..create_license_match()
        };
        let second = LicenseMatch {
            start_line: 11,
            end_line: 20,
            ..create_license_match()
        };
        assert!(!first.surround(&second));
        assert!(!second.surround(&first));
    }

    #[test]
    fn test_qcontains_simple_contained() {
        let outer = LicenseMatch {
            start_token: 0,
            end_token: 20,
            ..create_license_match()
        };
        let inner = LicenseMatch {
            start_token: 5,
            end_token: 15,
            ..create_license_match()
        };
        assert!(outer.qcontains(&inner));
        assert!(!inner.qcontains(&outer));
    }

    #[test]
    fn test_qcontains_same_boundaries() {
        let a = LicenseMatch {
            start_token: 0,
            end_token: 10,
            ..create_license_match()
        };
        let b = LicenseMatch {
            start_token: 0,
            end_token: 10,
            ..create_license_match()
        };
        assert!(a.qcontains(&b));
        assert!(b.qcontains(&a));
    }

    #[test]
    fn test_qcontains_overlapping_not_contained() {
        let a = LicenseMatch {
            start_token: 0,
            end_token: 10,
            ..create_license_match()
        };
        let b = LicenseMatch {
            start_token: 5,
            end_token: 15,
            ..create_license_match()
        };
        assert!(!a.qcontains(&b));
        assert!(!b.qcontains(&a));
    }

    #[test]
    fn test_qcontains_no_overlap() {
        let a = LicenseMatch {
            start_token: 0,
            end_token: 5,
            ..create_license_match()
        };
        let b = LicenseMatch {
            start_token: 10,
            end_token: 15,
            ..create_license_match()
        };
        assert!(!a.qcontains(&b));
        assert!(!b.qcontains(&a));
    }

    #[test]
    fn test_qcontains_start_overlap_only() {
        let a = LicenseMatch {
            start_token: 0,
            end_token: 10,
            ..create_license_match()
        };
        let b = LicenseMatch {
            start_token: 0,
            end_token: 15,
            ..create_license_match()
        };
        assert!(!a.qcontains(&b));
        assert!(b.qcontains(&a));
    }

    #[test]
    fn test_qcontains_end_overlap_only() {
        let a = LicenseMatch {
            start_token: 5,
            end_token: 15,
            ..create_license_match()
        };
        let b = LicenseMatch {
            start_token: 0,
            end_token: 15,
            ..create_license_match()
        };
        assert!(!a.qcontains(&b));
        assert!(b.qcontains(&a));
    }

    #[test]
    fn test_qcontains_zero_tokens_fallback_line_contained() {
        let outer = LicenseMatch {
            start_token: 0,
            end_token: 0,
            start_line: 1,
            end_line: 20,
            ..create_license_match()
        };
        let inner = LicenseMatch {
            start_token: 0,
            end_token: 0,
            start_line: 5,
            end_line: 15,
            ..create_license_match()
        };
        assert!(outer.qcontains(&inner));
        assert!(!inner.qcontains(&outer));
    }

    #[test]
    fn test_qcontains_zero_tokens_fallback_same_lines() {
        let a = LicenseMatch {
            start_token: 0,
            end_token: 0,
            start_line: 1,
            end_line: 10,
            ..create_license_match()
        };
        let b = LicenseMatch {
            start_token: 0,
            end_token: 0,
            start_line: 1,
            end_line: 10,
            ..create_license_match()
        };
        assert!(a.qcontains(&b));
        assert!(b.qcontains(&a));
    }

    #[test]
    fn test_qcontains_zero_tokens_fallback_no_containment() {
        let a = LicenseMatch {
            start_token: 0,
            end_token: 0,
            start_line: 1,
            end_line: 10,
            ..create_license_match()
        };
        let b = LicenseMatch {
            start_token: 0,
            end_token: 0,
            start_line: 5,
            end_line: 15,
            ..create_license_match()
        };
        assert!(!a.qcontains(&b));
        assert!(!b.qcontains(&a));
    }

    #[test]
    fn test_qcontains_zero_tokens_fallback_different_lines() {
        let a = LicenseMatch {
            start_token: 0,
            end_token: 0,
            start_line: 1,
            end_line: 5,
            ..create_license_match()
        };
        let b = LicenseMatch {
            start_token: 0,
            end_token: 0,
            start_line: 10,
            end_line: 15,
            ..create_license_match()
        };
        assert!(!a.qcontains(&b));
        assert!(!b.qcontains(&a));
    }

    #[test]
    fn test_qcontains_mixed_tokens_uses_token_positions() {
        let a = LicenseMatch {
            start_token: 0,
            end_token: 0,
            start_line: 1,
            end_line: 20,
            ..create_license_match()
        };
        let b = LicenseMatch {
            start_token: 5,
            end_token: 10,
            start_line: 5,
            end_line: 15,
            ..create_license_match()
        };
        assert!(!a.qcontains(&b));
    }
}
