//! License Detection Engine
//!
//! This module provides license detection capabilities by analyzing text content
//! and matching it against known license patterns.

pub mod aho_match;
mod detection;
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

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;

use crate::license_detection::index::build_index;
use crate::license_detection::query::Query;
use crate::license_detection::rules::{load_licenses_from_directory, load_rules_from_directory};
use crate::license_detection::spdx_mapping::{SpdxMapping, build_spdx_mapping};

pub use detection::{
    DetectionGroup, FileRegion, LicenseDetection, apply_detection_preferences, classify_detection,
    create_detection_from_group, determine_spdx_expression,
    determine_spdx_expression_from_scancode, filter_detections_by_score, group_matches_by_region,
    populate_detection_from_group_with_spdx, post_process_detections, rank_detections,
    remove_duplicate_detections, sort_matches_by_line,
};

pub use aho_match::{MATCH_AHO, MATCH_AHO_ORDER, aho_match};
pub use expression::{CombineRelation, combine_expressions};
pub use hash_match::{MATCH_HASH, MATCH_HASH_ORDER, compute_hash, hash_match, index_hash};
pub use match_refine::refine_matches;
pub use models::{License, LicenseMatch, Rule};
pub use seq_match::{MATCH_SEQ, MATCH_SEQ_ORDER, seq_match};
pub use spdx_lid::{MATCH_SPDX_ID, MATCH_SPDX_ID_ORDER, extract_spdx_expressions, spdx_lid_match};
pub use unknown_match::{MATCH_UNKNOWN, MATCH_UNKNOWN_ORDER, unknown_match};

/// License detection engine that orchestrates the detection pipeline.
///
/// The engine loads license rules and builds an index for efficient matching.
/// It supports multiple matching strategies (hash, SPDX-LID, Aho-Corasick, sequence)
/// and combines their results into final license detections.
#[derive(Debug, Clone)]
pub struct LicenseDetectionEngine {
    index: Arc<index::LicenseIndex>,
    spdx_mapping: SpdxMapping,
}

impl LicenseDetectionEngine {
    /// Create a new license detection engine from a directory of license rules.
    ///
    /// # Arguments
    /// * `rules_path` - Path to directory containing .LICENSE and .RULE files
    ///
    /// # Returns
    /// A Result containing the engine or an error
    pub fn new(rules_path: &Path) -> Result<Self> {
        let rules = load_rules_from_directory(rules_path)?;
        let licenses = load_licenses_from_directory(rules_path)?;
        let index = build_index(rules, licenses);
        let spdx_mapping =
            build_spdx_mapping(&index.licenses_by_key.values().cloned().collect::<Vec<_>>());

        Ok(Self {
            index: Arc::new(index),
            spdx_mapping,
        })
    }

    /// Detect licenses in the given text.
    ///
    /// This runs the full detection pipeline:
    /// 1. Create a Query from the text
    /// 2. Run matchers in priority order (hash, SPDX-LID, Aho-Corasick, sequence, unknown)
    /// 3. Refine matches
    /// 4. Group matches by region
    /// 5. Create LicenseDetection objects
    ///
    /// # Arguments
    /// * `text` - The text to analyze
    ///
    /// # Returns
    /// A Result containing a vector of LicenseDetection objects
    pub fn detect(&self, text: &str) -> Result<Vec<LicenseDetection>> {
        let query = Query::new(text, (*self.index).clone())?;
        let query_run = query.whole_query_run();

        let mut all_matches = Vec::new();

        let hash_matches = hash_match(&self.index, &query_run);
        all_matches.extend(hash_matches);

        let spdx_matches = spdx_lid_match(&self.index, text);
        all_matches.extend(spdx_matches);

        let aho_matches = aho_match(&self.index, &query_run);
        all_matches.extend(aho_matches);

        let seq_matches = seq_match(&self.index, &query_run);
        all_matches.extend(seq_matches);

        let unknown_matches = unknown_match(&self.index, &query, &all_matches);
        all_matches.extend(unknown_matches);

        let refined = refine_matches(&self.index, all_matches, &query);

        let mut sorted = refined;
        sort_matches_by_line(&mut sorted);

        let groups = group_matches_by_region(&sorted);

        let detections: Vec<LicenseDetection> =
            groups.iter().map(create_detection_from_group).collect();

        Ok(detections)
    }

    /// Get a reference to the license index.
    pub fn index(&self) -> &index::LicenseIndex {
        &self.index
    }

    /// Get a reference to the SPDX mapping.
    pub fn spdx_mapping(&self) -> &SpdxMapping {
        &self.spdx_mapping
    }
}
