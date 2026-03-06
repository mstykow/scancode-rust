//! Check if unknown-license-reference_118.RULE is being selected as a candidate

#[cfg(test)]
mod tests {
    use crate::license_detection::index::build_index;
    use crate::license_detection::query::Query;
    use crate::license_detection::rules::{
        load_licenses_from_directory, load_rules_from_directory,
    };
    use crate::license_detection::seq_match::{
        MAX_NEAR_DUPE_CANDIDATES, compute_candidates_with_msets,
    };
    use std::path::PathBuf;

    #[test]
    fn test_rule_118_in_candidates() {
        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");

        let rules = load_rules_from_directory(&rules_path, false).unwrap();
        let licenses = load_licenses_from_directory(&licenses_path, false).unwrap();
        let index = build_index(rules, licenses);

        // Find rule 118
        let mut rule_118_rid = None;
        for (rid, rule) in index.rules_by_rid.iter().enumerate() {
            if rule.identifier == "unknown-license-reference_118.RULE" {
                rule_118_rid = Some(rid);
                eprintln!("Found rule {} at rid={}", rule.identifier, rid);
                eprintln!("  is_small: {}", rule.is_small);
                eprintln!("  is_tiny: {}", rule.is_tiny);
                eprintln!("  is_license_reference: {}", rule.is_license_reference);
                eprintln!("  tokens.len: {}", rule.tokens.len());
                eprintln!(
                    "  min_matched_length_unique: {}",
                    rule.min_matched_length_unique
                );
                eprintln!(
                    "  min_high_matched_length_unique: {}",
                    rule.min_high_matched_length_unique
                );
                break;
            }
        }

        let rid = rule_118_rid.expect("Rule 118 should exist");
        eprintln!(
            "Is rid {} in approx_matchable_rids? {}",
            rid,
            index.approx_matchable_rids.contains(&rid)
        );

        // Now test with cigna text
        let text = std::fs::read_to_string(
            "testdata/license-golden/datadriven/unknown/cigna-go-you-mobile-app-eula.txt",
        )
        .unwrap();
        let query = Query::new(&text, &index).expect("Query creation failed");
        let whole_run = query.whole_query_run();

        let candidates =
            compute_candidates_with_msets(&index, &whole_run, false, MAX_NEAR_DUPE_CANDIDATES);

        eprintln!("\n=== CANDIDATES ===");
        let mut found_118 = false;
        for cand in &candidates {
            let rule = &index.rules_by_rid[cand.rid as usize];
            if rule.identifier.contains("unknown-license-reference") {
                eprintln!(
                    "{}: containment={:.4}, resemblance={:.4}, matched_length={}",
                    rule.identifier,
                    cand.score_vec_full.containment,
                    cand.score_vec_full.resemblance,
                    cand.score_vec_full.matched_length
                );
                if rule.identifier == "unknown-license-reference_118.RULE" {
                    found_118 = true;
                }
            }
        }

        eprintln!("\nTotal candidates returned: {}", candidates.len());

        if !found_118 {
            eprintln!("\n=== Rule 118 NOT in candidates - Investigating why ===");
            let rule = &index.rules_by_rid[rid];
            let rule_set = index.sets_by_rid.get(&rid).expect("rule set");

            let query_tokens: Vec<u16> = whole_run
                .matchable_tokens()
                .iter()
                .filter_map(|&tid| if tid >= 0 { Some(tid as u16) } else { None })
                .collect();

            use std::collections::HashSet;
            let query_set: HashSet<u16> = query_tokens.iter().copied().collect();

            let intersection: HashSet<u16> = query_set.intersection(rule_set).copied().collect();

            // Check step 1 scoring
            let matched_length = intersection.len();
            let qset_len = query_set.len();
            let iset_len = rule.length_unique;
            let union_len = qset_len + iset_len - matched_length;
            let resemblance = matched_length as f32 / union_len as f32;
            let containment = matched_length as f32 / iset_len as f32;
            let amplified_resemblance = resemblance.powi(2);

            eprintln!("Step 1 scores (set-based) for rule 118:");
            eprintln!("  matched_length (set): {}", matched_length);
            eprintln!("  qset_len: {}", qset_len);
            eprintln!("  iset_len (rule.length_unique): {}", iset_len);
            eprintln!("  resemblance: {:.6}", resemblance);
            eprintln!("  containment: {:.6}", containment);
            eprintln!("  amplified_resemblance: {:.6}", amplified_resemblance);
            eprintln!("\n  Rounded scores:");
            eprintln!(
                "    containment (rounded): {:.1}",
                (containment * 10.0).round() / 10.0
            );
            eprintln!(
                "    resemblance (rounded): {:.1}",
                (amplified_resemblance * 10.0).round() / 10.0
            );
            eprintln!(
                "    matched_length (rounded): {:.1}",
                (matched_length as f32 / 20.0).round()
            );

            // MAX_NEAR_DUPE_CANDIDATES is 10, so top_n * 10 = 100
            eprintln!(
                "\n  MAX_NEAR_DUPE_CANDIDATES * 10 = {}",
                MAX_NEAR_DUPE_CANDIDATES * 10
            );
        }
    }
}
