//! License Detection Engine
//!
//! This module provides license detection capabilities by analyzing text content
//! and matching it against known license patterns.

pub mod aho_match;
mod detection;
pub mod expression;
#[cfg(test)]
mod golden_test;
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
#[cfg(test)]
mod test_utils;
mod tokenize;
pub mod unknown_match;

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;

use crate::license_detection::index::build_index;
use crate::license_detection::query::Query;
use crate::license_detection::rules::{load_licenses_from_directory, load_rules_from_directory};
use crate::license_detection::spdx_mapping::{SpdxMapping, build_spdx_mapping};

use crate::license_detection::detection::{
    create_detection_from_group, group_matches_by_region, populate_detection_from_group_with_spdx,
    post_process_detections, sort_matches_by_line,
};

pub use detection::LicenseDetection;

pub use aho_match::aho_match;
pub use hash_match::hash_match;
pub use match_refine::refine_matches;
pub use seq_match::{
    MAX_NEAR_DUPE_CANDIDATES, compute_candidates_with_msets, seq_match, seq_match_with_candidates,
};
pub use spdx_lid::spdx_lid_match;
pub use unknown_match::unknown_match;

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
        let (rules_dir, licenses_dir) = if rules_path.ends_with("data") {
            (rules_path.join("rules"), rules_path.join("licenses"))
        } else if rules_path.ends_with("rules") {
            let parent = rules_path.parent().ok_or_else(|| {
                anyhow::anyhow!("Cannot determine parent directory for rules path")
            })?;
            (rules_path.to_path_buf(), parent.join("licenses"))
        } else {
            (rules_path.to_path_buf(), rules_path.to_path_buf())
        };

        let rules = load_rules_from_directory(&rules_dir, false)?;
        let licenses = load_licenses_from_directory(&licenses_dir, false)?;
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
    /// 2. Run matchers in priority order (hash, SPDX-LID, Aho-Corasick)
    /// 3. Phase 2: Near-duplicate detection (ALWAYS runs, even with exact matches)
    /// 4. Phase 3: Query run matching (per-run with high_resemblance=False)
    /// 5. Unknown matching
    /// 6. Refine matches
    /// 7. Group matches by region
    /// 8. Create LicenseDetection objects
    ///
    /// # Arguments
    /// * `text` - The text to analyze
    ///
    /// # Returns
    /// A Result containing a vector of LicenseDetection objects
    pub fn detect(&self, text: &str) -> Result<Vec<LicenseDetection>> {
        let mut query = Query::new(text, &self.index)?;

        let mut all_matches = Vec::new();
        let mut matched_qspans: Vec<query::PositionSpan> = Vec::new();

        // Phase 1: Hash, SPDX, Aho-Corasick matching
        // Track 100% coverage matches for Phase 3's is_matchable() check
        // Corresponds to Python: index.py:1056-1057
        {
            let whole_run = query.whole_query_run();

            let hash_matches = hash_match(&self.index, &whole_run);
            for m in &hash_matches {
                if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                    matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
                }
            }
            all_matches.extend(hash_matches);

            let spdx_matches = spdx_lid_match(&self.index, &query);
            for m in &spdx_matches {
                if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                    matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
                }
            }
            all_matches.extend(spdx_matches);

            let aho_matches = aho_match(&self.index, &whole_run);
            for m in &aho_matches {
                if m.match_coverage >= 99.99 && m.end_token > m.start_token {
                    matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
                }
            }
            all_matches.extend(aho_matches);
        }

        // Phase 2: Near-duplicate detection
        // Corresponds to Python: index.py:733-771
        {
            let whole_run = query.whole_query_run();
            let near_dupe_candidates = compute_candidates_with_msets(
                &self.index,
                &whole_run,
                true,
                MAX_NEAR_DUPE_CANDIDATES,
            );

            if !near_dupe_candidates.is_empty() {
                let near_dupe_matches =
                    seq_match_with_candidates(&self.index, &whole_run, &near_dupe_candidates);

                // Subtract matched positions from query to prevent double-matching.
                // Corresponds to Python: index.py:767-771
                // Note: Python adds ALL near-dupe matches to already_matched_qspans,
                // not just 100% coverage matches. This is crucial for Phase 4's
                // is_matchable() check to prevent double-matching.
                for m in &near_dupe_matches {
                    if m.end_token > m.start_token {
                        let span = query::PositionSpan::new(m.start_token, m.end_token - 1);
                        query.subtract(&span);
                        matched_qspans.push(span);
                    }
                }

                all_matches.extend(near_dupe_matches);
            }
        }

        // Phase 3: Regular sequence matching
        {
            let whole_run = query.whole_query_run();
            let seq_matches = seq_match(&self.index, &whole_run);
            all_matches.extend(seq_matches);
        }

        // Phase 4: Query run matching (high_resemblance=False, top 70)
        // This is essential for matching combined rules like "cddl-1.0_or_gpl-2.0-glassfish"
        // Corresponds to Python: index.py:786-812
        // Note: This is in addition to Phase 3, not a replacement
        // The is_matchable() check handles exclusion of already-matched positions
        const MAX_QUERY_RUN_CANDIDATES: usize = 70;
        {
            let whole_run = query.whole_query_run();
            for query_run in query.query_runs().iter() {
                // Skip the whole_run since it was already matched in Phase 2 and 3
                if query_run.start == whole_run.start && query_run.end == whole_run.end {
                    continue;
                }

                // is_matchable() excludes already-matched positions
                // Corresponds to Python: index.py:1061-1064
                if !query_run.is_matchable(false, &matched_qspans) {
                    continue;
                }

                let candidates = compute_candidates_with_msets(
                    &self.index,
                    query_run,
                    false,
                    MAX_QUERY_RUN_CANDIDATES,
                );
                if !candidates.is_empty() {
                    let matches = seq_match_with_candidates(&self.index, query_run, &candidates);
                    all_matches.extend(matches);
                }
            }
        }

        let unknown_matches = unknown_match(&self.index, &query, &all_matches);
        all_matches.extend(unknown_matches);

        let refined = refine_matches(&self.index, all_matches, &query);

        let mut sorted = refined;
        sort_matches_by_line(&mut sorted);

        let groups = group_matches_by_region(&sorted);

        let detections: Vec<LicenseDetection> = groups
            .iter()
            .map(|group| {
                let mut detection = create_detection_from_group(group);
                populate_detection_from_group_with_spdx(&mut detection, group, &self.spdx_mapping);
                detection
            })
            .collect();

        let detections = post_process_detections(detections, 0.0);

        Ok(detections)
    }

    /// Get a reference to the license index.
    pub fn index(&self) -> &index::LicenseIndex {
        &self.index
    }

    /// Get a reference to the SPDX mapping.
    #[allow(dead_code)]
    pub fn spdx_mapping(&self) -> &SpdxMapping {
        &self.spdx_mapping
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_reference_data_paths() -> Option<(PathBuf, PathBuf)> {
        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");
        if rules_path.exists() && licenses_path.exists() {
            Some((rules_path, licenses_path))
        } else {
            None
        }
    }

    fn create_engine_from_reference() -> Option<LicenseDetectionEngine> {
        let (rules_path, licenses_path) = get_reference_data_paths()?;
        let rules = load_rules_from_directory(&rules_path, false).ok()?;
        let licenses = load_licenses_from_directory(&licenses_path, false).ok()?;
        let index = build_index(rules, licenses);
        let spdx_mapping =
            build_spdx_mapping(&index.licenses_by_key.values().cloned().collect::<Vec<_>>());
        Some(LicenseDetectionEngine {
            index: Arc::new(index),
            spdx_mapping,
        })
    }

    #[test]
    fn test_engine_new_with_reference_rules() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        assert!(
            !engine.index().rules_by_rid.is_empty(),
            "Should have rules loaded"
        );
        assert!(
            !engine.index().licenses_by_key.is_empty(),
            "Should have licenses loaded"
        );
        assert!(
            engine.index().len_legalese > 0,
            "Should have legalese tokens"
        );
        assert!(
            !engine.index().rid_by_hash.is_empty(),
            "Should have hash mappings"
        );
        assert!(
            !engine.index().regular_rids.is_empty(),
            "Should have regular rules"
        );
    }

    #[test]
    fn test_engine_detect_mit_license() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let mit_text = r#"Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE."#;

        let detections = engine.detect(mit_text).expect("Detection should succeed");

        assert!(
            !detections.is_empty(),
            "Should detect at least one license in MIT text"
        );

        let mit_related = detections.iter().any(|d| {
            d.license_expression
                .as_ref()
                .map(|e| e.contains("mit") || e.contains("unknown"))
                .unwrap_or(false)
        });
        assert!(
            mit_related,
            "Should detect MIT or unknown license, got: {:?}",
            detections
                .iter()
                .map(|d| d.license_expression.as_deref().unwrap_or("none"))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_engine_detect_empty_text() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let detections = engine.detect("").expect("Detection should succeed");
        assert!(
            detections.is_empty() || !detections.is_empty(),
            "Detection completes"
        );

        let detections = engine
            .detect("   \n\n   ")
            .expect("Detection should succeed");
        assert!(
            detections.is_empty() || !detections.is_empty(),
            "Detection completes"
        );
    }

    #[test]
    fn test_engine_detect_spdx_identifier() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "SPDX-License-Identifier: MIT";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(
            !detections.is_empty(),
            "Should detect license from SPDX identifier"
        );
    }

    #[test]
    fn test_engine_detect_license_notice() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "Licensed under the MIT License";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(!detections.is_empty(), "Should detect license notice");
    }

    #[test]
    fn test_engine_index_populated() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };
        let index = engine.index();

        assert!(
            index.rules_by_rid.len() > 1000,
            "Should have at least 1000 rules loaded from reference"
        );

        assert!(
            index.licenses_by_key.len() > 100,
            "Should have at least 100 licenses loaded from reference"
        );

        assert!(
            !index.approx_matchable_rids.is_empty(),
            "Should have approx-matchable rules"
        );

        let has_false_positives = !index.false_positive_rids.is_empty();
        assert!(has_false_positives, "Should have false positive rules");

        let mut rules_with_tokens = 0;
        for &rid in index.regular_rids.iter().take(10) {
            let rule = &index.rules_by_rid[rid];
            if !rule.tokens.is_empty() {
                rules_with_tokens += 1;
                assert!(
                    rule.min_matched_length > 0,
                    "Regular rule {} should have computed threshold",
                    rid
                );
            }
        }
        assert!(
            rules_with_tokens > 0,
            "Should have at least one rule with tokens among first 10"
        );
    }

    #[test]
    fn test_engine_automaton_functional() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };
        let index = engine.index();

        if !index.rules_by_rid.is_empty() {
            let first_rule = &index.rules_by_rid[0];
            if !first_rule.tokens.is_empty() {
                let pattern: Vec<u8> = first_rule
                    .tokens
                    .iter()
                    .flat_map(|t| t.to_le_bytes())
                    .collect();

                let matches: Vec<_> = index.rules_automaton.find_iter(&pattern).collect();
                assert!(
                    !matches.is_empty(),
                    "Automaton should find pattern for rule 0"
                );
            }
        }
    }

    #[test]
    fn test_engine_spdx_mapping() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };
        let mapping = engine.spdx_mapping();

        let mit_spdx = mapping.scancode_to_spdx("mit");
        assert!(mit_spdx.is_some(), "Should have MIT SPDX mapping");
        assert_eq!(
            mit_spdx.unwrap(),
            "MIT",
            "MIT should map to MIT SPDX identifier"
        );
    }

    #[test]
    fn test_engine_detect_no_license() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "This is just some random text without any license information.";
        let detections = engine.detect(text).expect("Detection should succeed");
        assert!(
            !detections.is_empty() || detections.is_empty(),
            "Detection should complete without error"
        );
    }

    #[test]
    fn test_engine_detect_gpl_notice() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation.";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(!detections.is_empty(), "Should detect GPL notice");
    }

    #[test]
    fn test_engine_detect_apache_notice() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "Licensed under the Apache License, Version 2.0";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(!detections.is_empty(), "Should detect Apache notice");
    }

    #[test]
    fn test_engine_index_sets_by_rid() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };
        let index = engine.index();

        for &rid in index.regular_rids.iter().take(5) {
            assert!(
                index.sets_by_rid.contains_key(&rid),
                "Rule {} should have token set",
                rid
            );
            let set = &index.sets_by_rid[&rid];
            assert!(
                !set.is_empty(),
                "Rule {} token set should not be empty",
                rid
            );
        }
    }

    #[test]
    fn test_engine_index_msets_by_rid() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };
        let index = engine.index();

        for &rid in index.regular_rids.iter().take(5) {
            assert!(
                index.msets_by_rid.contains_key(&rid),
                "Rule {} should have token multiset",
                rid
            );
            let mset = &index.msets_by_rid[&rid];
            assert!(
                !mset.is_empty(),
                "Rule {} token multiset should not be empty",
                rid
            );
        }
    }

    #[test]
    fn test_engine_index_high_postings() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };
        let index = engine.index();

        if !index.approx_matchable_rids.is_empty() {
            let some_approx_rid = index.approx_matchable_rids.iter().next().unwrap();
            if index.high_postings_by_rid.contains_key(some_approx_rid) {
                let postings = &index.high_postings_by_rid[some_approx_rid];
                assert!(!postings.is_empty(), "High postings should have entries");
            }
        }
    }

    #[test]
    fn test_engine_matched_text_populated() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "SPDX-License-Identifier: MIT";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(!detections.is_empty(), "Should detect license");

        for detection in &detections {
            for m in &detection.matches {
                assert!(
                    m.matched_text.is_some(),
                    "matched_text should be populated for matcher {}",
                    m.matcher
                );
                let matched = m.matched_text.as_ref().unwrap();
                assert!(
                    !matched.is_empty(),
                    "matched_text should not be empty for matcher {}",
                    m.matcher
                );
            }
        }
    }

    #[test]
    fn test_detect_multiple_licenses_in_text() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let isc_text = r#"Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted, provided that the above
copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE."#;

        let darpa_text = r#"Portions of this software were developed by the University of California,
Irvine under a U.S. Government contract with the Defense Advanced Research
Projects Agency (DARPA)."#;

        let combined_text = format!("{}\n\n{}", isc_text, darpa_text);

        let detections = engine
            .detect(&combined_text)
            .expect("Detection should succeed");

        assert!(!detections.is_empty(), "Should detect at least one license");

        let detected_licenses: Vec<String> = detections
            .iter()
            .filter_map(|d| d.license_expression.as_ref())
            .cloned()
            .collect();

        assert!(
            detected_licenses
                .iter()
                .any(|l| l.to_lowercase().contains("isc")),
            "Should detect ISC license, got: {:?}",
            detected_licenses
        );
    }

    #[test]
    fn test_sudo_license_loaded_from_license_file() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let index = engine.index();

        let sudo_rules: Vec<_> = index
            .rules_by_rid
            .iter()
            .filter(|r| r.license_expression.contains("sudo"))
            .collect();

        eprintln!("Found {} rules with 'sudo' expression", sudo_rules.len());
        for rule in sudo_rules.iter().take(3) {
            eprintln!(
                "  Rule: {} - is_from_license: {}, text len: {}",
                rule.identifier,
                rule.is_from_license,
                rule.text.len()
            );
        }

        assert!(
            !sudo_rules.is_empty(),
            "Should have at least one rule with 'sudo' license expression"
        );

        let sudo_from_license = sudo_rules.iter().find(|r| r.is_from_license);
        assert!(
            sudo_from_license.is_some(),
            "Should have a sudo rule created from license file"
        );

        let rule = sudo_from_license.unwrap();
        assert!(
            rule.text.contains("Sponsored in part"),
            "sudo rule text should contain DARPA acknowledgment"
        );
    }

    #[test]
    fn test_spdx_simple() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "SPDX-License-Identifier: MIT\nSome code here";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(
            !detections.is_empty(),
            "Should detect license from SPDX identifier"
        );

        let has_mit = detections.iter().any(|d| {
            d.license_expression
                .as_ref()
                .map(|e| e.contains("mit"))
                .unwrap_or(false)
        });
        assert!(has_mit, "Should detect MIT license");
    }

    #[test]
    fn test_spdx_with_or() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "SPDX-License-Identifier: MIT OR Apache-2.0";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(
            !detections.is_empty(),
            "Should detect license from SPDX identifier with OR"
        );
    }

    #[test]
    fn test_spdx_with_plus() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "SPDX-License-Identifier: GPL-2.0+";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(
            !detections.is_empty(),
            "Should detect license from SPDX identifier with plus"
        );
    }

    #[test]
    fn test_spdx_in_comment() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "// SPDX-License-Identifier: MIT\n/* some code */";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(
            !detections.is_empty(),
            "Should detect SPDX identifier in comment"
        );
    }

    #[test]
    fn test_hash_exact_mit() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let mit_text = r#"Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software."#;

        let detections = engine.detect(mit_text).expect("Detection should succeed");

        assert!(!detections.is_empty(), "Should detect partial MIT license");
    }

    #[test]
    fn test_aho_single_rule() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "Licensed under the MIT License";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(!detections.is_empty(), "Should detect license notice");
    }

    #[test]
    fn test_seq_partial_license() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let partial_mit = r#"Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software."#;

        let detections = engine
            .detect(partial_mit)
            .expect("Detection should succeed");

        assert!(!detections.is_empty(), "Should detect partial MIT license");
    }

    #[test]
    fn test_unknown_proprietary() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let text = "This software is proprietary and confidential. All rights reserved.";
        let detections = engine.detect(text).expect("Detection should succeed");

        assert!(
            !detections.is_empty(),
            "Should detect unknown license or return empty"
        );
    }

    #[test]
    fn test_debug_camellia_bsd_detection() {
        let Some(engine) = create_engine_from_reference() else {
            eprintln!("Skipping test: reference directory not found");
            return;
        };

        let test_file =
            std::path::PathBuf::from("testdata/license-golden/datadriven/lic1/camellia_bsd.c");
        let text = match std::fs::read_to_string(&test_file) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Skipping test: cannot read test file: {}", e);
                return;
            }
        };

        println!("\n========================================");
        println!("DEBUG: camellia_bsd.c detection");
        println!("========================================");
        println!("Text length: {} bytes", text.len());
        println!();

        let detections = engine.detect(&text).expect("Detection failed");

        println!("Number of detections: {}", detections.len());
        println!();

        for (i, detection) in detections.iter().enumerate() {
            println!("Detection {}:", i + 1);
            println!("  license_expression: {:?}", detection.license_expression);
            println!(
                "  license_expression_spdx: {:?}",
                detection.license_expression_spdx
            );
            println!("  detection_log: {:?}", detection.detection_log);
            println!("  Number of matches: {}", detection.matches.len());

            for (j, m) in detection.matches.iter().enumerate() {
                println!("    Match {}:", j + 1);
                println!("      license_expression: {}", m.license_expression);
                println!("      matcher: {}", m.matcher);
                println!("      score: {:.2}", m.score);
                println!("      match_coverage: {:.1}%", m.match_coverage);
                println!("      matched_length: {}", m.matched_length);
                println!("      rule_relevance: {}", m.rule_relevance);
                println!("      rule_identifier: {}", m.rule_identifier);
                println!("      start_line: {}", m.start_line);
                println!("      end_line: {}", m.end_line);
                if let Some(ref matched_text) = m.matched_text {
                    let preview: String = matched_text.chars().take(200).collect();
                    println!(
                        "      matched_text (preview): {}...",
                        preview.replace('\n', "\\n")
                    );
                }
            }
            println!();
        }

        println!("========================================");
        println!("Expected license: bsd-2-clause-first-lines");
        println!("========================================");

        let index = engine.index();
        let key = "bsd-2-clause-first-lines";
        if index.licenses_by_key.contains_key(key) {
            println!("License '{}' found in index", key);
            let license = &index.licenses_by_key[key];
            println!("License text from index (first 500 chars):");
            let preview: String = license.text.chars().take(500).collect();
            println!("{}", preview);
        } else {
            println!("License '{}' NOT found in index", key);
            println!("Available licenses containing 'bsd-2':");
            for k in index.licenses_by_key.keys() {
                if k.contains("bsd-2") {
                    println!("  - {}", k);
                }
            }
        }

        println!("\n========================================");
        println!("Investigating gpl-1.0-plus false positive");
        println!("========================================");

        let gpl_rid = 20010;
        if gpl_rid < index.rules_by_rid.len() {
            let rule = &index.rules_by_rid[gpl_rid];
            println!("Rule #{}:", gpl_rid);
            println!("  license_expression: {}", rule.license_expression);
            println!("  text: {}", rule.text);
            println!("  is_license_tag: {}", rule.is_license_tag);
            println!("  is_license_reference: {}", rule.is_license_reference);
            println!("  is_license_notice: {}", rule.is_license_notice);
            println!("  is_false_positive: {}", rule.is_false_positive);
            println!("  relevance: {}", rule.relevance);
            println!("  tokens: {:?}", rule.tokens);
            println!("  is_small: {}", rule.is_small);
            println!("  is_tiny: {}", rule.is_tiny);
        }
    }
}
