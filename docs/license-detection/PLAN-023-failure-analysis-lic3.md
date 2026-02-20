# PLAN-023: lic3 Golden Test Failure Analysis

## Summary

| Metric | Value |
|--------|-------|
| Total Tests | 292 |
| Passed | 250 |
| Failed | 42 |
| Skipped | 0 |
| Pass Rate | 85.6% |

## Failure Pattern Groups

### Pattern 1: Duplicate Matches Not Merged (6 cases, 14%)

**Symptom:** Rust returns duplicate license expressions where Python returns one.

**Examples:**

- `mit_and_mit.txt`: Expected `["mit"]`, Actual `["mit", "mit"]`
- `mentalis.txt`: Expected `["bsd-source-code"]`, Actual `["bsd-source-code", "bsd-source-code"]`
- `lgpl-2.1_14.txt`: Expected `["lgpl-2.1"]`, Actual `["lgpl-2.1", "lgpl-2.1"]`

**Root Cause:** The `merge_overlapping_matches()` function in `src/license_detection/match_refine.rs:128` merges matches with the same `rule_identifier` that overlap in token space. However, Python's behavior appears to merge matches that represent the same license even when from different rules or non-overlapping positions.

**Key Code Paths:**

- `src/license_detection/match_refine.rs:128` - `merge_overlapping_matches()`
- `src/license_detection/detection.rs:752` - `determine_license_expression()`
- `src/license_detection/expression.rs` - `combine_expressions()`

**Python Reference:**

- `reference/scancode-toolkit/src/licensedcode/match.py:merge_matches()`
- Python appears to merge identical license expressions during detection grouping

---

### Pattern 2: Missing Detections - Fewer Matches Than Expected (9 cases, 21%)

**Symptom:** Rust returns fewer license expressions than Python expects.

**Examples:**

- `mit_18.txt`: Expected `["mit", "mit", "mit"]`, Actual `["mit"]`
- `mit_2.txt`: Expected `["mit", "mit"]`, Actual `["mit"]`
- `libXrandr-*.txt`: Expected 3 matches, Actual 1

**Root Cause:** This relates to how Python tracks multiple occurrences of the same license in a file. Python creates separate detections for each distinct license occurrence based on line position, while Rust appears to merge them into a single detection.

The issue is in detection grouping/post-processing:

- `src/license_detection/detection.rs:149` - `group_matches_by_region()`
- `src/license_detection/detection.rs:1134` - `post_process_detections()`

The `remove_duplicate_detections()` function may be incorrectly deduplicating detections that should remain separate because they represent different locations in the file.

**Key Code Paths:**

- `src/license_detection/detection.rs:891` - `remove_duplicate_detections()`
- `src/license_detection/detection.rs:1075` - `apply_detection_preferences()`

---

### Pattern 3: Wrong License Expression - Complex Expression Simplification (5 cases, 12%)

**Symptom:** Rust returns a complex expression where Python expects a simple one, or vice versa.

**Examples:**

- `lzma-sdk-original.txt`: Expected `["lzma-sdk-2006"]`, Actual `["lgpl-2.1 WITH lzma-sdk-2006-exception OR cpl-1.0 WITH lzma-sdk-2006-exception"]`
- `lgpl-2.1-plus_with_other-copyleft_1.RULE`: Expected `["unknown-spdx"]`, Actual `["lgpl-2.1-plus"]`
- `odc-1.0.text`: Expected `["ppl"]`, Actual `["odc-by-1.0"]`

**Root Cause:** The LZMA SDK case is particularly interesting. The test file's YAML notes say this is a "composite" - the text describes a choice between LGPL and CPL with an exception. Python's detection logic appears to normalize this to the simpler `lzma-sdk-2006` identifier, while Rust returns the full expression.

This suggests a missing expression normalization step in Rust that Python performs, or different rule priority/specificity handling.

**Key Code Paths:**

- `src/license_detection/detection.rs:651` - `determine_license_expression()`
- `src/license_detection/expression.rs` - expression combination logic
- Rule specificity/priority handling in match refinement

---

### Pattern 4: Extra Detections - Spurious Matches (5 cases, 12%)

**Symptom:** Rust returns extra license expressions that Python doesn't return.

**Examples:**

- `lgpl-2.0-plus_with_wxwindows-exception-3.1_2.txt`: Expected 1 expression, Actual 6 (including exploded exception components)
- `nvidia-cuda.txt`: Expected 2 expressions, Actual 6 (extra `proprietary-license` duplicates)
- `mit_31.txt`: Expected 4, Actual 7 (extra `unknown-license-reference`, `free-unknown`, `proprietary-license`)

**Root Cause:** The wxWindows exception case shows Rust detecting both the combined expression `lgpl-2.0-plus WITH wxwindows-exception-3.1` AND the individual components separately. This suggests:

1. Rules exist for both combined and component expressions
2. Rust is not properly filtering contained/subsumed matches
3. Exception handling in expression combination may differ from Python

The `filter_contained_matches()` and `filter_overlapping_matches()` functions may not be handling the "subsumed by combined expression" case correctly.

**Key Code Paths:**

- `src/license_detection/match_refine.rs:249` - `filter_contained_matches()`
- `src/license_detection/match_refine.rs:389` - `filter_overlapping_matches()`
- `src/license_detection/expression.rs` - `licensing_contains()`

---

### Pattern 5: Missing Detection - Complete Failure (3 cases, 7%)

**Symptom:** Rust returns empty array where Python returns license expression(s).

**Examples:**

- `mit_additions_1.c`: Expected `["mit", "mit"]`, Actual `[]`
- `jcharts.txt`: Expected `["other-permissive"]`, Actual `[]`
- `mixed_ansible.txt`: Expected `["gpl-2.0", "cc-by-4.0"]`, Actual `[]`

**Root Cause:** These are complete detection failures. The `mit_additions_1.c` case is notable - it contains MIT license text with "added text" markers inserted. This likely breaks the match because:

1. The added text disrupts the rule's token sequence
2. The query tokenization may differ from Python's
3. Match quality thresholds may be filtering these out

**Key Code Paths:**

- `src/license_detection/query.rs` - tokenization
- `src/license_detection/seq_match.rs` - sequence matching for imperfect matches
- `src/license_detection/match_refine.rs:62` - `filter_too_short_matches()`

---

### Pattern 6: UTF-8 Encoding Issues (2 cases, 5%)

**Symptom:** Test files fail to read due to non-UTF-8 content.

**Examples:**

- `javassist-3.3.html`: "stream did not contain valid UTF-8"
- `long-s3cli-0.0.53-linux-amd64.go`: "stream did not contain valid UTF-8"

**Root Cause:** The golden test infrastructure uses `fs::read_to_string()` which requires valid UTF-8. These test files contain binary or non-UTF-8 content that Python handles but Rust's current test setup rejects.

**Key Code Paths:**

- `src/license_detection/golden_test.rs:110` - `fs::read_to_string()`

**Fix:** Use `fs::read()` to get bytes, then attempt UTF-8 conversion with lossy fallback.

---

### Pattern 7: Complex Multi-License Files - Detection Ordering/Dedup (5 cases, 12%)

**Symptom:** Complex files with many license references produce different counts/orderings.

**Examples:**

- `libbsd-*.txt`: Expected 17 expressions, Actual 9 (missing bsd-simplified variants)
- `libevent.LICENSE`: Expected 7 expressions, Actual 6
- `openindiana.txt`: Expected 24 expressions, Actual 6 (large file with repeated license blocks)

**Root Cause:** Large files with many license blocks are being under-detected. The `openindiana.txt` case shows the file has 4 repeated sections of the same license block, but Rust only detects 6 expressions instead of 24.

This suggests:

1. Detection grouping is merging too aggressively
2. The `LINES_THRESHOLD` proximity grouping may be too large for these files
3. Post-processing is deduplicating detections that represent different file regions

**Key Code Paths:**

- `src/license_detection/detection.rs:14` - `LINES_THRESHOLD = 4`
- `src/license_detection/detection.rs:149` - `group_matches_by_region()`
- `src/license_detection/detection.rs:891` - `remove_duplicate_detections()`

---

### Pattern 8: Wrong License Variant Detected (3 cases, 7%)

**Symptom:** Rust detects a different but related license variant.

**Examples:**

- `lgpl-3.0-plus_3.txt`: Expected `["lgpl-3.0-plus"]`, Actual `["lgpl-2.1-plus"]`
- `mpl-1.1_3.txt`: Expected `["mpl-1.1 AND free-unknown"]`, Actual `["mpl-1.0", "mpl-1.1"]`

**Root Cause:** Rule matching is selecting a different rule than Python expects. This could be due to:

1. Different rule priority when multiple rules match the same text
2. Different tokenization leading to different match scores
3. Missing rules or different rule data

**Key Code Paths:**

- `src/license_detection/aho_match.rs` - Aho-Corasick matching
- `src/license_detection/hash_match.rs` - Hash matching
- Rule loading/index building

---

## Recommendations

### High Priority

1. **Fix duplicate detection merging** (Pattern 1, 2)
   - Investigate Python's `merge_matches()` behavior for identical expressions
   - Consider merging detections with same license expression that are proximal
   - Review `remove_duplicate_detections()` - it should only dedupe by identifier (location), not by expression

2. **Fix detection count for multi-occurrence files** (Pattern 2, 7)
   - Each distinct file region should produce its own detection
   - Review `group_matches_by_region()` to ensure it splits properly
   - Do NOT merge detections that represent different file locations

3. **Fix UTF-8 handling in tests** (Pattern 6)
   - Change test file reading to handle binary/non-UTF-8 content
   - Use lossy UTF-8 conversion or byte-based processing

### Medium Priority

1. **Add expression normalization** (Pattern 3)
   - Investigate Python's composite license handling (LZMA SDK case)
   - Consider adding expression normalization rules for known patterns

2. **Improve exception/combined expression handling** (Pattern 4)
   - Filter out component matches when combined expression exists
   - Review `licensing_contains()` logic for exception expressions

### Lower Priority

1. **Improve detection of modified license text** (Pattern 5)
   - Review sequence matching thresholds for imperfect matches
   - Consider fuzzy matching for licenses with small modifications

2. **Investigate rule variant selection** (Pattern 8)
   - Ensure rule priority/specificity matches Python's behavior
   - Add debug logging to trace which rules are selected and why

---

## Code Files Requiring Investigation

| File | Patterns | Purpose |
|------|----------|---------|
| `src/license_detection/detection.rs` | 1, 2, 7 | Detection grouping and post-processing |
| `src/license_detection/match_refine.rs` | 1, 4, 5 | Match merging and filtering |
| `src/license_detection/expression.rs` | 3, 4 | Expression combination |
| `src/license_detection/golden_test.rs` | 6 | Test infrastructure |
| `src/license_detection/query.rs` | 5 | Tokenization |
| `src/license_detection/seq_match.rs` | 5 | Fuzzy matching |

---

## Next Steps

1. Pick one representative case from Pattern 1 (e.g., `mit_and_mit.txt`)
2. Run both Python and Rust with debug logging
3. Compare the match lists before and after each refinement step
4. Identify the specific divergence point
5. Implement fix and verify against all related cases
