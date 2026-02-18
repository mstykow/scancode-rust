//! Debug test for CDDL/GPL Glassfish combined rule matching.

#[cfg(test)]
mod tests {
    use crate::license_detection::LicenseDetectionEngine;
    use crate::license_detection::index::token_sets::build_set_and_mset;
    use crate::license_detection::query::Query;
    use std::collections::HashSet;
    use std::path::PathBuf;

    #[test]
    fn verify_matchable_tokens_bug() {
        let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
        if !data_path.exists() {
            eprintln!("Reference data not available");
            return;
        }

        let engine = match LicenseDetectionEngine::new(&data_path) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Failed to create engine: {:?}", e);
                return;
            }
        };

        let text = match std::fs::read_to_string(
            "testdata/license-golden/datadriven/lic1/cddl-1.0_or_gpl-2.0-glassfish.txt",
        ) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Could not read test file: {}", e);
                return;
            }
        };

        let index = engine.index();

        // Create a query
        let query = match Query::new(&text, index) {
            Ok(q) => q,
            Err(e) => {
                eprintln!("Failed to create query: {}", e);
                return;
            }
        };

        let whole_run = query.whole_query_run();

        eprintln!("=== Current Rust Behavior ===");

        // Current: only HIGH matchables
        let matchable_tokens_high_only = whole_run.matchable_tokens();
        let token_ids_high_only: Vec<u16> = matchable_tokens_high_only
            .iter()
            .filter_map(|&tid| if tid >= 0 { Some(tid as u16) } else { None })
            .collect();
        let (set_high_only, _) = build_set_and_mset(&token_ids_high_only);

        eprintln!(
            "HIGH ONLY matchable_tokens count: {}",
            token_ids_high_only.len()
        );
        eprintln!("HIGH ONLY set size (unique): {}", set_high_only.len());

        // What Python does: all matchables (high OR low)
        let all_matchables_positions = whole_run.matchables(true); // include_low=true
        let token_ids_all: Vec<u16> = whole_run
            .tokens()
            .iter()
            .enumerate()
            .filter(|(pos, _)| all_matchables_positions.contains(pos))
            .map(|(_, &tid)| tid)
            .collect();
        let (set_all, _) = build_set_and_mset(&token_ids_all);

        eprintln!("\nALL matchables (high+low) count: {}", token_ids_all.len());
        eprintln!("ALL matchables set size (unique): {}", set_all.len());

        // Find glassfish rule
        let mut glassfish_rid: Option<usize> = None;
        for (rid, rule) in index.rules_by_rid.iter().enumerate() {
            if rule.identifier.contains("cddl-1.0_or_gpl-2.0-glassfish") {
                glassfish_rid = Some(rid);
                break;
            }
        }

        let rid = glassfish_rid.unwrap();
        let rule = &index.rules_by_rid[rid];
        let rule_set = index.sets_by_rid.get(&rid).unwrap();

        eprintln!("\n=== Rule Stats ===");
        eprintln!("rule.length_unique: {}", rule.length_unique);
        eprintln!("rule_set size: {}", rule_set.len());

        // Compute resemblance with HIGH ONLY
        let intersection_high: HashSet<u16> =
            set_high_only.intersection(rule_set).copied().collect();
        let union_high = set_high_only.len() + rule_set.len() - intersection_high.len();
        let resemblance_high = intersection_high.len() as f32 / union_high as f32;

        eprintln!("\n=== Resemblance with HIGH ONLY ===");
        eprintln!("intersection: {}", intersection_high.len());
        eprintln!("union: {}", union_high);
        eprintln!("resemblance: {:.3}", resemblance_high);
        eprintln!("is_highly_resemblant (>= 0.8): {}", resemblance_high >= 0.8);

        // Compute resemblance with ALL matchables
        let intersection_all: HashSet<u16> = set_all.intersection(rule_set).copied().collect();
        let union_all = set_all.len() + rule_set.len() - intersection_all.len();
        let resemblance_all = intersection_all.len() as f32 / union_all as f32;

        eprintln!("\n=== Resemblance with ALL MATCHABLES (Python behavior) ===");
        eprintln!("intersection: {}", intersection_all.len());
        eprintln!("union: {}", union_all);
        eprintln!("resemblance: {:.3}", resemblance_all);
        eprintln!("is_highly_resemblant (>= 0.8): {}", resemblance_all >= 0.8);
    }
}
