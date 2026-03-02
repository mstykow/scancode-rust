//! Debug License Detection Pipeline for Rust
//!
//! This binary provides detailed instrumentation of the Rust license detection
//! pipeline for debugging purposes. It mirrors the Python debug script.
//!
//! Usage:
//!     cargo run --features debug-pipeline --bin debug_license_detection -- <file_path>
//!
//! Example:
//!     cargo run --features debug-pipeline --bin debug_license_detection -- testdata/mit.txt

use std::path::PathBuf;

use anyhow::Result;
use scancode_rust::license_detection::{
    aho_match, compute_candidates_with_msets, create_detection_from_group, group_matches_by_region, hash_match,
    merge_overlapping_matches, post_process_detections, refine_aho_matches, sort_matches_by_line,
    refine_matches, refine_matches_without_false_positive_filter, seq_match_with_candidates,
    spdx_lid_match, split_weak_matches, LicenseDetectionEngine, MAX_NEAR_DUPE_CANDIDATES,
};

#[cfg(feature = "debug-pipeline")]
use scancode_rust::license_detection::{
    filter_below_rule_minimum_coverage_debug_only, filter_contained_matches_debug_only,
    filter_false_positive_matches_debug_only, filter_invalid_matches_to_single_word_gibberish_debug_only,
    filter_matches_missing_required_phrases_debug_only, filter_matches_to_spurious_single_token_debug_only,
    filter_short_matches_scattered_on_too_many_lines_debug_only, filter_spurious_matches_debug_only,
    filter_too_short_matches_debug_only,
};

fn print_section(title: &str) {
    println!("\n{}", "=".repeat(80));
    println!(" {}", title);
    println!("{}", "=".repeat(80));
}

#[cfg(feature = "debug-pipeline")]
fn print_subsection(title: &str) {
    println!("\n{}", "-".repeat(80));
    println!(" {}", title);
    println!("{}", "-".repeat(80));
}

fn format_match(m: &scancode_rust::license_detection::models::LicenseMatch) -> String {
    format!(
        "  Rule: {} (license: {})\n  Score: {:.1}%, Coverage: {:.1}%\n  Lines: {}-{}, Tokens: {}-{}",
        m.rule_identifier,
        m.license_expression,
        m.score * 100.0,
        m.match_coverage,
        m.start_line,
        m.end_line,
        m.start_token,
        m.end_token,
    )
}

fn run_debug_pipeline(file_path: &str) -> Result<()> {
    let path = PathBuf::from(file_path);
    if !path.exists() {
        anyhow::bail!("File not found: {}", file_path);
    }

    let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
    let engine = LicenseDetectionEngine::new(&rules_path)?;
    
    // Use the same text extraction as the golden tests
    let bytes = std::fs::read(&path)?;
    let text = scancode_rust::utils::file_text::extract_text_for_detection(&bytes, &path)
        .map(|ft| ft.text)
        .unwrap_or_else(|| String::from_utf8_lossy(&bytes).into_owned());
    let index = engine.index();
    
    print_section(&format!("LICENSE DETECTION DEBUG: {}", file_path));
    
    // Stage 1: Query building
    print_section("STAGE 1: QUERY BUILDING");
    let query = scancode_rust::license_detection::query::Query::new(&text, index)?;
    println!("Total tokens: {}", query.tokens.len());
    println!("Total lines tracked: {}", query.line_by_pos.len());
    println!("Query runs: {}", query.query_runs().len());
    
    let whole_run = query.whole_query_run();
    println!("Whole run start: {}, end: {:?}", whole_run.start, whole_run.end);
    println!("Matchables count: {}", whole_run.matchables(false).len());
    
    // Stage 2: Hash matching
    print_section("STAGE 2: HASH MATCHING");
    let hash_matches = hash_match(index, &whole_run);
    if !hash_matches.is_empty() {
        println!("HASH MATCH FOUND: {} match(es)", hash_matches.len());
        for m in &hash_matches {
            println!("  - {} (license: {})", m.rule_identifier, m.license_expression);
        }
        println!("\n*** HASH MATCH FOUND - stopping early ***");
        return Ok(());
    }
    println!("No hash matches found");
    
    // Stage 3: SPDX-LID matching
    print_section("STAGE 3: SPDX IDENTIFIER MATCHING");
    let spdx_matches = spdx_lid_match(index, &query);
    println!("SPDX ID MATCHES: {}", spdx_matches.len());
    for m in &spdx_matches {
        println!("  - {} (license: {})", m.rule_identifier, m.license_expression);
    }
    
    // Stage 4: Aho-Corasick matching
    print_section("STAGE 4: EXACT MATCHING (Aho-Corasick)");
    let raw_aho_matches = aho_match(index, &whole_run);
    // Python's get_exact_matches() calls refine_matches with merge=False (index.py:691-696)
    let aho_matches = refine_aho_matches(index, raw_aho_matches.clone(), &query);
    println!("EXACT MATCHES: {} (raw: {})", aho_matches.len(), raw_aho_matches.len());
    for m in aho_matches.iter().take(10) {
        println!("{}", format_match(m));
    }
    if aho_matches.len() > 10 {
        println!("  ... and {} more", aho_matches.len() - 10);
    }
    
    // Stage 5: Sequence matching
    print_section("STAGE 5a: CANDIDATE SELECTION");
    let candidates = compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);
    println!("NEAR-DUPE CANDIDATES: {}", candidates.len());
    for (i, c) in candidates.iter().take(10).enumerate() {
        println!("  {}. {} (score: {:.4})", i + 1, c.rule.identifier, c.score_vec_rounded.resemblance);
    }
    
    print_section("STAGE 5: SEQUENCE MATCHING");
    let seq_matches = if !candidates.is_empty() {
        seq_match_with_candidates(index, &whole_run, &candidates)
    } else {
        Vec::new()
    };
    println!("SEQUENCE MATCHES: {}", seq_matches.len());
    for m in seq_matches.iter().take(10) {
        println!("{}", format_match(m));
    }
    
    // Combine all matches
    // Python merges each matcher's results before adding to matches (index.py:1040)
    let mut all_matches = Vec::new();
    all_matches.extend(spdx_matches.clone());
    all_matches.extend(aho_matches.clone());
    // Merge sequence matches ONCE (like Python's approx matcher and Rust's engine.detect)
    let merged_seq = merge_overlapping_matches(&seq_matches);
    all_matches.extend(merged_seq);
    
    // Stage 6: Merging
    print_section("STAGE 6: MATCH MERGING");
    println!("Input matches: {}", all_matches.len());
    let merged = merge_overlapping_matches(&all_matches);
    println!("After merge: {}", merged.len());
    
    // Stage 7: Refinement (with debug detail if feature enabled)
    print_section("STAGE 7: MATCH REFINEMENT");
    
    #[cfg(feature = "debug-pipeline")]
    {
        println!("\n=== Individual Filter Stages ===");
        
        print_subsection("filter_too_short_matches");
        let after_short = filter_too_short_matches_debug_only(index, &merged);
        println!("  Before: {}, After: {}", merged.len(), after_short.len());
        
        print_subsection("filter_spurious_matches");
        let after_spurious = filter_spurious_matches_debug_only(&after_short, &query);
        println!("  Before: {}, After: {}", after_short.len(), after_spurious.len());
        
        print_subsection("filter_below_rule_minimum_coverage");
        let after_coverage = filter_below_rule_minimum_coverage_debug_only(index, &after_spurious);
        println!("  Before: {}, After: {}", after_spurious.len(), after_coverage.len());
        
        print_subsection("filter_short_matches_scattered_on_too_many_lines");
        let after_scattered = filter_short_matches_scattered_on_too_many_lines_debug_only(index, &after_coverage);
        println!("  Before: {}, After: {}", after_coverage.len(), after_scattered.len());
        
        print_subsection("filter_matches_missing_required_phrases");
        let (after_phrases, discarded_phrases) = filter_matches_missing_required_phrases_debug_only(index, &after_scattered, &query);
        println!("  Kept: {}, Discarded: {}", after_phrases.len(), discarded_phrases.len());
        
        print_subsection("filter_contained_matches");
        let (after_contained, discarded_contained) = filter_contained_matches_debug_only(&after_phrases);
        println!("  Kept: {}, Discarded: {}", after_contained.len(), discarded_contained.len());
        
        print_subsection("filter_false_positive_matches");
        let after_fp = filter_false_positive_matches_debug_only(index, &after_contained);
        println!("  Before: {}, After: {}", after_contained.len(), after_fp.len());
    }
    
    let refined = refine_matches_without_false_positive_filter(index, all_matches, &query);
    
    // Note: We do NOT call split_weak_matches() here because that's only done
    // when unknown_licenses=True. The default behavior (matching Python with
    // unknown_licenses=False) is to pass all matches directly to refine_matches.
    // See Python: index.py:1079-1118
    
    // Stage 8: Detection assembly
    print_section("STAGE 8: DETECTION ASSEMBLY");
    let mut final_refined = refine_matches(index, refined, &query);
    sort_matches_by_line(&mut final_refined);
    let groups = group_matches_by_region(&final_refined);
    println!("Groups: {}", groups.len());
    
    let detections: Vec<_> = groups.iter().map(|g| create_detection_from_group(g)).collect();
    let detections = post_process_detections(detections, 0.0);
    
    println!("\nFINAL DETECTIONS: {}", detections.len());
    for (i, d) in detections.iter().enumerate() {
        println!("\nDetection {}:", i + 1);
        if let Some(expr) = &d.license_expression {
            println!("  License expression: {}", expr);
        }
        if let Some(spdx) = &d.license_expression_spdx {
            println!("  SPDX expression: {}", spdx);
        }
        println!("  Matches: {}", d.matches.len());
    }
    
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Debug License Detection Pipeline");
        eprintln!();
        eprintln!("Usage: cargo run --features debug-pipeline --bin debug_license_detection -- <file_path>");
        eprintln!();
        eprintln!("Example:");
        eprintln!("  cargo run --features debug-pipeline --bin debug_license_detection -- testdata/mit.txt");
        std::process::exit(1);
    }
    
    if let Err(e) = run_debug_pipeline(&args[1]) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
