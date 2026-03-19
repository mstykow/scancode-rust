//! Loader-stage license type.
//!
//! This module defines `LoadedLicense`, which represents a parsed and normalized
//! license file (.LICENSE) before it is converted to a runtime `License`.
//!
//! Loader-stage responsibilities include:
//! - Key derivation from filename
//! - Name fallback chain resolution
//! - URL merging from multiple source fields
//! - Text trimming and normalization
//! - Deprecation metadata preservation (without filtering)

use serde::{Deserialize, Serialize};

/// Loader-stage representation of a license.
///
/// This struct contains parsed and normalized data from a .LICENSE file.
/// It is serialized at build time and deserialized at runtime, then converted
/// to a runtime `License` during the build stage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadedLicense {
    /// Unique lowercase ASCII identifier derived from the filename.
    pub key: String,

    /// Full name of the license.
    pub name: String,

    /// SPDX license identifier if available.
    pub spdx_license_key: Option<String>,

    /// Alternative SPDX license identifiers (aliases).
    pub other_spdx_license_keys: Vec<String>,

    /// License category (e.g., "Permissive", "Copyleft").
    pub category: Option<String>,

    /// Full license text, trimmed and normalized.
    pub text: String,

    /// Reference URLs for this license, merged from source URL fields.
    pub reference_urls: Vec<String>,

    /// Free text notes.
    pub notes: Option<String>,

    /// Whether this license is deprecated.
    pub is_deprecated: bool,

    /// List of license keys that replace this deprecated license.
    pub replaced_by: Vec<String>,

    /// Minimum match coverage percentage (0-100) if specified.
    pub minimum_coverage: Option<u8>,

    /// Copyrights that should be ignored when found in this license text.
    pub ignorable_copyrights: Option<Vec<String>>,

    /// Holder names that should be ignored when found in this license text.
    pub ignorable_holders: Option<Vec<String>>,

    /// Author names that should be ignored when found in this license text.
    pub ignorable_authors: Option<Vec<String>>,

    /// URLs that should be ignored when found in this license text.
    pub ignorable_urls: Option<Vec<String>>,

    /// Emails that should be ignored when found in this license text.
    pub ignorable_emails: Option<Vec<String>>,
}
