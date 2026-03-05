use scancode_rust::license_detection::index::LicenseIndex;
use scancode_rust::license_detection::query::Query;
use scancode_rust::license_detection::seq_match::candidates::compute_candidates_with_msets;
use std::path::PathBuf;

fn main() {
    let path = PathBuf::from("testdata/license-golden/datadriven/external/glc/CC-BY-SA-1.0.t1");
    let bytes = std::fs::read(&path).unwrap();
    let text = String::from_utf8_lossy(&bytes).into_owned();

    let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");

    println!("Loading license index...");
    let engine =
        scancode_rust::license_detection::LicenseDetectionEngine::new(&rules_path).unwrap();
    let index = engine.index();

    let query = Query::new(&text, index).unwrap();
    let whole_run = query.whole_query_run();

    println!("\nQuery token stats:");
    let tokens = whole_run.matchable_tokens();
    println!("  Total tokens: {}", tokens.len());
    println!(
        "  Unique tokens: {}",
        tokens
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
    );

    // Get the candidates
    let candidates = compute_candidates_with_msets(index, &whole_run, true, 50);

    println!("\n=== ALL CANDIDATES (sorted by rank) ===");
    for (i, c) in candidates.iter().enumerate() {
        println!("{:2}. {} (rid={})", i + 1, c.rule.license_expression, c.rid);
        println!(
            "    resemblance: {:.4}, containment: {:.4}, matched_len: {:.0}",
            c.score_vec_full.resemblance,
            c.score_vec_full.containment,
            c.score_vec_full.matched_length
        );
        println!(
            "    is_highly_resemblant: {}",
            c.score_vec_full.is_highly_resemblant
        );
    }

    // Find specific licenses
    let cc_by_sa: Vec<_> = candidates
        .iter()
        .filter(|c| c.rule.license_expression == "cc-by-sa-1.0")
        .collect();
    let cc_by_nc_sa: Vec<_> = candidates
        .iter()
        .filter(|c| c.rule.license_expression == "cc-by-nc-sa-1.0")
        .collect();

    println!("\n=== CC-BY-SA-1.0 candidates: {} ===", cc_by_sa.len());
    for c in &cc_by_sa {
        println!(
            "  rid={}, resemblance={:.4}, containment={:.4}",
            c.rid, c.score_vec_full.resemblance, c.score_vec_full.containment
        );
    }

    println!(
        "\n=== CC-BY-NC-SA-1.0 candidates: {} ===",
        cc_by_nc_sa.len()
    );
    for c in &cc_by_nc_sa {
        println!(
            "  rid={}, resemblance={:.4}, containment={:.4}",
            c.rid, c.score_vec_full.resemblance, c.score_vec_full.containment
        );
    }
}
