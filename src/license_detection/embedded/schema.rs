//! Schema definitions for the embedded loader artifact.
//!
//! This module defines the top-level wrapper types for the serialized
//! loader artifact that is embedded in the binary.

use serde::{Deserialize, Serialize};

use crate::license_detection::models::{LoadedLicense, LoadedRule};

/// Schema version for the embedded loader artifact.
///
/// This version should be incremented whenever the artifact format changes
/// in a way that is not backwards-compatible.
pub const SCHEMA_VERSION: u32 = 1;

/// Top-level wrapper for the embedded loader artifact.
///
/// This structure is serialized at build time and embedded in the binary.
/// At runtime, it is deserialized and fed into the build stage to construct
/// the runtime `LicenseIndex`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedLoaderSnapshot {
    /// Schema version for compatibility checking.
    pub schema_version: u32,

    /// Loaded rules from the ScanCode dataset.
    pub rules: Vec<LoadedRule>,

    /// Loaded licenses from the ScanCode dataset.
    pub licenses: Vec<LoadedLicense>,
}
