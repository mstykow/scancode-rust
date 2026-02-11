//! License Detection Engine
//!
//! This module provides license detection capabilities by analyzing text content
//! and matching it against known license patterns.

pub mod aho_match;
pub mod expression;
pub mod hash_match;
pub mod index;
mod match_refine;
mod models;
mod query;
pub mod rules;
pub mod seq_match;
pub mod spans;
pub mod spdx_lid;
pub mod spdx_mapping;
mod tokenize;
pub mod unknown_match;

pub use aho_match::{MATCH_AHO, MATCH_AHO_ORDER, aho_match};
pub use expression::{CombineRelation, combine_expressions};
pub use hash_match::{MATCH_HASH, MATCH_HASH_ORDER, compute_hash, hash_match, index_hash};
pub use match_refine::refine_matches;
pub use models::{License, LicenseMatch, Rule};
pub use seq_match::{MATCH_SEQ, MATCH_SEQ_ORDER, seq_match};
pub use spdx_lid::{MATCH_SPDX_ID, MATCH_SPDX_ID_ORDER, extract_spdx_expressions, spdx_lid_match};
pub use unknown_match::{MATCH_UNKNOWN, MATCH_UNKNOWN_ORDER, unknown_match};

/// License detection engine - placeholder for Phase 2 implementation
#[derive(Debug, Clone)]
pub struct LicenseDetectionEngine;

impl LicenseDetectionEngine {
    /// Create a new license detection engine
    pub fn new() -> Self {
        Self
    }
}

impl Default for LicenseDetectionEngine {
    fn default() -> Self {
        Self::new()
    }
}
