//! Loader-stage rule type.
//!
//! This module defines `LoadedRule`, which represents a parsed and normalized
//! rule file (.RULE or .LICENSE) before it is converted to a runtime `Rule`.
//!
//! Loader-stage responsibilities include:
//! - Text trimming and normalization
//! - Fallback/default handling derived only from one file
//! - Empty-vector to `None` cleanup
//! - File-local validation
//! - False-positive handling for missing `license_expression`

use serde::{Deserialize, Serialize};

use super::RuleKind;

/// Loader-stage representation of a rule.
///
/// This struct contains parsed and normalized data from a .RULE or .LICENSE file.
/// It is serialized at build time and deserialized at runtime, then converted
/// to a runtime `Rule` during the build stage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadedRule {
    /// Unique identifier derived from the filename (e.g., "mit.LICENSE").
    pub identifier: String,

    /// License expression string using SPDX syntax and ScanCode license keys.
    /// For false-positive rules with no source expression, this is set to "unknown".
    pub license_expression: String,

    /// Pattern text to match, trimmed and normalized.
    pub text: String,

    /// Classification of this rule, derived from source rule-kind booleans.
    pub rule_kind: RuleKind,

    /// True if exact matches to this rule are false positives.
    pub is_false_positive: bool,

    /// True if this rule text is a required phrase.
    pub is_required_phrase: bool,

    /// Relevance score 0-100 (100 is most relevant).
    pub relevance: Option<u8>,

    /// Minimum match coverage percentage (0-100) if specified.
    pub minimum_coverage: Option<u8>,

    /// True if minimum_coverage was explicitly stored in source frontmatter.
    pub has_stored_minimum_coverage: bool,

    /// Tokens must appear in order if true.
    pub is_continuous: bool,

    /// Filenames where this rule should be considered.
    pub referenced_filenames: Option<Vec<String>>,

    /// URLs that should be ignored when found in this rule text.
    pub ignorable_urls: Option<Vec<String>>,

    /// Emails that should be ignored when found in this rule text.
    pub ignorable_emails: Option<Vec<String>>,

    /// Copyrights that should be ignored when found in this rule text.
    pub ignorable_copyrights: Option<Vec<String>>,

    /// Holder names that should be ignored when found in this rule text.
    pub ignorable_holders: Option<Vec<String>>,

    /// Author names that should be ignored when found in this rule text.
    pub ignorable_authors: Option<Vec<String>>,

    /// Programming language for the rule if specified.
    pub language: Option<String>,

    /// Free text notes.
    pub notes: Option<String>,

    /// Whether this rule is deprecated.
    pub is_deprecated: bool,
}
