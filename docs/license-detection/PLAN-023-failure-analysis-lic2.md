# PLAN-023: lic2 Golden Test Failure Analysis

**Date:** 2026-02-20  
**Status:** Analysis Complete  
**Scope:** 78 failures out of 853 lic2 tests (90.9% pass rate)

## Summary

| Metric | Count |
|--------|-------|
| Total Tests | 853 |
| Passed | 775 |
| Failed | 78 |
| Pass Rate | 90.9% |

## Failure Pattern Groups

### Pattern 1: Duplicate Detection Expected (20 cases)

**Symptom:** Rust returns 1 expression, Python expects 2 identical expressions

**Examples:**

- `bsd-new_17.txt`: Expected `["bsd-new", "bsd-new"]`, Actual `["bsd-new"]`
- `bsd-new_157.txt`, `bsd-new_158.txt`, `bsd-new_24.txt`, etc.
- `apache-2.0_and_apache-2.0.txt`: Expected `["apache-2.0", "apache-2.0"]`, Actual `["apache-2.0"]`

**Root Cause:**
Python's test expects one expression per **match**, while Rust's golden test flattens matches from detections. The key difference:

- **Python** (`licensedcode_test_utils.py:215`):

  ```python
  detected_expressions = [match.rule.license_expression for match in matches]
  ```

  This extracts expressions from **each individual match**.

- **Rust** (`golden_test.rs:125-129`):

  ```rust
  let actual: Vec<&str> = detections
      .iter()
      .flat_map(|d| d.matches.iter())
      .map(|m| m.license_expression.as_str())
      .collect();
  ```

  This also extracts from each match, but the issue is in **match merging/deduplication**.

**Investigation Area:**

- `src/license_detection/match_refine.rs:merge_overlapping_matches()` - may be merging matches that should remain separate
- Python may have multiple matches to the same license at different positions that are kept separate

**Files to Fix:**

- `src/license_detection/match_refine.rs` - match merging logic
- `src/license_detection/detection.rs` - detection grouping

---

### Pattern 2: Missing Detection - Empty Result (7 cases)

**Symptom:** Rust returns `[]`, Python detects license

**Examples:**

- `bsd-new_61.txt`: Expected `["bsd-new"]`, Actual `[]`
- `bsd-new_90.txt`: Expected `["bsd-new"]`, Actual `[]`
- `apache-1.1_11.txt`: Expected `["apache-1.1"]`, Actual `[]`
- `apple-attribution-1997.txt`: Expected `["apple-attribution-1997"]`, Actual `[]`
- `nuget/nuget_test_url_155.txt`: Expected `["mit"]`, Actual `[]`

**Root Cause:**
Complete detection failure - no matches found at all. Likely causes:

1. Rule text not matching due to encoding issues (e.g., `bsd-new_61.txt` has `ï¿œ` character)
2. Sequence matching threshold too strict
3. Rules missing from index or not loaded

**Investigation Area:**

- `bsd-new_61.txt` has corrupted encoding: `Copyright ï¿œ 2004` - may affect tokenization
- `apple-attribution-1997.txt` - check if rule exists in index
- `apache-1.1_11.txt` - check file content and rule coverage

**Files to Fix:**

- `src/license_detection/tokenize.rs` - handle encoding edge cases
- `src/license_detection/seq_match.rs` - matching thresholds
- `src/license_detection/index.rs` - rule loading

---

### Pattern 3: Extra Detection (20 cases)

**Symptom:** Rust returns more expressions than expected

**Examples:**

- `apache-1.1_1.txt`: Expected `["apache-1.1"]`, Actual `["apache-1.1", "mx4j"]`
- `apache-1.1_16.txt`: Expected `["apache-1.1"]`, Actual `["apache-1.1", "mx4j"]`
- `apache-1.1_19.txt`: Expected `["apache-1.1"]`, Actual `["apache-1.1", "apache-1.1", "apache-2.0"]`
- `apache-1.1_21.txt`: Expected `["apache-1.1"]`, Actual `["osl-3.0", "apache-1.1", "apache-1.1", "apache-2.0"]`
- `artistic-2.0.txt`: Expected `["artistic-2.0"]`, Actual `["gpl-1.0-plus OR artistic-perl-1.0", "warranty-disclaimer", "artistic-2.0"]`

**Root Cause:**
Rust is detecting additional matches that Python filters out. Likely causes:

1. False positive filtering differences
2. Rule containment/overlap filtering differences
3. Different handling of license intro/clue matches

**Investigation Area:**

- `filter_false_positive_matches()` in `match_refine.rs`
- `filter_contained_matches()` in `match_refine.rs`
- `filter_overlapping_matches()` in `match_refine.rs`

**Specific Cases:**

- `mx4j` appearing alongside `apache-1.1` - likely a contained match that should be filtered
- `warranty-disclaimer` appearing - may need to be filtered as contained in larger match

---

### Pattern 4: Wrong Expression - Fallback vs Primary (1 case)

**Example:**

- `antlr-pd_1.txt`: Expected `["antlr-pd"]`, Actual `["antlr-pd-fallback"]`

**Root Cause:**
Rule priority/selection issue - Rust is matching a "fallback" rule instead of the primary rule. This typically happens when:

1. Both rules match the same text
2. The fallback rule has higher priority in Rust's matching
3. Coverage or relevance calculation differs

**Investigation Area:**

- `src/license_detection/aho_match.rs` - rule matching order
- `src/license_detection/index.rs` - rule relevance calculation

---

### Pattern 5: UTF-8 Encoding Errors (5 cases)

**Examples:**

- `2189-bsd-bin/faq.doctree`: stream did not contain valid UTF-8
- `apache-1.1_25.txt`: stream did not contain valid UTF-8
- `basename.elf`: stream did not contain valid UTF-8
- `bsd-new_147.txt`: stream did not contain valid UTF-8
- `bsd-new_156.pdf`: stream did not contain valid UTF-8

**Root Cause:**
Rust's `fs::read_to_string()` fails on non-UTF-8 files. Python handles these by reading as bytes and decoding with error handling.

**Files to Fix:**

- `src/license_detection/golden_test.rs:110-116` - use `fs::read()` with lossy conversion or error handling
- Consider using `encoding_rs` or `bstr` crate for encoding-agnostic reading

---

### Pattern 6: Complex Multi-License Mismatches (25+ cases)

**Symptom:** Large arrays of expressions with missing or extra entries

**Examples:**

- `aes-128-3.0_and_bsd-new_and_bsd-original-uc_and_bsd-simplified_and_other.txt`: 88 expected, 75 actual
- `boost-1.0_and_bsd-simplified_and_cddl-1.0_and_gpl-2.0-classpath_and_other.txt`: 88 expected, 70+ actual

**Root Cause:**
Combination of Patterns 1, 2, and 3. Complex files with many licenses exhibit all the above issues compounded.

---

## Key Code Differences

### Python Test Logic

```python
# licensedcode_test_utils.py:215
detected_expressions = [match.rule.license_expression for match in matches]
```

Each match contributes one expression to the result list, even if multiple matches have the same expression.

### Rust Test Logic

```rust
// golden_test.rs:125-129
let actual: Vec<&str> = detections
    .iter()
    .flat_map(|d| d.matches.iter())
    .map(|m| m.license_expression.as_str())
    .collect();
```

Same approach, but the underlying match merging/filtering differs.

---

## Priority Fixes

### High Priority

1. **Match Merging Logic** - Pattern 1 & 3 affect 40+ tests
   - Review `merge_overlapping_matches()` - may be too aggressive
   - Check if Python keeps separate matches for same license at different positions

2. **False Positive Filtering** - Pattern 3 affects 20+ tests
   - Extra detections like `mx4j`, `warranty-disclaimer` need filtering
   - Review `filter_contained_matches()` and `filter_overlapping_matches()`

### Medium Priority

3. **Missing Detection** - Pattern 2
   - Debug why `bsd-new_61.txt` and similar files fail completely
   - Check encoding handling in tokenization

2. **UTF-8 Handling** - Pattern 5
   - Add graceful handling for non-UTF-8 files

### Low Priority

5. **Rule Selection** - Pattern 4
   - Single case, may resolve with other fixes

---

## Recommended Next Steps

1. **Debug Representative Cases:**
   - Run `bsd-new_17.txt` through Python and Rust with detailed logging
   - Compare the match lists before flattening
   - Identify which matches are merged/filtered differently

2. **Create Minimal Reproduction Tests:**

   ```rust
   #[test]
   fn debug_bsd_new_17() {
       // Add detailed logging for match merging
   }
   ```

3. **Compare Python vs Rust Match Lists:**
   - Focus on the matches BEFORE flattening to expressions
   - Check if Python produces 2 matches and Rust produces 1

4. **Review Python's Match Refinement:**
   - Read `reference/scancode-toolkit/src/licensedcode/match.py`
   - Compare `merge_matches()` and `filter_contained_matches()` implementations

---

## Files Requiring Investigation

| File | Purpose | Patterns |
|------|---------|----------|
| `src/license_detection/match_refine.rs` | Match merging/filtering | 1, 3 |
| `src/license_detection/detection.rs` | Detection grouping | 1 |
| `src/license_detection/seq_match.rs` | Sequence matching | 2 |
| `src/license_detection/tokenize.rs` | Text tokenization | 2 |
| `src/license_detection/golden_test.rs` | Test execution | 5 |
| `src/license_detection/index.rs` | Rule loading | 2, 4 |
