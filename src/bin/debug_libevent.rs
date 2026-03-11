use std::path::PathBuf;
use std::fs;
use scancode_rust::license_detection::LicenseDetectionEngine;
use scancode_rust::license_detection::aho_match::aho_match;
use scancode_rust::license_detection::query::{Query, PositionSpan};
use scancode_rust::utils::file::extract_text_for_detection;

fn main() {
    let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
    let engine = LicenseDetectionEngine::new(&data_path).expect("Failed to create engine");
    
    let test_file = PathBuf::from("testdata/license-golden/datadriven/lic3/libevent.LICENSE");
    let bytes = fs::read(&test_file).expect("Failed to read file");
    let (text, _) = extract_text_for_detection(&test_file, &bytes);
    
    // Create query
    let query = Query::new(&text, engine.index()).expect("Failed to create query");
    let whole_run = query.whole_query_run();
    
    // Get UNREFINED Aho matches (like Python)
    let aho_matches = aho_match(engine.index(), &whole_run);
    
    // Build matched_qspans from UNREFINED 100% coverage matches
    let matched_qspans: Vec<PositionSpan> = aho_matches.iter()
        .filter(|m| (m.match_coverage * 100.0).round() / 100.0 == 100.0 && m.end_token > m.start_token)
        .map(|m| PositionSpan::new(m.start_token, m.end_token - 1))
        .collect();
    
    println!("matched_qspans count: {}", matched_qspans.len());
    
    // Check if whole_run is matchable with matched_qspans
    let is_matchable = whole_run.is_matchable(false, &matched_qspans);
    println!("is_matchable (whole_run with matched_qspans): {}", is_matchable);
    
    // Check query_runs
    let query_runs = query.query_runs();
    println!("query_runs: {} runs", query_runs.len());
    
    // For each query_run, check is_matchable
    for (i, run) in query_runs.iter().enumerate() {
        let run_is_matchable = run.is_matchable(false, &matched_qspans);
        println!("  run[{}] is_matchable: {}", i, run_is_matchable);
    }
}
