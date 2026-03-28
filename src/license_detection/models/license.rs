//! License metadata loaded from .LICENSE files.

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

/// License metadata loaded from .LICENSE files.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Default,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
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
