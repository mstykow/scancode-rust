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
    aho_match, compute_candidates_with_msets, create_detection_from_group, group_matches_by_region,
    hash_match, merge_overlapping_matches, post_process_detections, refine_aho_matches,
    refine_matches, refine_matches_without_false_positive_filter, seq_match_with_candidates,
    sort_matches_by_line, spdx_lid_match, split_weak_matches, LicenseDetectionEngine,
    MAX_NEAR_DUPE_CANDIDATES,
};

#[cfg(feature = "debug-pipeline")]
use scancode_rust::license_detection::{
    filter_below_rule_minimum_coverage_debug_only, filter_contained_matches_debug_only,
    filter_false_positive_matches_debug_only,
    filter_invalid_matches_to_single_word_gibberish_debug_only,
    filter_matches_missing_required_phrases_debug_only,
    filter_matches_to_spurious_single_token_debug_only,
    filter_short_matches_scattered_on_too_many_lines_debug_only,
    filter_spurious_matches_debug_only, filter_too_short_matches_debug_only,
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
    println!(
        "Whole run start: {}, end: {:?}",
        whole_run.start, whole_run.end
    );
    println!("Matchables count: {}", whole_run.matchables(false).len());

    // Stage 2: Hash matching
    print_section("STAGE 2: HASH MATCHING");
    let hash_matches = hash_match(index, &whole_run);
    if !hash_matches.is_empty() {
        println!("HASH MATCH FOUND: {} match(es)", hash_matches.len());
        for m in &hash_matches {
            println!(
                "  - {} (license: {})",
                m.rule_identifier, m.license_expression
            );
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
        println!(
            "  - {} (license: {})",
            m.rule_identifier, m.license_expression
        );
    }

    // Stage 4: Aho-Corasick matching
    print_section("STAGE 4: EXACT MATCHING (Aho-Corasick)");
    let raw_aho_matches = aho_match(index, &whole_run);
    // Python's get_exact_matches() calls refine_matches with merge=False (index.py:691-696)
    let aho_matches = refine_aho_matches(index, raw_aho_matches.clone(), &query);
    println!(
        "EXACT MATCHES: {} (raw: {})",
        aho_matches.len(),
        raw_aho_matches.len()
    );
    for m in aho_matches.iter().take(10) {
        println!("{}", format_match(m));
    }
    if aho_matches.len() > 10 {
        println!("  ... and {} more", aho_matches.len() - 10);
    }

    // Stage 5: Sequence matching
    print_section("STAGE 5a: CANDIDATE SELECTION");
    let candidates =
        compute_candidates_with_msets(index, &whole_run, true, MAX_NEAR_DUPE_CANDIDATES);

    // Get query multiset length for context
    let query_tokens = whole_run.matchable_tokens();
    let query_token_ids: Vec<u16> = query_tokens
        .iter()
        .filter_map(|&tid| if tid >= 0 { Some(tid as u16) } else { None })
        .collect();
    let (_, query_mset) =
        scancode_rust::license_detection::index::token_sets::build_set_and_mset(&query_token_ids);
    let qset_len: usize = query_mset.values().sum();
    println!("Query multiset length (qset_len): {}", qset_len);

    println!("NEAR-DUPE CANDIDATES: {}", candidates.len());
    for (i, c) in candidates.iter().take(10).enumerate() {
        let rule_len = if c.score_vec_full.containment > 0.0 {
            (c.score_vec_full.matched_length / c.score_vec_full.containment) as usize
        } else {
            0
        };
        let union_len = (c.score_vec_full.matched_length as usize + rule_len
            - c.score_vec_full.matched_length as usize);
        println!(
            "  {}. {} (resemblance: {:.4}, containment: {:.4}, matched_len: {:.0}, rule_len: {}, union_len: {})",
            i + 1,
            c.rule.identifier,
            c.score_vec_full.resemblance,
            c.score_vec_full.containment,
            c.score_vec_full.matched_length,
            rule_len,
            union_len
        );
    }

    print_section("STAGE 5: SEQUENCE MATCHING");
    let seq_matches = if !candidates.is_empty() {
        seq_match_with_candidates(index, &whole_run, &candidates)
    } else {
        Vec::new()
    };
    println!("SEQUENCE MATCHES: {}", seq_matches.len());

    // Find CC-BY-SA and CC-BY-NC-SA matches specifically
    let sa_matches: Vec<_> = seq_matches
        .iter()
        .filter(|m| {
            m.rule_identifier.contains("cc-by-sa-2.0")
                || m.rule_identifier.contains("cc-by-nc-sa-2.0")
        })
        .collect();

    for m in sa_matches.iter().take(30) {
        println!(
            "{} matched_len={} coverage={:.1}%",
            format_match(m),
            m.matched_length,
            m.match_coverage
        );
    }
    println!(
        "\\n(Showing only cc-by-sa and cc-by-nc-sa matches, {} total)",
        sa_matches.len()
    );

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

    // Show SA and NC-SA matches after merge
    let sa_merged: Vec<_> = merged
        .iter()
        .filter(|m| {
            m.rule_identifier.contains("cc-by-sa-2.0")
                || m.rule_identifier.contains("cc-by-nc-sa-2.0")
        })
        .collect();
    println!(
        "\\nCC-BY-SA and CC-BY-NC-SA matches after merge ({} total):",
        sa_merged.len()
    );
    for m in sa_merged.iter() {
        println!("  {} lines {}-{} len={} hilen={} coverage={:.1}% match_coverage={:.1}% resemblance={:.4}",
            m.rule_identifier, m.start_line, m.end_line,
            m.matched_length, m.hilen, m.match_coverage, m.match_coverage, m.candidate_resemblance);
    }

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
        println!(
            "  Before: {}, After: {}",
            after_short.len(),
            after_spurious.len()
        );

        print_subsection("filter_below_rule_minimum_coverage");
        let after_coverage = filter_below_rule_minimum_coverage_debug_only(index, &after_spurious);
        println!(
            "  Before: {}, After: {}",
            after_spurious.len(),
            after_coverage.len()
        );

        print_subsection("filter_short_matches_scattered_on_too_many_lines");
        let after_scattered =
            filter_short_matches_scattered_on_too_many_lines_debug_only(index, &after_coverage);
        println!(
            "  Before: {}, After: {}",
            after_coverage.len(),
            after_scattered.len()
        );

        print_subsection("filter_matches_missing_required_phrases");
        let (after_phrases, discarded_phrases) =
            filter_matches_missing_required_phrases_debug_only(index, &after_scattered, &query);
        println!(
            "  Kept: {}, Discarded: {}",
            after_phrases.len(),
            discarded_phrases.len()
        );

        print_subsection("filter_contained_matches");
        println!("  Input matches (sorted by qstart, -hilen, -matched_length, matcher_order):");
        let mut sorted_input: Vec<_> = after_phrases.iter().collect();
        sorted_input.sort_by(|a, b| {
            a.qstart()
                .cmp(&b.qstart())
                .then_with(|| b.hilen.cmp(&a.hilen))
                .then_with(|| b.matched_length.cmp(&a.matched_length))
                .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
        });
        for (idx, m) in sorted_input.iter().take(10).enumerate() {
            println!(
                "    [{}] {} (license: {}, qstart={}, end_token={}, hilen={}, matched_len={}, matcher_order={}, coverage={:.4}, qspan_positions={})",
                idx,
                m.rule_identifier,
                m.license_expression,
                m.qstart(),
                m.end_token,
                m.hilen(),
                m.matched_length,
                m.matcher_order(),
                m.match_coverage,
                m.qspan_positions.as_ref().map(|p| p.len()).unwrap_or(0)
            );
        }
        println!("  ... and {} more", sorted_input.len().saturating_sub(10));

        // Check if top two matches have equal qspan
        if sorted_input.len() >= 2 {
            let m0 = &sorted_input[0];
            let m1 = &sorted_input[1];
            println!("\n  Checking if first two matches have equal qspan:");
            println!(
                "    m0.qstart() == m1.qstart(): {} == {} = {}",
                m0.qstart(),
                m1.qstart(),
                m0.qstart() == m1.qstart()
            );
            println!(
                "    m0.end_token == m1.end_token: {} == {} = {}",
                m0.end_token,
                m1.end_token,
                m0.end_token == m1.end_token
            );
            println!("    m0.qcontains(m1): {}", m0.qcontains(m1));
            println!("    m1.qcontains(m0): {}", m1.qcontains(m0));

            // Check if qspan positions are equal sets
            if let (Some(p0), Some(p1)) = (&m0.qspan_positions, &m1.qspan_positions) {
                let set0: std::collections::HashSet<_> = p0.iter().copied().collect();
                let set1: std::collections::HashSet<_> = p1.iter().copied().collect();
                println!("    qspan_positions sets equal: {}", set0 == set1);
                println!(
                    "    m0 qspan_positions (first 10): {:?}",
                    &p0[..p0.len().min(10)]
                );
                println!(
                    "    m1 qspan_positions (first 10): {:?}",
                    &p1[..p1.len().min(10)]
                );
            }
        }

        // Check if bsd-new and bsd-simplified have different qspan sets
        let bsd_new = sorted_input
            .iter()
            .find(|m| m.rule_identifier == "bsd-new_1319.RULE");
        let bsd_simp = sorted_input
            .iter()
            .find(|m| m.rule_identifier == "bsd-simplified_204.RULE");
        if let (Some(bn), Some(bs)) = (bsd_new, bsd_simp) {
            println!("\n  Checking bsd-new vs bsd-simplified_204:");
            println!(
                "    bsd-new.qcontains(bsd-simplified_204): {}",
                bn.qcontains(bs)
            );
            println!(
                "    bsd-simplified_204.qcontains(bsd-new): {}",
                bs.qcontains(bn)
            );
            if let (Some(pn), Some(ps)) = (&bn.qspan_positions, &bs.qspan_positions) {
                let set_n: std::collections::HashSet<_> = pn.iter().copied().collect();
                let set_s: std::collections::HashSet<_> = ps.iter().copied().collect();
                let intersection: Vec<_> = set_n.intersection(&set_s).copied().collect();
                println!("    bsd-new positions count: {}", pn.len());
                println!("    bsd-simplified_204 positions count: {}", ps.len());
                println!("    Intersection count: {}", intersection.len());
            }
        }

        let (after_contained, discarded_contained) =
            filter_contained_matches_debug_only(&after_phrases);
        println!(
            "\n  Kept: {}, Discarded: {}",
            after_contained.len(),
            discarded_contained.len()
        );
        println!("  Kept matches:");
        for m in &after_contained {
            println!(
                "    - {} (license: {}, lines {}-{}, qstart={}, qend={}, start_token={}, end_token={}, hilen={}, coverage={:.4})",
                m.rule_identifier,
                m.license_expression,
                m.start_line,
                m.end_line,
                m.qstart(),
                m.end_token,
                m.start_token,
                m.end_token,
                m.hilen(),
                m.match_coverage
            );
        }
        println!("  First 5 discarded matches:");
        for m in discarded_contained.iter().take(5) {
            println!(
                "    - {} (license: {}, lines {}-{}, qstart={}, qend={}, start_token={}, end_token={}, hilen={}, coverage={:.4})",
                m.rule_identifier,
                m.license_expression,
                m.start_line,
                m.end_line,
                m.qstart(),
                m.end_token,
                m.start_token,
                m.end_token,
                m.hilen(),
                m.match_coverage
            );
        }

        print_subsection("filter_false_positive_matches");
        let after_fp = filter_false_positive_matches_debug_only(index, &after_contained);
        println!(
            "  Before: {}, After: {}",
            after_contained.len(),
            after_fp.len()
        );
        println!("  Kept matches:");
        for m in &after_fp {
            println!(
                "    - {} (license: {}, lines {}-{}, cand_resemblance={:.4}, cand_containment={:.4}, rule_len={})",
                m.rule_identifier,
                m.license_expression,
                m.start_line,
                m.end_line,
                m.candidate_resemblance,
                m.candidate_containment,
                m.rule_length
            );
        }
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
    println!("Final refined matches: {}", final_refined.len());
    for m in &final_refined {
        println!(
            "  - {} (license: {}, lines {}-{})",
            m.rule_identifier, m.license_expression, m.start_line, m.end_line
        );
    }
    let groups = group_matches_by_region(&final_refined);
    println!("Groups: {}", groups.len());

    let detections: Vec<_> = groups
        .iter()
        .map(|g| create_detection_from_group(g))
        .collect();
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
        for m in &d.matches {
            println!(
                "    - {} (license: {})",
                m.rule_identifier, m.license_expression
            );
        }
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Debug License Detection Pipeline");
        eprintln!();
        eprintln!(
            "Usage: cargo run --features debug-pipeline --bin debug_license_detection -- <file_path>"
        );
        eprintln!();
        eprintln!("Example:");
        eprintln!(
            "  cargo run --features debug-pipeline --bin debug_license_detection -- testdata/mit.txt"
        );
        std::process::exit(1);
    }

    if let Err(e) = run_debug_pipeline(&args[1]) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
