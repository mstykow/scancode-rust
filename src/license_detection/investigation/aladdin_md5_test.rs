//! Diagnostic test for aladdin-md5 sequence matching candidate selection.

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;

    use crate::license_detection::index::build_index;
    use crate::license_detection::index::token_sets::{build_set_and_mset, high_tids_set_subset};
    use crate::license_detection::query::Query;
    use crate::license_detection::rules::{
        load_licenses_from_directory, load_rules_from_directory,
    };

    #[test]
    fn test_aladdin_md5_candidate_selection_diagnosis() {
        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");
        if !rules_path.exists() || !licenses_path.exists() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let test_file_path = PathBuf::from(
            "testdata/license-golden/datadriven/lic2/aladdin-md5_and_not_rsa-md5.txt",
        );
        if !test_file_path.exists() {
            eprintln!("Skipping test: test file not found");
            return;
        }

        let rules = load_rules_from_directory(&rules_path, false).expect("Failed to load rules");
        let licenses =
            load_licenses_from_directory(&licenses_path, false).expect("Failed to load licenses");
        let index = build_index(rules, licenses);

        let query_text =
            std::fs::read_to_string(&test_file_path).expect("Failed to read test file");
        let query = Query::new(&query_text, &index).expect("Query creation failed");
        let run = query.whole_query_run();

        eprintln!("\n=== DIAGNOSTIC: aladdin-md5.RULE Candidate Selection ===\n");

        // Find the aladdin-md5.RULE in the index
        let aladdin_rid = index
            .rules_by_rid
            .iter()
            .position(|r| r.identifier == "aladdin-md5.RULE");

        let rid = match aladdin_rid {
            Some(r) => r,
            None => {
                eprintln!("ERROR: aladdin-md5.RULE not found in index!");
                return;
            }
        };

        eprintln!("Found aladdin-md5.RULE at rid={}", rid);

        let rule = &index.rules_by_rid[rid];
        eprintln!("  license_expression: {}", rule.license_expression);
        eprintln!(
            "  is_approx_matchable: {}",
            index.approx_matchable_rids.contains(&rid)
        );
        eprintln!(
            "  min_high_matched_length_unique: {}",
            rule.min_high_matched_length_unique
        );
        eprintln!(
            "  min_matched_length_unique: {}",
            rule.min_matched_length_unique
        );
        eprintln!("  minimum_coverage: {:?}", rule.minimum_coverage);

        // Get query tokens
        let query_token_ids: Vec<u16> = run.tokens().to_vec();
        eprintln!("\n=== Query Analysis ===");
        eprintln!("Query has {} total tokens", query_token_ids.len());

        // Build query set
        let (query_set, _query_mset) = build_set_and_mset(&query_token_ids);
        eprintln!("Query has {} unique token IDs", query_set.len());

        // Print len_legalese
        let len_legalese = index.len_legalese;
        eprintln!("\n=== Index Info ===");
        eprintln!("len_legalese = {}", len_legalese);
        eprintln!(
            "  (Token IDs < {} are considered legalese/high-value)",
            len_legalese
        );
        eprintln!(
            "  (Token IDs >= {} are considered non-legalese/low-value)",
            len_legalese
        );

        // Get rule set
        let rule_set = index.sets_by_rid.get(&rid).expect("Rule set not found");
        eprintln!("\n=== Rule Token Set ===");
        eprintln!("Rule has {} unique token IDs", rule_set.len());

        // Count high vs low tokens in rule
        let rule_high_tokens: HashSet<u16> = rule_set
            .iter()
            .filter(|&&t| (t as usize) < len_legalese)
            .copied()
            .collect();
        let rule_low_tokens: HashSet<u16> = rule_set
            .iter()
            .filter(|&&t| (t as usize) >= len_legalese)
            .copied()
            .collect();
        eprintln!(
            "  High-value tokens (ID < {}): {} tokens",
            len_legalese,
            rule_high_tokens.len()
        );
        eprintln!(
            "  Low-value tokens (ID >= {}): {} tokens",
            len_legalese,
            rule_low_tokens.len()
        );

        // Count high vs low tokens in query
        let query_high_tokens: HashSet<u16> = query_set
            .iter()
            .filter(|&&t| (t as usize) < len_legalese)
            .copied()
            .collect();
        let query_low_tokens: HashSet<u16> = query_set
            .iter()
            .filter(|&&t| (t as usize) >= len_legalese)
            .copied()
            .collect();
        eprintln!("\n=== Query Token Set ===");
        eprintln!(
            "  High-value tokens (ID < {}): {} tokens",
            len_legalese,
            query_high_tokens.len()
        );
        eprintln!(
            "  Low-value tokens (ID >= {}): {} tokens",
            len_legalese,
            query_low_tokens.len()
        );

        // Compute intersection
        let intersection: HashSet<u16> = query_set.intersection(rule_set).copied().collect();
        eprintln!("\n=== Set Intersection ===");
        eprintln!("Intersection has {} tokens", intersection.len());

        // Split intersection into high and low
        let intersection_high: HashSet<u16> = intersection
            .iter()
            .filter(|&&t| (t as usize) < len_legalese)
            .copied()
            .collect();
        let intersection_low: HashSet<u16> = intersection
            .iter()
            .filter(|&&t| (t as usize) >= len_legalese)
            .copied()
            .collect();
        eprintln!(
            "  High-value in intersection (ID < {}): {} tokens",
            len_legalese,
            intersection_high.len()
        );
        eprintln!(
            "  Low-value in intersection (ID >= {}): {} tokens",
            len_legalese,
            intersection_low.len()
        );

        // Apply high_tids_set_subset
        let high_set_intersection = high_tids_set_subset(&intersection, len_legalese);
        eprintln!("\n=== high_tids_set_subset Result ===");
        eprintln!(
            "high_set_intersection.is_empty() = {}",
            high_set_intersection.is_empty()
        );
        eprintln!(
            "high_set_intersection.len() = {}",
            high_set_intersection.len()
        );

        if high_set_intersection.is_empty() {
            eprintln!("\n=== ROOT CAUSE IDENTIFIED ===");
            eprintln!(
                "The intersection contains only LOW-VALUE tokens (IDs >= {})",
                len_legalese
            );
            eprintln!(
                "high_tids_set_subset filters to keep only high-value tokens (IDs < {})",
                len_legalese
            );
            eprintln!(
                "Since there are no high-value tokens in common, high_set_intersection is EMPTY"
            );
            eprintln!("This causes the rule to be filtered out at candidates.rs:326-328");
            eprintln!("\nThe intersection tokens are all low-value/common words that appear in both texts,");
            eprintln!("but the aladdin-md5.RULE legalese tokens are NOT present in the query.");
        }

        // Check what high-value tokens are in the rule but NOT in the query
        eprintln!("\n=== High-Value Tokens Missing from Query ===");
        let rule_high_missing: HashSet<u16> = rule_high_tokens
            .difference(&query_high_tokens)
            .copied()
            .collect();
        eprintln!(
            "Rule has {} high-value tokens NOT in query",
            rule_high_missing.len()
        );

        // Now check the thresholds step by step
        eprintln!("\n=== Threshold Checks ===");
        let high_matched_length = high_set_intersection.len();
        eprintln!("high_matched_length = {}", high_matched_length);
        eprintln!(
            "rule.min_high_matched_length_unique = {}",
            rule.min_high_matched_length_unique
        );
        eprintln!(
            "PASSES: {}",
            high_matched_length >= rule.min_high_matched_length_unique
        );

        let matched_length = intersection.len();
        eprintln!("\nmatched_length = {}", matched_length);
        eprintln!(
            "rule.min_matched_length_unique = {}",
            rule.min_matched_length_unique
        );
        eprintln!(
            "PASSES: {}",
            matched_length >= rule.min_matched_length_unique
        );

        // Compute resemblance
        let qset_len = query_set.len();
        let iset_len = rule.length_unique;
        eprintln!("\nqset_len (query unique) = {}", qset_len);
        eprintln!("iset_len (rule unique) = {}", iset_len);

        let union_len = qset_len + iset_len - matched_length;
        let resemblance = matched_length as f32 / union_len as f32;
        let containment = matched_length as f32 / iset_len as f32;
        eprintln!("union_len = {}", union_len);
        eprintln!("resemblance = {:.4}", resemblance);
        eprintln!("containment = {:.4}", containment);

        // HIGH_RESEMBLANCE_THRESHOLD
        const HIGH_RESEMBLANCE_THRESHOLD: f32 = 0.7;
        eprintln!(
            "\nHIGH_RESEMBLANCE_THRESHOLD = {}",
            HIGH_RESEMBLANCE_THRESHOLD
        );
        eprintln!(
            "is_highly_resemblant = {}",
            resemblance >= HIGH_RESEMBLANCE_THRESHOLD
        );

        // Now actually run the candidate selection
        eprintln!("\n=== Running Full Candidate Selection (high_resemblance=true) ===");
        use crate::license_detection::seq_match::compute_candidates_with_msets;
        let candidates_high = compute_candidates_with_msets(&index, &run, true, 100);
        eprintln!(
            "Found {} candidates with high_resemblance=true",
            candidates_high.len()
        );

        let aladdin_candidate = candidates_high
            .iter()
            .find(|c| c.rule.identifier == "aladdin-md5.RULE");
        if let Some(candidate) = aladdin_candidate {
            eprintln!("\n=== ALADDIN-MD5 CANDIDATE FOUND (high_resemblance=true) ===");
            eprintln!("resemblance: {:.4}", candidate.score_vec_full.resemblance);
            eprintln!("containment: {:.4}", candidate.score_vec_full.containment);
        } else {
            eprintln!("\n=== ALADDIN-MD5 CANDIDATE NOT FOUND (high_resemblance=true) ===");
        }

        // Also try with high_resemblance=false
        eprintln!("\n=== Running Full Candidate Selection (high_resemblance=false) ===");
        let candidates_low = compute_candidates_with_msets(&index, &run, false, 100);
        eprintln!(
            "Found {} candidates with high_resemblance=false",
            candidates_low.len()
        );

        let aladdin_candidate = candidates_low
            .iter()
            .find(|c| c.rule.identifier == "aladdin-md5.RULE");
        if let Some(candidate) = aladdin_candidate {
            eprintln!("\n=== ALADDIN-MD5 CANDIDATE FOUND (high_resemblance=false) ===");
            eprintln!("resemblance: {:.4}", candidate.score_vec_full.resemblance);
            eprintln!("containment: {:.4}", candidate.score_vec_full.containment);
        } else {
            eprintln!("\n=== ALADDIN-MD5 CANDIDATE NOT FOUND (high_resemblance=false) ===");
            eprintln!("Showing top 5 candidates:");
            for (i, c) in candidates_low.iter().take(5).enumerate() {
                eprintln!(
                    "  {}. {} (resemblance: {:.4})",
                    i + 1,
                    c.rule.identifier,
                    c.score_vec_full.resemblance
                );
            }
        }
    }

    #[test]
    fn test_aladdin_md5_full_detection() {
        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");
        if !rules_path.exists() || !licenses_path.exists() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let test_file_path = PathBuf::from(
            "testdata/license-golden/datadriven/lic2/aladdin-md5_and_not_rsa-md5.txt",
        );
        if !test_file_path.exists() {
            eprintln!("Skipping test: test file not found");
            return;
        }

        use crate::license_detection::LicenseDetectionEngine;

        let engine = LicenseDetectionEngine::new(&rules_path).expect("Failed to create engine");
        let text = std::fs::read_to_string(&test_file_path).expect("Failed to read test file");

        let detections = engine.detect(&text, false).expect("Detection failed");

        eprintln!("\n=== FULL DETECTION RESULTS ===");
        eprintln!("Found {} license detections", detections.len());

        for d in &detections {
            let expr = d.license_expression.as_deref().unwrap_or("unknown");
            let matches_str: Vec<String> = d
                .matches
                .iter()
                .map(|m| format!("{}@{}-{}", m.rule_identifier, m.start_line, m.end_line))
                .collect();
            eprintln!("  {} -> [{}]", expr, matches_str.join(", "));
        }

        let has_aladdin = detections.iter().any(|d| {
            d.matches
                .iter()
                .any(|m| m.rule_identifier == "aladdin-md5.RULE")
        });
        eprintln!("\naladdin-md5.RULE matched: {}", has_aladdin);

        assert!(
            has_aladdin,
            "aladdin-md5.RULE should be matched in the file"
        );
    }
}
