//! Debug test for ar-ER.js.map duplicate MIT detection issue.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::license_detection::aho_match;
    use crate::license_detection::index::build_index;
    use crate::license_detection::match_refine::{
        filter_contained_matches, filter_overlapping_matches, merge_overlapping_matches,
    };
    use crate::license_detection::query::Query;
    use crate::license_detection::rules::{
        load_licenses_from_directory, load_rules_from_directory,
    };
    use crate::license_detection::tokenize::tokenize;

    #[test]
    fn test_ar_er_debug() {
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

        let text = std::fs::read_to_string("testdata/license-golden/datadriven/lic2/ar-ER.js.map")
            .expect("Failed to read ar-ER.js.map");

        eprintln!("\n=== TEXT ANALYSIS ===");
        eprintln!("Text length: {} bytes", text.len());
        eprintln!("Text has {} lines", text.lines().count());
        for (i, line) in text.lines().take(5).enumerate() {
            eprintln!("Line {}: {} chars", i + 1, line.len());
        }

        let query = Query::new(&text, &index).expect("Query creation failed");
        let run = query.whole_query_run();

        eprintln!("\n=== TOKEN ANALYSIS ===");
        eprintln!("Total tokens: {}", run.tokens().len());

        // Run aho matching
        let matches = aho_match::aho_match(&index, &run);

        eprintln!("\n=== RAW AHO MATCHES ({}) ===", matches.len());
        for (i, m) in matches.iter().enumerate() {
            eprintln!(
                "{}: {} (rule: {}, start_token: {}, end_token: {}, lines: {}-{}, coverage: {:.1}%, len: {}, hilen: {})",
                i, m.license_expression, m.rule_identifier, m.start_token, m.end_token,
                m.start_line, m.end_line, m.match_coverage, m.matched_length, m.hilen
            );
        }

        // Run merge
        let merged = merge_overlapping_matches(&matches);
        eprintln!("\n=== AFTER MERGE ({}) ===", merged.len());
        for (i, m) in merged.iter().enumerate() {
            eprintln!(
                "{}: {} (rule: {}, start_token: {}, end_token: {}, lines: {}-{}, coverage: {:.1}%, len: {}, hilen: {})",
                i, m.license_expression, m.rule_identifier, m.start_token, m.end_token,
                m.start_line, m.end_line, m.match_coverage, m.matched_length, m.hilen
            );
        }

        // Check containment details
        if merged.len() >= 2 {
            eprintln!("\n=== CONTAINMENT ANALYSIS ===");
            let m0 = &merged[0];
            let m1 = &merged[1];

            eprintln!("Match 0 qstart: {}, qend: {}", m0.qstart(), m0.end_token);
            eprintln!("Match 1 qstart: {}, qend: {}", m1.qstart(), m1.end_token);
            eprintln!("m0.qcontains(m1): {}", m0.qcontains(m1));
            eprintln!("m1.qcontains(m0): {}", m1.qcontains(m0));
            eprintln!("m0.qstart() == m1.qstart(): {}", m0.qstart() == m1.qstart());
            eprintln!(
                "m0.end_token == m1.end_token: {}",
                m0.end_token == m1.end_token
            );
        }

        // Run filter_contained
        let (non_contained, discarded) = filter_contained_matches(&merged);
        eprintln!("\n=== AFTER FILTER_CONTAINED ===");
        eprintln!("Kept ({}):", non_contained.len());
        for (i, m) in non_contained.iter().enumerate() {
            eprintln!(
                "{}: {} (rule: {}, start_token: {}, end_token: {}, lines: {}-{})",
                i,
                m.license_expression,
                m.rule_identifier,
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line
            );
        }
        eprintln!("Discarded ({}):", discarded.len());
        for (i, m) in discarded.iter().enumerate() {
            eprintln!(
                "{}: {} (rule: {}, start_token: {}, end_token: {}, lines: {}-{})",
                i,
                m.license_expression,
                m.rule_identifier,
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line
            );
        }

        // Run filter_overlapping
        let (kept, discarded2) = filter_overlapping_matches(non_contained, &index);
        eprintln!("\n=== AFTER FILTER_OVERLAPPING ===");
        eprintln!("Kept ({}):", kept.len());
        for (i, m) in kept.iter().enumerate() {
            eprintln!(
                "{}: {} (rule: {}, start_token: {}, end_token: {}, lines: {}-{})",
                i,
                m.license_expression,
                m.rule_identifier,
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line
            );
        }
        eprintln!("Discarded ({}):", discarded2.len());
        for (i, m) in discarded2.iter().enumerate() {
            eprintln!(
                "{}: {} (rule: {}, start_token: {}, end_token: {}, lines: {}-{})",
                i,
                m.license_expression,
                m.rule_identifier,
                m.start_token,
                m.end_token,
                m.start_line,
                m.end_line
            );
        }

        eprintln!("\n=== EXPECTED ===");
        eprintln!("Python finds: 1 MIT match (mit_129.RULE at lines 5-6)");
        eprintln!(
            "Rust finds: {} MIT matches",
            kept.iter()
                .filter(|m| m.license_expression == "mit")
                .count()
        );
    }

    #[test]
    fn test_mit_129_in_index() {
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

        eprintln!("\n=== CHECKING IF MIT_129 IS IN INDEX ===");

        // Find MIT_129 in rules_by_rid
        let mit_129_rid = index
            .rules_by_rid
            .iter()
            .position(|r| r.identifier == "mit_129.RULE");

        if let Some(rid) = mit_129_rid {
            eprintln!("MIT_129.RULE found at rid={}", rid);
            let rule = &index.rules_by_rid[rid];
            eprintln!("  license_expression: {}", rule.license_expression);
            eprintln!("  is_false_positive: {}", rule.is_false_positive);
            eprintln!("  is_required_phrase: {}", rule.is_required_phrase);
            eprintln!("  is_deprecated: {}", rule.is_deprecated);
            eprintln!("  tokens ({}): {:?}", rule.tokens.len(), rule.tokens);
            eprintln!("  is_continuous: {}", rule.is_continuous);
            eprintln!("  is_small: {}", rule.is_small);
            eprintln!("  is_tiny: {}", rule.is_tiny);

            // Check if it's in regular_rids
            eprintln!("  in regular_rids: {}", index.regular_rids.contains(&rid));
            eprintln!(
                "  in false_positive_rids: {}",
                index.false_positive_rids.contains(&rid)
            );

            // Check if it's in the automaton patterns
            // Find pattern_id that maps to this rid
            let pattern_id = index.pattern_id_to_rid.iter().position(|&prid| prid == rid);
            eprintln!("  pattern_id in automaton: {:?}", pattern_id);

            // Check if tokens are empty
            if rule.tokens.is_empty() {
                eprintln!(
                    "  WARNING: Rule has empty tokens! This would NOT be added to automaton."
                );
            }

            // Check required_phrase_spans
            eprintln!("  required_phrase_spans: {:?}", rule.required_phrase_spans);
        } else {
            eprintln!("MIT_129.RULE NOT found in index!");
        }

        // Also check MIT_131 and MIT_132
        for rule_id in &["mit_131.RULE", "mit_132.RULE"] {
            let rid = index
                .rules_by_rid
                .iter()
                .position(|r| r.identifier == *rule_id);
            if let Some(rid) = rid {
                eprintln!("\n{} found at rid={}", rule_id, rid);
                let rule = &index.rules_by_rid[rid];
                eprintln!("  tokens ({}): {:?}", rule.tokens.len(), rule.tokens);
                let pattern_id = index.pattern_id_to_rid.iter().position(|&prid| prid == rid);
                eprintln!("  pattern_id in automaton: {:?}", pattern_id);
            }
        }
    }

    #[test]
    fn test_mit_129_aho_matching() {
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

        // Get MIT_129 token IDs
        let mit_129_rid = index
            .rules_by_rid
            .iter()
            .position(|r| r.identifier == "mit_129.RULE")
            .unwrap();
        let mit_129_tokens = index.tids_by_rid[mit_129_rid].clone();

        eprintln!("\n=== MIT_129 TOKENS ===");
        eprintln!("MIT_129 token IDs: {:?}", mit_129_tokens);

        // Manually encode and search in automaton
        fn tokens_to_bytes(tokens: &[u16]) -> Vec<u8> {
            tokens.iter().flat_map(|t| t.to_le_bytes()).collect()
        }

        let mit_129_bytes = tokens_to_bytes(&mit_129_tokens);
        eprintln!(
            "MIT_129 encoded bytes ({}): first 20 = {:?}",
            mit_129_bytes.len(),
            &mit_129_bytes[..20.min(mit_129_bytes.len())]
        );

        // Now get query tokens from ar-ER.js.map
        let text = std::fs::read_to_string("testdata/license-golden/datadriven/lic2/ar-ER.js.map")
            .expect("Failed to read ar-ER.js.map");

        let query = Query::new(&text, &index).expect("Query creation failed");
        let run = query.whole_query_run();
        let query_tokens = run.tokens().to_vec();

        eprintln!("\n=== QUERY TOKENS ===");
        eprintln!("Query token count: {}", query_tokens.len());
        eprintln!("Query token IDs: {:?}", query_tokens);

        // Check if MIT_129 tokens are a prefix of query tokens
        let query_bytes = tokens_to_bytes(&query_tokens);

        eprintln!("\n=== SEARCHING FOR MIT_129 IN QUERY ===");

        // Directly search for MIT_129 pattern in query bytes
        use aho_corasick::AhoCorasick;
        let pattern = mit_129_bytes.clone();
        let ac = AhoCorasick::new(&[pattern.as_slice()]).unwrap();
        let matches: Vec<_> = ac.find_iter(&query_bytes).collect();
        eprintln!(
            "Direct Aho-Corasick matches for MIT_129: {} matches",
            matches.len()
        );
        for m in &matches {
            eprintln!("  Match: start={}, end={}", m.start(), m.end());
        }

        // Now check the main automaton
        eprintln!("\n=== MAIN AUTOMATON TEST ===");
        let main_matches: Vec<_> = index
            .rules_automaton
            .find_overlapping_iter(&query_bytes)
            .collect();
        eprintln!("Main automaton found {} total matches", main_matches.len());

        // Find matches that correspond to MIT_129 (pattern_id 26363)
        let mit_129_matches: Vec<_> = main_matches
            .iter()
            .filter(|m| index.pattern_id_to_rid.get(m.pattern().as_usize()) == Some(&mit_129_rid))
            .collect();
        eprintln!(
            "Matches for MIT_129 (rid={}): {} matches",
            mit_129_rid,
            mit_129_matches.len()
        );

        // Find matches for MIT_131
        let mit_131_rid = index
            .rules_by_rid
            .iter()
            .position(|r| r.identifier == "mit_131.RULE")
            .unwrap();
        let mit_131_matches: Vec<_> = main_matches
            .iter()
            .filter(|m| index.pattern_id_to_rid.get(m.pattern().as_usize()) == Some(&mit_131_rid))
            .collect();
        eprintln!(
            "Matches for MIT_131 (rid={}): {} matches",
            mit_131_rid,
            mit_131_matches.len()
        );

        // Check if query tokens match MIT_129 tokens exactly at some position
        eprintln!("\n=== MANUAL TOKEN COMPARISON ===");
        if query_tokens.len() >= mit_129_tokens.len() {
            let mut found_positions = vec![];
            for i in 0..=query_tokens.len() - mit_129_tokens.len() {
                if query_tokens[i..i + mit_129_tokens.len()] == mit_129_tokens[..] {
                    found_positions.push(i);
                }
            }
            eprintln!(
                "MIT_129 token sequence found at positions: {:?}",
                found_positions
            );
        }

        // Check MIT_131 tokens
        let mit_131_tokens = index.tids_by_rid[mit_131_rid].clone();
        if query_tokens.len() >= mit_131_tokens.len() {
            let mut found_positions = vec![];
            for i in 0..=query_tokens.len() - mit_131_tokens.len() {
                if query_tokens[i..i + mit_131_tokens.len()] == mit_131_tokens[..] {
                    found_positions.push(i);
                }
            }
            eprintln!(
                "MIT_131 token sequence found at positions: {:?}",
                found_positions
            );
        }
    }

    #[test]
    fn test_token_8579_mystery() {
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

        eprintln!("\n=== INVESTIGATING TOKEN 8579 ===");

        // Find what token 8579 corresponds to by scanning the dictionary
        let token_8579: Option<String> = index
            .dictionary
            .tokens_to_ids()
            .find(|(_, id)| **id == 8579)
            .map(|(s, _)| s.clone());
        eprintln!("Token 8579 -> {:?}", token_8579);

        // Get MIT_129 rule text
        let mit_129_rid = index
            .rules_by_rid
            .iter()
            .position(|r| r.identifier == "mit_129.RULE")
            .unwrap();
        let mit_129_rule = &index.rules_by_rid[mit_129_rid];
        eprintln!("\nMIT_129 rule text:\n{}", mit_129_rule.text);

        // Get the actual sourcesContent from ar-ER.js.map
        let text = std::fs::read_to_string("testdata/license-golden/datadriven/lic2/ar-ER.js.map")
            .expect("Failed to read ar-ER.js.map");

        // Extract sourcesContent from JSON
        let sources_content_start = text
            .find("\"sourcesContent\":[\"")
            .map(|i| i + "\"sourcesContent\":[\"".len());
        let sources_content_end = text.find("\"]").or_else(|| text.find("\",\""));

        if let (Some(start), Some(end)) = (sources_content_start, sources_content_end) {
            let sources_content = &text[start..end];
            eprintln!("\nSourcesContent ({} chars):", sources_content.len());
            eprintln!("{}", sources_content);

            // Find the MIT notice
            if let Some(mit_start) = sources_content.find("Use of this source") {
                let mit_end = sources_content[mit_start..]
                    .find("*/")
                    .map(|i| mit_start + i)
                    .unwrap_or(sources_content.len());
                let mit_notice = &sources_content[mit_start..mit_end.min(sources_content.len())];
                eprintln!("\nMIT notice section:");
                eprintln!("{}", mit_notice);

                // Check for asterisks
                eprintln!("\n=== ASTERISK ANALYSIS ===");
                for (i, c) in mit_notice.char_indices() {
                    if c == '*' {
                        eprintln!(
                            "Asterisk at char {}: context = {:?}",
                            i,
                            &mit_notice[i.saturating_sub(5)..(i + 5).min(mit_notice.len())]
                        );
                    }
                }
            }
        }

        // Tokenize specific parts
        eprintln!("\n=== TOKENIZING 'can be * found' ===");
        let part1 = "can be  * found"; // MIT_129 version (double space before *)
        let part2 = "can be\n * found"; // Query version (newline before space-*)
        let part3 = "can be * found"; // Single space

        eprintln!("'{}' -> {:?}", part1, tokenize(part1));
        eprintln!("'{}' -> {:?}", part2.replace('\n', "\\n"), tokenize(part2));
        eprintln!("'{}' -> {:?}", part3, tokenize(part3));

        // Check what happens with the actual text
        eprintln!("\n=== CHECKING THE * CHARACTER ===");
        // The * by itself should be ignored by the tokenizer
        let asterisk_only = "*";
        eprintln!("'{}' -> {:?}", asterisk_only, tokenize(asterisk_only));

        // Check if * is in the dictionary
        let asterisk_token = index.dictionary.get("*");
        eprintln!("'*' in dictionary: {:?}", asterisk_token);

        // Look for the "star" token which might be what * is tokenized as
        let star_token = index.dictionary.get("star");
        eprintln!("'star' in dictionary: {:?}", star_token);
    }

    #[test]
    fn test_json_escaping_issue() {
        eprintln!("\n=== JSON ESCAPING ANALYSIS ===");

        // Read the raw file
        let raw_text =
            std::fs::read_to_string("testdata/license-golden/datadriven/lic2/ar-ER.js.map")
                .expect("Failed to read ar-ER.js.map");

        // Find the MIT notice in the raw text
        if let Some(start) = raw_text.find("Use of this source") {
            let end = raw_text[start..]
                .find("\\*")
                .map(|i| start + i + 10)
                .unwrap_or(start + 200);
            let raw_section = &raw_text[start..end.min(raw_text.len())];
            eprintln!("Raw text around MIT notice:");
            eprintln!("{}", raw_section);
            eprintln!();

            // Check for literal backslash-n
            if raw_section.contains("\\n") {
                eprintln!("Found literal \\n (backslash-n) in raw text!");
            }
            if raw_section.contains('\n') {
                eprintln!("Found actual newline in raw text!");
            }
        }

        // Tokenize the raw text
        let raw_tokens = tokenize(&raw_text);
        eprintln!(
            "\nFirst 50 tokens from raw text: {:?}",
            &raw_tokens[..50.min(raw_tokens.len())]
        );

        // Count "n" tokens
        let n_count = raw_tokens.iter().filter(|t| t.as_str() == "n").count();
        eprintln!("Number of 'n' tokens in raw text: {}", n_count);

        // Parse JSON properly and tokenize the sourcesContent
        eprintln!("\n=== PROPER JSON PARSING ===");
        // The sourcesContent is at a specific position - let's find it
        if let Some(sources_start) = raw_text.find("\"sourcesContent\":[\"") {
            let sources_start = sources_start + "\"sourcesContent\":[\"".len();
            if let Some(sources_end) = raw_text[sources_start..].find("\"]") {
                let sources_content = &raw_text[sources_start..sources_start + sources_end];

                // Unescape the JSON string (replace \\n with actual newline)
                let unescaped = sources_content.replace("\\n", "\n");

                eprintln!("Unescaped sourcesContent (first 500 chars):");
                eprintln!("{}", &unescaped[..500.min(unescaped.len())]);

                // Tokenize the unescaped version
                let unescaped_tokens = tokenize(&unescaped);
                eprintln!(
                    "\nUnescaped tokens ({}): first 30 = {:?}",
                    unescaped_tokens.len(),
                    &unescaped_tokens[..30.min(unescaped_tokens.len())]
                );

                // Check MIT_129 tokens against unescaped tokens
                let mit_129_text = "Use of this source code is governed by an {{MIT-style license}} that can be  * found in the LICENSE file at https://angular.io/license";
                let mit_129_tokens = tokenize(mit_129_text);

                eprintln!(
                    "\nMIT_129 tokens ({}): {:?}",
                    mit_129_tokens.len(),
                    mit_129_tokens
                );

                // Find MIT_129 in unescaped tokens
                for i in 0..unescaped_tokens.len().saturating_sub(mit_129_tokens.len()) {
                    if unescaped_tokens[i..i + mit_129_tokens.len()] == mit_129_tokens[..] {
                        eprintln!("MIT_129 found at position {} in unescaped tokens!", i);
                    }
                }
            }
        }
    }

    #[test]
    fn test_mit_129_tokenization() {
        eprintln!("\n=== MIT_129 TOKENIZATION ANALYSIS ===");

        // The actual rule text from mit_129.RULE
        let mit_129_text = "Use of this source code is governed by an {{MIT-style license}} that can be  * found in the LICENSE file at https://angular.io/license";

        // The query text from ar-ER.js.map (the sourcesContent)
        let query_text = "Use of this source code is governed by an MIT-style license that can be\n * found in the LICENSE file at https://angular.io/license";

        eprintln!("Rule text (mit_129):");
        eprintln!("{}", mit_129_text);
        eprintln!();
        eprintln!("Query text:");
        eprintln!("{}", query_text);
        eprintln!();

        let mit_129_tokens = tokenize(mit_129_text);
        let query_tokens = tokenize(query_text);

        eprintln!(
            "MIT_129 tokens ({}): {:?}",
            mit_129_tokens.len(),
            mit_129_tokens
        );
        eprintln!();
        eprintln!("Query tokens ({}): {:?}", query_tokens.len(), query_tokens);
        eprintln!();

        // Find where they diverge
        eprintln!("=== TOKEN COMPARISON ===");
        for (i, (r, q)) in mit_129_tokens.iter().zip(query_tokens.iter()).enumerate() {
            if r != q {
                eprintln!("DIVERGENCE at {}: rule='{}' vs query='{}'", i, r, q);
            }
        }

        // Also check mit_131 and mit_132
        let mit_131_text = "Use of this source code is governed by an {{MIT-style license}}";
        let mit_132_text = "https://angular.io/license";

        let mit_131_tokens = tokenize(mit_131_text);
        let mit_132_tokens = tokenize(mit_132_text);

        eprintln!(
            "\nMIT_131 tokens ({}): {:?}",
            mit_131_tokens.len(),
            mit_131_tokens
        );
        eprintln!(
            "MIT_132 tokens ({}): {:?}",
            mit_132_tokens.len(),
            mit_132_tokens
        );

        // Find MIT_131 tokens in query
        eprintln!("\n=== FINDING MIT_131 IN QUERY ===");
        for i in 0..query_tokens.len().saturating_sub(mit_131_tokens.len()) {
            if query_tokens[i..i + mit_131_tokens.len()] == mit_131_tokens[..] {
                eprintln!(
                    "MIT_131 found at query position {}-{}",
                    i,
                    i + mit_131_tokens.len()
                );
            }
        }

        // Find MIT_129 tokens in query
        eprintln!("\n=== FINDING MIT_129 IN QUERY ===");
        for i in 0..query_tokens.len().saturating_sub(mit_129_tokens.len()) {
            if query_tokens[i..i + mit_129_tokens.len()] == mit_129_tokens[..] {
                eprintln!(
                    "MIT_129 found at query position {}-{}",
                    i,
                    i + mit_129_tokens.len()
                );
            }
        }
        if query_tokens.len() >= mit_129_tokens.len() {
            let found = (0..query_tokens.len().saturating_sub(mit_129_tokens.len()) + 1)
                .any(|i| query_tokens[i..i + mit_129_tokens.len()] == mit_129_tokens[..]);
            if !found {
                eprintln!("MIT_129 NOT found as contiguous sequence in query!");
            }
        }
    }
}
