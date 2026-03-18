//! Tests for query module.

#[cfg(test)]
mod tests {
    use crate::license_detection::index::LicenseIndex;
    use crate::license_detection::query::{PositionSpan, Query, QueryRun};
    use crate::license_detection::test_utils::create_test_index;

    fn create_query_test_index() -> LicenseIndex {
        create_test_index(&[("license", 0), ("copyright", 1), ("permission", 2)], 3)
    }
    #[test]
    fn test_query_new_with_empty_text() {
        let index = create_query_test_index();
        let query = Query::new("", &index).unwrap();

        assert!(query.is_empty());
        assert_eq!(query.len(), 0);
        assert!(!query.is_binary);
    }

    #[test]
    fn test_query_new_with_known_tokens() {
        let index = create_query_test_index();
        let text = "License copyright permission";
        let query = Query::new(text, &index).unwrap();

        assert_eq!(query.len(), 3);
        assert_eq!(query.token_at(0), Some(0));
        assert_eq!(query.token_at(1), Some(1));
        assert_eq!(query.token_at(2), Some(2));
        assert_eq!(query.line_for_pos(0), Some(1));
        assert_eq!(query.line_for_pos(1), Some(1));
        assert_eq!(query.line_for_pos(2), Some(1));
    }

    #[test]
    fn test_query_new_with_unknown_tokens() {
        let index = create_query_test_index();
        let text = "License foobar copyright";
        let query = Query::new(text, &index).unwrap();

        assert_eq!(query.len(), 2);
        assert_eq!(query.token_at(0), Some(0));
        assert_eq!(query.token_at(1), Some(1));

        assert_eq!(query.unknown_count_after(Some(0)), 1);
        assert_eq!(query.unknown_count_after(Some(1)), 0);
    }

    #[test]
    fn test_query_new_with_stopwords() {
        let index = create_query_test_index();
        let text = "license div copyright p";
        let query = Query::new(text, &index).unwrap();

        assert_eq!(query.len(), 2);

        assert_eq!(query.stopword_count_after(Some(0)), 1);
        assert_eq!(query.stopword_count_after(Some(1)), 1);
    }

    #[test]
    fn test_query_new_with_short_tokens() {
        let mut index = create_query_test_index();
        let _ = index.dictionary.get_or_assign("x");
        let _ = index.dictionary.get_or_assign("y");
        let _ = index.dictionary.get_or_assign("z");

        let text = "x y z license";
        let query = Query::new(text, &index).unwrap();

        assert!(!query.is_empty());
        assert!(query.len() <= 4);

        for pos in 0..query.len().min(3) {
            assert!(
                query.is_short_or_digit(pos),
                "Position {} should be short",
                pos
            );
        }
    }

    #[test]
    fn test_query_new_with_digit_tokens() {
        let mut index = create_query_test_index();
        let _ = index.dictionary.get_or_assign("123");
        let _ = index.dictionary.get_or_assign("456");

        let text = "123 456 license";
        let query = Query::new(text, &index).unwrap();

        assert!(query.is_short_or_digit(0));
        assert!(query.is_short_or_digit(1));
        assert!(!query.is_short_or_digit(2));
    }

    #[test]
    fn test_query_new_multiline() {
        let index = create_query_test_index();
        let text = "Line 1 license\nLine 2 copyright\nLine 3 permission";
        let query = Query::new(text, &index).unwrap();

        assert_eq!(query.len(), 3);
        assert_eq!(query.line_for_pos(0), Some(1));
        assert_eq!(query.line_for_pos(1), Some(2));
        assert_eq!(query.line_for_pos(2), Some(3));
    }

    #[test]
    fn test_query_tokens_length_without_unknowns() {
        let index = create_query_test_index();
        let text = "license foobar copyright";
        let query = Query::new(text, &index).unwrap();

        assert_eq!(query.tokens_length(false), 2);
    }

    #[test]
    fn test_query_tokens_length_with_unknowns() {
        let index = create_query_test_index();
        let text = "license foobar copyright";
        let query = Query::new(text, &index).unwrap();

        assert_eq!(query.tokens_length(true), 3);
    }

    #[test]
    fn test_query_detect_binary_text() {
        let index = create_query_test_index();

        let query = Query::new("license copyright", &index).unwrap();
        assert!(!query.is_binary);
    }

    #[test]
    fn test_query_detect_binary_null_bytes() {
        let index = create_query_test_index();
        let text = "license\0copyright";

        let query = Query::new(text, &index).unwrap();
        assert!(query.is_binary);
    }

    #[test]
    fn test_query_new_with_empty_lines() {
        let index = create_query_test_index();
        let text = "license\n\ncopyright";
        let query = Query::new(text, &index).unwrap();

        assert_eq!(query.len(), 2);
        assert_eq!(query.line_for_pos(0), Some(1));
        assert_eq!(query.line_for_pos(1), Some(3));
    }

    #[test]
    fn test_query_new_with_leading_unknowns() {
        let index = create_query_test_index();
        let text = "unknown1 unknown2 license";
        let query = Query::new(text, &index).unwrap();

        assert_eq!(query.len(), 1);
        assert_eq!(query.unknown_count_after(None), 2);
    }

    #[test]
    fn test_query_new_with_leading_stopwords() {
        let index = create_query_test_index();
        let text = "div p license";
        let query = Query::new(text, &index).unwrap();

        assert_eq!(query.len(), 1);
        assert_eq!(query.stopword_count_after(None), 2);
    }

    #[test]
    fn test_query_run_new() {
        let index = create_query_test_index();
        let query = Query::new("license copyright permission", &index).unwrap();
        let run = QueryRun::new(&query, 0, Some(2));

        assert_eq!(run.start, 0);
        assert_eq!(run.end, Some(2));
    }

    #[test]
    fn test_query_whole_query_run() {
        let index = create_query_test_index();
        let query = Query::new("license copyright permission", &index).unwrap();
        let run = query.whole_query_run();

        assert_eq!(run.start, 0);
        assert_eq!(run.end, Some(2));
        assert_eq!(run.start_line(), Some(1));
        assert_eq!(run.end_line(), Some(1));
    }

    #[test]
    fn test_query_run_tokens() {
        let index = create_query_test_index();
        let query = Query::new("license copyright permission", &index).unwrap();
        let run = QueryRun::new(&query, 0, Some(1));

        assert_eq!(run.tokens(), vec![0, 1]);
    }

    #[test]
    fn test_query_run_tokens_with_pos() {
        let index = create_query_test_index();
        let query = Query::new("license copyright permission", &index).unwrap();
        let run = QueryRun::new(&query, 0, Some(1));

        let tokens_with_pos: Vec<(usize, u16)> = run.tokens_with_pos().collect();
        assert_eq!(tokens_with_pos, vec![(0, 0), (1, 1)]);
    }

    #[test]
    fn test_query_run_empty() {
        let index = create_query_test_index();
        let query = Query::new("", &index).unwrap();
        let run = QueryRun::new(&query, 0, None);

        assert_eq!(run.tokens().len(), 0);
        assert_eq!(run.start, 0);
        assert_eq!(run.end, None);
    }

    #[test]
    fn test_query_matchables() {
        let index = create_query_test_index();
        let query = Query::new("license copyright permission", &index).unwrap();

        let matchables = query.matchables();
        assert_eq!(matchables.len(), 3);
        assert!(matchables.contains(&0));
        assert!(matchables.contains(&1));
        assert!(matchables.contains(&2));
    }

    #[test]
    fn test_query_high_matchables() {
        let index = create_query_test_index();
        let query = Query::new("license copyright permission", &index).unwrap();

        let high = query.high_matchables(&0, &Some(2));
        assert!(high.is_some());
        assert_eq!(high.unwrap().len(), 3);
        assert!(query.is_high_matchable(0));
        assert!(query.is_high_matchable(1));
        assert!(query.is_high_matchable(2));
    }

    #[test]
    fn test_query_low_matchables_in_range() {
        let mut index = create_query_test_index();
        let _ = index.dictionary.get_or_assign("word");
        let query = Query::new("license word copyright", &index).unwrap();

        let low = query.low_matchables(&0, &Some(2));
        assert!(low.is_some());
        assert!(low.unwrap().contains(&1));
    }

    #[test]
    fn test_query_run_matchables() {
        let index = create_query_test_index();
        let query = Query::new("license copyright permission", &index).unwrap();
        let run = QueryRun::new(&query, 0, Some(2));

        let matchables = run.matchables(true);
        assert_eq!(matchables.len(), 3);

        let high_matchables = run.matchables(false);
        assert_eq!(high_matchables.len(), 3);
    }

    #[test]
    fn test_query_run_is_matchable() {
        let index = create_query_test_index();
        let query = Query::new("license copyright permission", &index).unwrap();
        let run = QueryRun::new(&query, 0, Some(2));

        assert!(run.is_matchable(false, &[]));
        assert!(run.is_matchable(true, &[]));
    }

    #[test]
    fn test_query_run_is_not_matchable_digits_only() {
        let mut index = create_query_test_index();
        let tid_123 = index.dictionary.get_or_assign("123");
        let tid_456 = index.dictionary.get_or_assign("456");
        let _ = index.digit_only_tids.insert(tid_123);
        let _ = index.digit_only_tids.insert(tid_456);

        let query = Query::new("123 456", &index).unwrap();
        let run = QueryRun::new(&query, 0, Some(1));

        assert!(!run.is_matchable(false, &[]));
    }

    #[test]
    fn test_query_run_is_matchable_with_exclude() {
        let index = create_query_test_index();
        let query = Query::new("license copyright permission", &index).unwrap();
        let run = QueryRun::new(&query, 0, Some(2));

        let exclude_span = PositionSpan::new(0, 1);
        assert!(run.is_matchable(false, &[exclude_span]));
    }

    #[test]
    fn test_query_matchable_tokens() {
        let index = create_query_test_index();
        let query = Query::new("license copyright permission", &index).unwrap();
        let run = QueryRun::new(&query, 0, Some(2));

        let tokens = run.matchable_tokens();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], 0);
        assert_eq!(tokens[1], 1);
        assert_eq!(tokens[2], 2);
    }

    #[test]
    fn test_query_run_subtract() {
        let index = create_query_test_index();
        let mut query = Query::new("license copyright permission", &index).unwrap();

        let span = PositionSpan::new(0, 1);
        query.subtract(&span);

        assert!(!query.is_high_matchable(0));
        assert!(!query.is_high_matchable(1));
        assert!(query.is_high_matchable(2));
    }

    #[test]
    fn test_query_new_lowercase_tokens() {
        let index = create_query_test_index();
        let text = "LICENSE COPYRIGHT Permission";
        let query = Query::new(text, &index).unwrap();

        assert_eq!(query.len(), 3);
        assert_eq!(query.token_at(0), Some(0));
        assert_eq!(query.token_at(1), Some(1));
        assert_eq!(query.token_at(2), Some(2));
    }

    #[test]
    fn test_query_matched_text_single_line() {
        let index = create_query_test_index();
        let text = "license copyright permission";
        let query = Query::new(text, &index).unwrap();

        let matched = query.matched_text(1, 1);
        assert_eq!(matched, "license copyright permission");
    }

    #[test]
    fn test_query_matched_text_multiple_lines() {
        let index = create_query_test_index();
        let text = "line1\nline2\nline3\nline4";
        let query = Query::new(text, &index).unwrap();

        let matched = query.matched_text(2, 3);
        assert_eq!(matched, "line2\nline3");
    }

    #[test]
    fn test_query_matched_text_full_range() {
        let index = create_query_test_index();
        let text = "line1\nline2\nline3";
        let query = Query::new(text, &index).unwrap();

        let matched = query.matched_text(1, 3);
        assert_eq!(matched, "line1\nline2\nline3");
    }

    #[test]
    fn test_query_matched_text_invalid_range() {
        let index = create_query_test_index();
        let text = "line1\nline2";
        let query = Query::new(text, &index).unwrap();

        assert_eq!(query.matched_text(0, 1), "");
        assert_eq!(query.matched_text(2, 1), "");
        assert_eq!(query.matched_text(0, 0), "");
    }

    #[test]
    fn test_query_run_matched_text() {
        let index = create_query_test_index();
        let text = "line1\nlicense\nline3";
        let query = Query::new(text, &index).unwrap();
        let run = QueryRun::new(&query, 0, Some(0));

        let matched = run.matched_text(2, 2);
        assert_eq!(matched, "license");
    }

    #[test]
    fn test_query_detect_long_lines() {
        let index = create_query_test_index();
        let tokens: Vec<String> = (0..30).map(|i| format!("word{}", i)).collect();
        let text = tokens.join(" ");
        let query = Query::new(&text, &index).unwrap();

        assert!(query.has_long_lines);
    }

    #[test]
    fn test_query_no_long_lines() {
        let index = create_query_test_index();
        let text = "license copyright permission";
        let query = Query::new(text, &index).unwrap();

        assert!(!query.has_long_lines);
    }

    #[test]
    fn test_query_matched() {
        let index = create_query_test_index();
        let mut query = Query::new("license copyright permission", &index).unwrap();

        let span = PositionSpan::new(0, 1);
        query.subtract(&span);

        let matched = query.matched();
        assert_eq!(matched.len(), 2);
        assert!(matched.contains(&0));
        assert!(matched.contains(&1));
        assert!(!matched.contains(&2));
    }

    #[test]
    fn test_query_run_get_index() {
        let index = create_query_test_index();
        let query = Query::new("license copyright", &index).unwrap();
        let run = QueryRun::new(&query, 0, Some(1));

        let idx = run.get_index();
        assert_eq!(idx.len_legalese, 3);
    }

    #[test]
    fn test_query_run_line_for_pos() {
        let index = create_query_test_index();
        let text = "license\ncopyright\npermission";
        let query = Query::new(text, &index).unwrap();
        let run = QueryRun::new(&query, 0, Some(2));

        assert_eq!(run.line_for_pos(0), Some(1));
        assert_eq!(run.line_for_pos(1), Some(2));
        assert_eq!(run.line_for_pos(2), Some(3));
        assert_eq!(run.line_for_pos(10), None);
    }

    #[test]
    fn test_query_run_is_matchable_all_excluded() {
        let index = create_query_test_index();
        let query = Query::new("license copyright permission", &index).unwrap();
        let run = QueryRun::new(&query, 0, Some(2));

        let exclude_span = PositionSpan::new(0, 2);
        assert!(!run.is_matchable(false, &[exclude_span]));
    }

    #[test]
    fn test_query_new_with_unicode() {
        let index = create_query_test_index();
        let text = "licença copyright 许可";
        let query = Query::new(text, &index).unwrap();

        assert_eq!(query.len(), 1);
        assert_eq!(query.token_at(0), Some(1));
    }

    #[test]
    fn test_query_high_matchables_out_of_range() {
        let index = create_query_test_index();
        let query = Query::new("license copyright", &index).unwrap();

        let high = query.high_matchables(&100, &Some(200));
        assert!(high.is_none());
    }

    #[test]
    fn test_query_low_matchables_out_of_range() {
        let index = create_query_test_index();
        let query = Query::new("license copyright", &index).unwrap();

        let low = query.low_matchables(&100, &Some(200));
        assert!(low.is_none());
    }

    #[test]
    fn test_query_high_matchables_unbounded_end() {
        let index = create_query_test_index();
        let query = Query::new("license copyright permission", &index).unwrap();

        let high = query.high_matchables(&1, &None);
        assert!(high.is_some());
        let high_set = high.unwrap();
        assert!(high_set.contains(&1));
        assert!(high_set.contains(&2));
        assert!(!high_set.contains(&0));
    }

    #[test]
    fn test_query_run_end_line_none() {
        let index = create_query_test_index();
        let query = Query::new("", &index).unwrap();
        let run = QueryRun::new(&query, 0, None);

        assert_eq!(run.end_line(), None);
    }

    #[test]
    fn test_query_with_only_stopwords() {
        let index = create_query_test_index();
        let text = "div p a br";
        let query = Query::new(text, &index).unwrap();

        assert!(query.is_empty());
        assert_eq!(query.stopword_count_after(None), 4);
    }

    #[test]
    fn test_query_with_only_unknowns() {
        let index = create_query_test_index();
        let text = "unknown1 unknown2 unknown3";
        let query = Query::new(text, &index).unwrap();

        assert!(query.is_empty());
        assert_eq!(query.unknown_count_after(None), 3);
    }

    #[test]
    fn test_query_run_matchable_tokens_empty_high_matchables() {
        let mut index = create_query_test_index();
        let _ = index.dictionary.get_or_assign("word");
        index.len_legalese = 0;

        let query = Query::new("word word", &index).unwrap();
        let run = QueryRun::new(&query, 0, Some(1));

        let tokens = run.matchable_tokens();
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_query_run_is_digits_only_mixed() {
        let mut index = create_query_test_index();
        let tid_123 = index.dictionary.get_or_assign("123");
        let _ = index.digit_only_tids.insert(tid_123);
        let _ = index.dictionary.get_or_assign("license");

        let query = Query::new("123 license", &index).unwrap();
        let run = QueryRun::new(&query, 0, Some(1));

        assert!(!run.is_digits_only());
    }

    #[test]
    fn test_query_run_high_low_matchables_slice() {
        let index = create_query_test_index();
        let query = Query::new("license copyright permission", &index).unwrap();
        let run = QueryRun::new(&query, 1, Some(2));

        let high = run.high_matchables();
        assert_eq!(high.len(), 2);
        assert!(high.contains(&1));
        assert!(high.contains(&2));
        assert!(!high.contains(&0));

        let low = run.low_matchables();
        assert!(low.is_empty());
    }

    #[test]
    fn test_query_run_splitting_single_run() {
        let index = create_query_test_index();
        let text = "license copyright permission";
        let query = Query::with_options(text, &index, 4).unwrap();

        let runs = query.query_runs();
        assert_eq!(runs.len(), 1);
        assert_eq!(query.query_run_ranges, vec![(0, Some(2))]);
        assert_eq!(runs[0].start, 0);
        assert_eq!(runs[0].end, Some(2));
    }

    #[test]
    fn test_query_run_splitting_with_empty_lines() {
        let index = create_query_test_index();
        let text = "license\n\n\n\n\ncopyright";
        let query = Query::with_options(text, &index, 4).unwrap();

        let runs = query.query_runs();
        assert_eq!(query.query_run_ranges, vec![(0, Some(0)), (1, Some(1))]);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].start, 0);
        assert_eq!(runs[0].end, Some(0));
        assert_eq!(runs[1].start, 1);
        assert_eq!(runs[1].end, Some(1));
    }

    #[test]
    fn test_query_run_splitting_below_threshold() {
        let index = create_query_test_index();
        let text = "license\n\n\ncopyright";
        let query = Query::with_options(text, &index, 4).unwrap();

        let runs = query.query_runs();
        assert_eq!(runs.len(), 1);
        assert_eq!(query.query_run_ranges, vec![(0, Some(1))]);
    }

    #[test]
    fn test_query_run_splitting_empty_query() {
        let index = create_query_test_index();
        let query = Query::with_options("", &index, 4).unwrap();

        assert!(query.query_run_ranges.is_empty());

        let runs = query.query_runs();
        assert!(runs.is_empty());
    }

    #[test]
    fn test_query_tracks_spdx_lines_with_positions() {
        let mut index = create_query_test_index();
        let _ = index.dictionary.get_or_assign("spdx");
        let _ = index.dictionary.get_or_assign("license");
        let _ = index.dictionary.get_or_assign("identifier");
        let _ = index.dictionary.get_or_assign("mit");
        let _ = index.dictionary.get_or_assign("apache");

        let text = "SPDX-License-Identifier: MIT\nSPDX-License-Identifier: Apache-2.0";
        let query = Query::new(text, &index).unwrap();

        assert_eq!(query.spdx_lines.len(), 2, "Should track 2 SPDX lines");

        for (spdx_text, start, end) in &query.spdx_lines {
            assert!(*start <= *end, "Token positions should be valid");
            assert!(
                spdx_text.to_lowercase().contains("spdx"),
                "SPDX text should contain SPDX keyword"
            );
        }
    }

    #[test]
    fn test_query_spdx_lines_not_at_position_zero() {
        let mut index = create_query_test_index();
        let _ = index.dictionary.get_or_assign("spdx");
        let _ = index.dictionary.get_or_assign("license");
        let _ = index.dictionary.get_or_assign("identifier");
        let _ = index.dictionary.get_or_assign("mit");

        let text = "license copyright\nSPDX-License-Identifier: MIT";
        let query = Query::new(text, &index).unwrap();

        assert_eq!(query.spdx_lines.len(), 1, "Should track 1 SPDX line");

        let (_, start, _) = &query.spdx_lines[0];
        assert!(
            *start >= 2,
            "SPDX start position should be >= 2 (after 'license copyright')"
        );
    }

    #[test]
    fn test_query_run_splitting_multiple_segments() {
        let index = create_query_test_index();
        let text = "license\n\n\n\n\ncopyright\n\n\n\n\npermission";
        let query = Query::with_options(text, &index, 4).unwrap();

        let runs = query.query_runs();
        assert_eq!(
            query.query_run_ranges,
            vec![(0, Some(0)), (1, Some(1)), (2, Some(2))]
        );
        assert_eq!(runs.len(), 3);
    }

    #[test]
    fn test_query_run_splitting_breaks_good_long_line_into_python_pseudolines() {
        let mut index = create_query_test_index();
        let low_tokens: Vec<String> = (0..25).map(|i| format!("word{}", i)).collect();
        for token in &low_tokens {
            let _ = index.dictionary.get_or_assign(token);
        }

        let text = format!("{} license", low_tokens.join(" "));
        let query = Query::with_options(&text, &index, 1).unwrap();

        assert!(query.has_long_lines);
        assert_eq!(query.query_run_ranges, vec![(0, Some(24)), (25, Some(25))]);
    }

    #[test]
    fn test_query_run_splitting_unknown_only_lines_count_toward_threshold() {
        let index = create_query_test_index();
        let text = "license\nfoobar bazqux\nbleep bloop\ncopyright";
        let query = Query::with_options(text, &index, 2).unwrap();

        assert_eq!(query.query_run_ranges, vec![(0, Some(0)), (1, Some(1))]);
    }

    #[test]
    fn test_query_run_splitting_low_only_lines_count_toward_threshold() {
        let mut index = create_query_test_index();
        let _ = index.dictionary.get_or_assign("word1");
        let _ = index.dictionary.get_or_assign("word2");

        let text = "license\nword1 word2\nword1\ncopyright";
        let query = Query::with_options(text, &index, 2).unwrap();

        assert_eq!(query.query_run_ranges, vec![(0, Some(3)), (4, Some(4))]);
    }

    #[test]
    fn test_query_run_splitting_digits_only_lines_do_not_emit_final_digits_only_run() {
        let mut index = create_query_test_index();
        let tid_123 = index.dictionary.get_or_assign("123");
        let tid_456 = index.dictionary.get_or_assign("456");
        let _ = index.digit_only_tids.insert(tid_123);
        let _ = index.digit_only_tids.insert(tid_456);

        let text = "license\n123\n456";
        let query = Query::with_options(text, &index, 1).unwrap();

        assert_eq!(query.query_run_ranges, vec![(0, Some(1))]);
    }

    #[test]
    fn test_query_run_splitting_breaks_at_exact_threshold() {
        let index = create_query_test_index();
        let text = "license\n\n\ncopyright";
        let query = Query::with_options(text, &index, 2).unwrap();

        assert_eq!(query.query_run_ranges, vec![(0, Some(0)), (1, Some(1))]);
    }

    #[test]
    fn test_query_run_splitting_exact_long_line_boundary_does_not_split() {
        let mut index = create_query_test_index();
        let low_tokens: Vec<String> = (0..24).map(|i| format!("word{}", i)).collect();
        for token in &low_tokens {
            let _ = index.dictionary.get_or_assign(token);
        }

        let text = format!("{} license", low_tokens.join(" "));
        let query = Query::with_options(&text, &index, 1).unwrap();

        assert!(!query.has_long_lines);
        assert_eq!(query.query_run_ranges, vec![(0, Some(24))]);
    }

    #[test]
    fn test_query_run_splitting_uses_python_pseudoline_boundaries() {
        let mut index = create_query_test_index();
        let low_tokens: Vec<String> = (0..30).map(|i| format!("word{}", i)).collect();
        for token in &low_tokens {
            let _ = index.dictionary.get_or_assign(token);
        }

        let text = format!("license\nword0 word1\n{} license", low_tokens.join(" "));
        let query = Query::with_options(&text, &index, 2).unwrap();

        assert!(query.has_long_lines);
        assert_eq!(query.query_run_ranges, vec![(0, Some(27)), (28, Some(33))]);
    }

    #[test]
    fn test_query_from_extracted_text_uses_binary_line_threshold_and_flag() {
        let index = create_query_test_index();
        let text = format!("license\n{}copyright", "\n".repeat(20));

        let text_query = Query::new(&text, &index).unwrap();
        let binary_query = Query::from_extracted_text(&text, &index, true).unwrap();

        assert_eq!(
            text_query.query_run_ranges,
            vec![(0, Some(0)), (1, Some(1))]
        );
        assert_eq!(binary_query.query_run_ranges, vec![(0, Some(1))]);
        assert!(binary_query.is_binary);
    }

    #[test]
    fn test_query_subtract_removes_positions() {
        let index = create_query_test_index();
        let mut query = Query::new("license copyright permission", &index).unwrap();

        assert!(query.high_matchables.contains(&0));
        assert!(query.high_matchables.contains(&1));

        let span = PositionSpan::new(0, 1);
        query.subtract(&span);

        assert!(!query.high_matchables.contains(&0));
        assert!(!query.high_matchables.contains(&1));
        assert!(query.high_matchables.contains(&2));
    }

    #[test]
    fn test_query_run_is_matchable_with_exclusions() {
        let index = create_query_test_index();
        let query = Query::new("license copyright permission", &index).unwrap();
        let run = query.whole_query_run();

        assert!(run.is_matchable(false, &[]));

        let exclude = vec![PositionSpan::new(0, 1)];
        assert!(run.is_matchable(false, &exclude));

        let exclude_all = vec![PositionSpan::new(0, 2)];
        assert!(!run.is_matchable(false, &exclude_all));
    }

    #[test]
    fn test_subtraction_after_near_duplicate_match() {
        let index = create_query_test_index();
        let mut query = Query::new("license copyright license copyright", &index).unwrap();

        assert!(query.is_high_matchable(0));
        assert!(query.is_high_matchable(1));

        let near_dupe_span = PositionSpan::new(0, 1);
        query.subtract(&near_dupe_span);

        assert!(!query.is_high_matchable(0));
        assert!(!query.is_high_matchable(1));
        assert!(query.is_high_matchable(2));
        assert!(query.is_high_matchable(3));
    }

    #[test]
    fn test_whole_query_run_snapshot_preserves_pre_subtraction_matchables() {
        let index = create_query_test_index();
        let mut query = Query::new("license copyright permission", &index).unwrap();

        let whole_run = query.whole_query_run();
        let before_subtraction = whole_run.high_matchables();
        assert_eq!(before_subtraction.len(), 3);
        assert!(before_subtraction.contains(&0));
        assert!(before_subtraction.contains(&1));
        assert!(before_subtraction.contains(&2));

        query.subtract(&PositionSpan::new(0, 1));

        let snapshot_after_subtraction = whole_run.high_matchables();
        assert_eq!(snapshot_after_subtraction, before_subtraction);

        let live_run = query.query_runs().into_iter().next().unwrap();
        let live_matchables = live_run.high_matchables();
        assert_eq!(live_matchables.len(), 1);
        assert!(live_matchables.contains(&2));
        assert!(!live_matchables.contains(&0));
        assert!(!live_matchables.contains(&1));
    }
}
