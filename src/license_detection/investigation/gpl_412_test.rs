//! Investigation test for gpl-2.0-plus_412.RULE not being found by Aho-Corasick matching.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::license_detection::aho_match;
    use crate::license_detection::index::build_index;
    use crate::license_detection::query::Query;
    use crate::license_detection::rules::{
        load_licenses_from_directory, load_rules_from_directory,
    };

    #[test]
    fn test_gpl_412_rule_investigation() {
        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");
        if !rules_path.exists() || !licenses_path.exists() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let rules = load_rules_from_directory(&rules_path, false).expect("Failed to load rules");
        let licenses =
            load_licenses_from_directory(&licenses_path, false).expect("Failed to load licenses");
        let index = build_index(rules, licenses);

        eprintln!("\n=== INVESTIGATING GPL-2.0-PLUS_412.RULE ===");

        let rule_412_rid = index
            .rules_by_rid
            .iter()
            .position(|r| r.identifier == "gpl-2.0-plus_412.RULE");

        if let Some(rid) = rule_412_rid {
            eprintln!("Rule 412 found at rid={}", rid);
            let rule = &index.rules_by_rid[rid];
            eprintln!("  license_expression: {}", rule.license_expression);
            eprintln!("  is_false_positive: {}", rule.is_false_positive);
            eprintln!("  tokens count: {}", rule.tokens.len());
            eprintln!("  minimum_coverage: {:?}", rule.minimum_coverage);
            eprintln!("  ignorable_urls: {:?}", rule.ignorable_urls);
        } else {
            eprintln!("Rule 412 NOT found!");
        }
    }

    #[test]
    fn test_gpl_412_actual_matching() {
        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");
        if !rules_path.exists() || !licenses_path.exists() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let rules = load_rules_from_directory(&rules_path, false).expect("Failed to load rules");
        let licenses =
            load_licenses_from_directory(&licenses_path, false).expect("Failed to load licenses");
        let index = build_index(rules, licenses);

        let query_text =
            "License GPLv2+: GNU GPL version 2 or later <http://gnu.org/licenses/gpl.html>.
This is free software: you are free to change and redistribute it.
There is NO WARRANTY, to the extent permitted by law.";

        let query = Query::new(query_text, &index).expect("Query creation failed");
        let run = query.whole_query_run();

        let matches = aho_match::aho_match(&index, &run);
        eprintln!("\n=== AHO MATCH RESULTS ===");
        eprintln!("Found {} Aho matches", matches.len());

        let rule_412_matched = matches
            .iter()
            .any(|m| m.rule_identifier == "gpl-2.0-plus_412.RULE");
        eprintln!("\nRule 412 matched: {}", rule_412_matched);
    }

    #[test]
    fn test_gpl_412_refinement_pipeline() {
        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");
        if !rules_path.exists() || !licenses_path.exists() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let rules = load_rules_from_directory(&rules_path, false).expect("Failed to load rules");
        let licenses =
            load_licenses_from_directory(&licenses_path, false).expect("Failed to load licenses");
        let index = build_index(rules, licenses);

        let query_text =
            "License GPLv2+: GNU GPL version 2 or later <http://gnu.org/licenses/gpl.html>.
This is free software: you are free to change and redistribute it.
There is NO WARRANTY, to the extent permitted by law.";

        let query = Query::new(query_text, &index).expect("Query creation failed");
        let run = query.whole_query_run();

        let aho_matches = aho_match::aho_match(&index, &run);
        eprintln!("\n=== STEP 1: AHO MATCHES ({}) ===", aho_matches.len());

        let rule_412_in_aho = aho_matches
            .iter()
            .any(|m| m.rule_identifier == "gpl-2.0-plus_412.RULE");
        eprintln!("\nRule 412 in Aho matches: {}", rule_412_in_aho);

        use crate::license_detection::match_refine::merge_overlapping_matches;
        let merged = merge_overlapping_matches(&aho_matches);

        let rule_412_in_merged = merged
            .iter()
            .any(|m| m.rule_identifier == "gpl-2.0-plus_412.RULE");
        eprintln!("\nRule 412 in merged: {}", rule_412_in_merged);

        use crate::license_detection::match_refine::filter_contained_matches;
        let (non_contained, _) = filter_contained_matches(&merged);

        let rule_412_in_non_contained = non_contained
            .iter()
            .any(|m| m.rule_identifier == "gpl-2.0-plus_412.RULE");
        eprintln!("\nRule 412 in non_contained: {}", rule_412_in_non_contained);

        use crate::license_detection::match_refine::filter_overlapping_matches;
        let (final_matches, _) = filter_overlapping_matches(non_contained, &index);

        let rule_412_in_final = final_matches
            .iter()
            .any(|m| m.rule_identifier == "gpl-2.0-plus_412.RULE");
        eprintln!("\nRule 412 in final: {}", rule_412_in_final);
    }

    #[test]
    fn test_gpl_412_token_comparison() {
        // Rule 412 text (actual newlines)
        let rule_text =
            "License GPLv2+: GNU GPL version 2 or later <http://gnu.org/licenses/gpl.html>.
This is free software: you are free to change and redistribute it.
There is NO WARRANTY, to the extent permitted by law.";

        // What options.c actually contains (literal \n in C strings)
        let options_c_text = r#"License GPLv2+: GNU GPL version 2 or later <http://gnu.org/licenses/gpl.html>\n
This is free software: you are free to change and redistribute it.\n
There is NO WARRANTY, to the extent permitted by law.\n"#;

        use crate::license_detection::tokenize::tokenize;

        let rule_tokens = tokenize(rule_text);
        let options_tokens = tokenize(options_c_text);

        eprintln!("\n=== TOKEN COMPARISON ===");
        eprintln!(
            "Rule text ({} tokens): {:?}",
            rule_tokens.len(),
            rule_tokens
        );
        eprintln!(
            "Options.c text ({} tokens): {:?}",
            options_tokens.len(),
            options_tokens
        );
        eprintln!("Are they equal? {}", rule_tokens == options_tokens);

        // Show character-by-character comparison
        eprintln!("\n=== CHARACTER COMPARISON ===");
        for (i, (c1, c2)) in rule_text.chars().zip(options_c_text.chars()).enumerate() {
            if c1 != c2 {
                eprintln!(
                    "Position {}: rule='{}' ({:?}) vs options='{}' ({:?})",
                    i, c1, c1, c2, c2
                );
            }
        }
    }

    #[test]
    fn test_gpl_412_in_options_c() {
        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/rules");
        let licenses_path =
            PathBuf::from("reference/scancode-toolkit/src/licensedcode/data/licenses");
        if !rules_path.exists() || !licenses_path.exists() {
            eprintln!("Skipping test: reference directory not found");
            return;
        }

        let options_c_path =
            PathBuf::from("testdata/license-golden/datadriven/lic2/regression/options.c");
        if !options_c_path.exists() {
            eprintln!("Skipping test: options.c not found");
            return;
        }

        let rules = load_rules_from_directory(&rules_path, false).expect("Failed to load rules");
        let licenses =
            load_licenses_from_directory(&licenses_path, false).expect("Failed to load licenses");
        let index = build_index(rules, licenses);

        let text = std::fs::read_to_string(&options_c_path).expect("Failed to read options.c");
        eprintln!("\n=== OPTIONS.C ANALYSIS ===");
        eprintln!(
            "File has {} bytes, {} lines",
            text.len(),
            text.lines().count()
        );

        let query = Query::new(&text, &index).expect("Query creation failed");
        let run = query.whole_query_run();

        eprintln!("\n=== QUERY INFO ===");
        eprintln!("Query has {} tokens", run.tokens().len());

        let aho_matches = aho_match::aho_match(&index, &run);
        eprintln!("\n=== AHO MATCHES ({}) ===", aho_matches.len());

        let rule_412_matches: Vec<_> = aho_matches
            .iter()
            .filter(|m| m.rule_identifier == "gpl-2.0-plus_412.RULE")
            .collect();
        eprintln!("Rule 412 matches: {}", rule_412_matches.len());
        for m in &rule_412_matches {
            eprintln!(
                "  lines: {}-{}, tokens: {}-{}, coverage: {:.1}%",
                m.start_line, m.end_line, m.start_token, m.end_token, m.match_coverage
            );
        }

        // Check what matches are at lines 679-681
        let lines_679_681_matches: Vec<_> = aho_matches
            .iter()
            .filter(|m| m.start_line >= 679 && m.end_line <= 681)
            .collect();
        eprintln!(
            "\nMatches at lines 679-681: {}",
            lines_679_681_matches.len()
        );
        for m in &lines_679_681_matches {
            eprintln!(
                "  {} (rule: {}, lines: {}-{}, coverage: {:.1}%)",
                m.license_expression, m.rule_identifier, m.start_line, m.end_line, m.match_coverage
            );
        }

        // Also check the license detection engine
        use crate::license_detection::LicenseDetectionEngine;
        let engine = LicenseDetectionEngine::new(&rules_path).expect("Failed to create engine");
        let detections = engine.detect(&text, false).expect("Detection failed");

        eprintln!("\n=== FINAL DETECTIONS ({}) ===", detections.len());
        for d in &detections {
            let rule_id = d
                .matches
                .first()
                .map(|m| m.rule_identifier.as_str())
                .unwrap_or("unknown");
            let start_line = d.matches.first().map(|m| m.start_line).unwrap_or(0);
            let end_line = d.matches.first().map(|m| m.end_line).unwrap_or(0);
            let coverage = d.matches.first().map(|m| m.match_coverage).unwrap_or(0.0);
            let expr = d.license_expression.as_deref().unwrap_or("unknown");
            eprintln!(
                "  {} (rule: {}, lines: {}-{}, coverage: {:.1}%)",
                expr, rule_id, start_line, end_line, coverage
            );
        }

        let rule_412_detected = detections.iter().any(|d| {
            d.matches
                .iter()
                .any(|m| m.rule_identifier == "gpl-2.0-plus_412.RULE")
        });
        eprintln!("\nRule 412 in final detections: {}", rule_412_detected);
    }
}
