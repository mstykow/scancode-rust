# License Detection Feature Parity Roadmap

**Status:** Planning Complete  
**Created:** 2025-02-28  
**Last Updated:** 2025-02-28

## Executive Summary

### Current State

- **Golden Tests:** 146 failures
- **Root Cause:** The `filter_dupes` fix is correct and implemented. Remaining failures reveal pre-existing issues in other parts of the pipeline.
- **Impact:** Feature parity with Python requires addressing 8 categories of issues.

### Summary by Phase

| Phase | Category | Estimated Tests Fixed | Complexity | Priority |
|-------|----------|----------------------|------------|----------|
| 1 | Duplicate Detection Merging | ~30 | Medium | High |
| 2 | Source Map File Processing | ~2 | Medium | High |
| 3 | License Expression Combination | ~20 | Medium | High |
| 4 | Missing Detection | ~25 | Complex | Medium |
| 5 | Wrong License Selection | ~20 | Complex | Medium |
| 6 | Extra/Spurious Detections | ~35 | Complex | Medium |
| 7 | SPDX Expression Parsing | ~10 | Simple | Low |
| 8 | Minor/Order Differences | ~4 | Simple | Low |

---

## Phase 1: Duplicate Detection Merging

**Goal:** Ensure identical license occurrences are merged into single detections.

### Problem Statement

When the same license text appears multiple times in a file, Rust sometimes reports multiple detections instead of merging them.

**Example Failures:**

| Test File | Expected | Actual |
|-----------|----------|--------|
| `AAL.txt` | `["attribution"]` | `["attribution", "attribution"]` |
| `NCSA.txt` | `["uoi-ncsa"]` | `["uoi-ncsa", "uoi-ncsa"]` |
| `mit_and_mit.txt` | `["mit"]` | `["mit", "mit"]` |
| `bsd-new_92.txt` | `["bsd-new"]` | `["bsd-new", "bsd-new"]` |

### Root Cause

The match refinement pipeline in `match_refine.rs` has logic to merge overlapping/adjacent matches, but the detection grouping in `detection.rs` may create separate detections for matches that should be combined.

**Specific Issue Areas:**

1. **Match grouping threshold**: `LINES_THRESHOLD = 4` may not be sufficient for some cases
2. **Detection expression deduplication**: Same expression in one detection not being collapsed
3. **Query run splitting**: Matches from different query runs may not merge

### Code Locations

| Component | File | Key Function |
|-----------|------|--------------|
| Match merging | `src/license_detection/match_refine.rs` | `merge_overlapping_matches()` |
| Detection grouping | `src/license_detection/detection.rs` | `group_matches_by_region()` |
| Expression dedup | `src/license_detection/detection.rs` | `create_detection_from_group()` |

### Proposed Fix

1. Add detection-level expression deduplication
2. Review `is_after()` merge logic at `match_refine.rs:304`
3. Ensure matches from same rule at distant positions create separate detections (expected), but matches at identical/nearby positions are merged

### Expected Test Improvement

- **Tests Fixed:** ~30
- **Validation:** Run `cargo test --release -q --lib license_detection::golden_test`

---

## Phase 2: Source Map File Processing

**Goal:** Correctly extract and process license information from JavaScript/CSS source map files.

### Problem Statement

Source map files (`.js.map`, `.css.map`) are JSON files containing `sourcesContent` arrays with embedded source code. The license text in these files uses escaped newlines (`\n`), which must be unescaped before tokenization.

**Example Failure:**

| Test File | Expected | Actual |
|-----------|----------|--------|
| `ar-ER.js.map` | `["mit"]` | `["mit", "mit"]` |

### Root Cause Analysis

**What Python does (correct):**
1. `textcode/analysis.py:js_map_sources_lines()` parses JSON
2. JSON parser automatically unescapes `\n` to actual newlines
3. License detection runs on extracted content

**What Rust does (incorrect):**
1. Reads raw JSON file content directly
2. Tokenizer sees literal `\n` (backslash + 'n')
3. Extra "n" token breaks matches, causing shorter rules to match separately

### Evidence

```
MIT_129 tokens: [..., "can", "be", "found", ...]
Query tokens:   [..., "can", "be", "n", "found", ...]  // extra "n" from \n
```

### Code Locations

| Component | File | Action Needed |
|-----------|------|---------------|
| Scanner | `src/scanner/mod.rs` | Add source map detection |
| New module | `src/utils/sourcemap.rs` | Extract sourcesContent |

### Proposed Fix

```rust
pub fn extract_sourcemap_content(content: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(content).ok()?;
    let sources = json.get("sourcesContent")?.as_array()?;
    let combined: String = sources.iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    Some(combined)
}
```

Then in scanner:
1. Detect `.js.map` and `.css.map` files
2. Extract sourcesContent and use for license detection
3. Fall back to raw content if not valid source map

### Expected Test Improvement

- **Tests Fixed:** ~2
- **Validation:** `cargo test ar_er_debug_test --lib`

---

## Phase 3: License Expression Combination

**Goal:** Correctly combine license expressions for overlapping/nearby matches.

### Problem Statement

When multiple license matches are found in proximity, their expressions should be combined correctly using AND/OR logic.

**Example Failures:**

| Test File | Expected | Actual |
|-----------|----------|--------|
| `BSL-1.0_or_MIT.txt` | `["mit OR boost-1.0"]` | `["mit", "boost-1.0"]` |
| `Ruby.t2` | `["gpl-2.0 OR other-copyleft"]` | `["gpl-2.0", "other-copyleft"]` |
| `mit_or_commercial-option.txt` | `["mit OR commercial-license"]` | `["unknown-license-reference", "commercial-license", "mit", "mit"]` |

### Root Cause

The expression combination logic in `detection.rs` is not properly handling OR expressions. Matches that represent a dual-license choice should be combined with OR, not listed separately.

### Code Locations

| Component | File | Key Function |
|-----------|------|--------------|
| Detection creation | `src/license_detection/detection.rs` | `create_detection_from_group()` |
| Expression building | `src/license_detection/detection.rs` | `determine_license_expression()` |

### Proposed Fix

1. Identify dual-license rules (those with OR in their license_expression)
2. When combining matches from dual-license rules, preserve the OR relationship
3. Don't split OR expressions into separate detections

### Expected Test Improvement

- **Tests Fixed:** ~20
- **Validation:** `cargo test --release -q --lib license_detection::golden_test`

---

## Phase 4: Missing Detection

**Goal:** Ensure expected licenses are detected.

### Problem Statement

Some files have expected license detections that Rust completely misses.

**Example Failures:**

| Test File | Expected | Actual |
|-----------|----------|--------|
| `CATOSL.sep` | `["uoi-ncsa"]` | `[]` |
| `Apache-2.0-Header.t2` | `["apache-2.0", "warranty-disclaimer"]` | `[]` |
| `NASA-1.3.t1` | `["nasa-1.3", "nasa-1.3", "nasa-1.3"]` | `[]` |
| `WTFPL.t4` | `["wtfpl-2.0"]` | `[]` |
| `gpl-test2.txt` | `["gpl-1.0-plus"]` | `[]` |

### Root Cause Categories

1. **File format issues**: Non-standard extensions (`.sep`, `.RULE`) may not be processed
2. **Rule matching thresholds**: Minimum coverage or other thresholds too high
3. **Candidate selection**: Correct rule not being selected
4. **Query run boundaries**: File being split incorrectly

### Investigation Required

For each failing test:
1. Verify file is being read and tokenized
2. Check if correct rule exists in index
3. Trace candidate selection to see why rule isn't chosen
4. Check minimum_coverage thresholds

### Code Locations

| Component | File | Key Function |
|-----------|------|--------------|
| File scanning | `src/scanner/mod.rs` | File filtering |
| Candidate selection | `src/license_detection/seq_match.rs` | `compute_candidates_with_msets()` |
| Threshold filtering | `src/license_detection/match_refine.rs` | `filter_below_rule_minimum_coverage()` |

### Expected Test Improvement

- **Tests Fixed:** ~25
- **Complexity:** High (requires individual investigation per test)

---

## Phase 5: Wrong License Selection

**Goal:** Ensure the correct license is selected when multiple candidates match.

### Problem Statement

Rust sometimes detects a different license than Python for the same text.

**Example Failures:**

| Test File | Expected | Actual |
|-----------|----------|--------|
| `IBM-MIT-style.txt` | `["x11-ibm"]` | `["historical"]` |
| `MIT-CMU-style.txt` | `["x11-dec1"]` | `["cmu-uc"]` |
| `bsd.f` | `["bsd-simplified"]` | `["bsd-new"]` |
| `MIT.t19` | `["proprietary-license"]` | `["mit"]` |
| `BSD-3-Clause.t26` | `["bsd-new"]` | `["bsd-x11"]` |

### Root Cause Analysis

The `filter_dupes()` fix is correct, but it changes which candidate "wins" when multiple similar rules match. The issue is in candidate prioritization after deduplication.

**Key Finding from 0015-filter-dupes-regressions.md:**

The `matched_length` precision in `DupeGroupKey` may cause incorrect grouping:

| License | matched_length | Python rounded | Rust rounded | Same Group? |
|---------|---------------|----------------|--------------|-------------|
| x11-dec1 | 138 | 6.9 | 7 | - |
| cmu-uc | 133 | 6.7 | 7 | **YES (wrong)** |

In Python, 6.9 and 6.7 are DIFFERENT groups, so both candidates survive.
In Rust, they're the SAME group (7 = 7), so only one survives.

### Code Location

| Component | File | Line |
|-----------|------|------|
| DupeGroupKey | `src/license_detection/seq_match.rs` | 69 |

### Proposed Fix

```rust
// Current (line 69):
matched_length: ((candidate.score_vec_full.matched_length / 20.0) * 10.0).round() as i32,

// Should match Python's 1-decimal precision:
// Store the value that produces different groups for 6.9 vs 6.7
```

### Expected Test Improvement

- **Tests Fixed:** ~20
- **Validation:** `cargo test --release -q --lib license_detection::golden_test`

---

## Phase 6: Extra/Spurious Detections

**Goal:** Eliminate false positive detections that Python doesn't produce.

### Problem Statement

Rust sometimes detects more licenses than Python for the same text.

**Example Failures:**

| Test File | Expected | Actual |
|-----------|----------|--------|
| `Python-2.0.t1` | `["python"]` | `["psf-2.0", "python", "matplotlib-1.3.0", "python", "python", ...]` |
| `CC-BY-NC-4.0.t1` | `["cc-by-nc-4.0"]` | `["cc-by-4.0", "proprietary-license", ...]` |
| `Artistic-2.0.t1` | `["artistic-2.0"]` | `["artistic-2.0", "warranty-disclaimer", "warranty-disclaimer", "robert-hubley"]` |
| `OpenSSL.t1` | `["openssl-ssleay"]` | `["openssl-ssleay", "openssl", "ssleay-windows"]` |

### Root Cause Categories

1. **Over-matching in sequence matcher**: Too many candidates passing thresholds
2. **Insufficient filtering**: False positive filters not catching spurious matches
3. **Rule overlap**: Multiple similar rules all matching the same text

### Code Locations

| Component | File | Key Function |
|-----------|------|--------------|
| False positive filter | `src/license_detection/match_refine.rs` | `filter_false_positive_matches()` |
| Overlap filter | `src/license_detection/match_refine.rs` | `filter_overlapping_matches()` |
| Spurious filter | `src/license_detection/match_refine.rs` | `filter_spurious_matches()` |

### Investigation Approach

1. For each failing test, identify which extra detections are being added
2. Trace back to which matcher produced them
3. Determine which filter should have removed them
4. Add or adjust filter logic

### Expected Test Improvement

- **Tests Fixed:** ~35
- **Complexity:** High (requires analysis per test category)

---

## Phase 7: SPDX Expression Parsing

**Goal:** Correctly parse complex SPDX license expressions.

### Problem Statement

Files with `SPDX-License-Identifier:` tags containing complex expressions (OR, AND, WITH) may not be parsed correctly.

**Example Failures:**

| Test File | Expected | Actual |
|-----------|----------|--------|
| `uboot.c` | `["unknown-spdx OR unknown-spdx OR ..."]` | `["unknown-spdx"]` |
| `misc.c` | `["unknown-spdx OR unknown-spdx"]` | `["unknown-spdx"]` |
| `missing_leading_trailing_paren.txt` | `["(gpl-2.0 AND mit) AND unknown-spdx"]` | `["gpl-2.0"]` |

### Root Cause

The SPDX expression parser in `spdx_lid.rs` may not handle all expression formats correctly.

### Code Location

| Component | File | Key Function |
|-----------|------|--------------|
| SPDX parsing | `src/license_detection/spdx_lid.rs` | `parse_spdx_expression()` |

### Proposed Fix

1. Add support for repeated OR expressions without parentheses
2. Handle leading/trailing parentheses correctly
3. Preserve expression structure in output

### Expected Test Improvement

- **Tests Fixed:** ~10
- **Complexity:** Medium

---

## Phase 8: Minor/Order Differences

**Goal:** Address remaining minor differences in output format.

### Problem Statement

Some tests fail due to minor differences in expression order or formatting that don't affect semantic meaning.

**Example Failures:**

| Test File | Expected | Actual |
|-----------|----------|--------|
| `spdx-license-ids/README.md` | `["bsd-zero", "adobe-scl", "adobe-glyph", ...]` | `["adobe-glyph", "bsd-zero", "adobe-scl", ...]` |
| `mixed_ansible.txt` | `["gpl-2.0", "cc-by-4.0"]` | `["cc-by-4.0", "gpl-2.0"]` |

### Root Cause

Detection ordering is not deterministic or differs from Python's ordering logic.

### Proposed Fix

1. Sort detections by start_line before output
2. Sort expressions alphabetically within detections

### Expected Test Improvement

- **Tests Fixed:** ~4
- **Complexity:** Simple

---

## Dependencies Between Phases

```
Phase 1 (Duplicate Detection)
    ↓
Phase 5 (Wrong License Selection) - depends on Phase 1 for baseline
    ↓
Phase 3 (Expression Combination) - depends on Phase 5 for correct candidates
    ↓
Phase 6 (Extra Detections) - depends on Phase 3 for proper filtering
    ↓
Phase 4 (Missing Detection) - independent, can be done in parallel
    ↓
Phase 2 (Source Map) - independent, can be done in parallel
    ↓
Phase 7 (SPDX Parsing) - independent
    ↓
Phase 8 (Minor Differences) - final cleanup
```

---

## Risk Assessment

### High Risk

1. **Phase 4 (Missing Detection)** - Each test may require individual investigation. No single fix will address all cases.
2. **Phase 6 (Extra Detections)** - Filtering logic changes may have unintended side effects.

### Medium Risk

1. **Phase 5 (Wrong License Selection)** - The `matched_length` precision fix may cause other regressions.
2. **Phase 3 (Expression Combination)** - AND/OR logic changes could affect many tests.

### Low Risk

1. **Phase 1 (Duplicate Detection)** - Well-understood issue with clear fix path.
2. **Phase 2 (Source Map)** - Isolated change with minimal impact on other tests.
3. **Phase 7 (SPDX Parsing)** - Limited scope, only affects files with SPDX tags.
4. **Phase 8 (Minor Differences)** - Cosmetic fixes only.

---

## Detailed Issue Analysis

### Issue 1: AAL.txt Duplicate Detection

- **Test File:** `datadriven/external/atarashi/AAL.txt`
- **Expected:** `["attribution"]`
- **Actual:** `["attribution", "attribution"]`
- **Root Cause:** Match grouping not collapsing identical expressions
- **Python Reference:** `reference/scancode-toolkit/src/licensedcode/detect.py:group_matches()`
- **Rust Code:** `src/license_detection/detection.rs:group_matches_by_region()`
- **Proposed Fix:** Add expression deduplication within detection groups

### Issue 2: ar-ER.js.map Source Map

- **Test File:** `datadriven/lic2/ar-ER.js.map`
- **Expected:** `["mit"]`
- **Actual:** `["mit", "mit"]`
- **Root Cause:** Escaped newlines in JSON not being processed
- **Python Reference:** `reference/scancode-toolkit/src/textcode/analysis.py:js_map_sources_lines()`
- **Rust Code:** New module needed
- **Proposed Fix:** Extract sourcesContent from source map files before detection

### Issue 3: BSL-1.0_or_MIT Expression

- **Test File:** `datadriven/external/fossology-tests/Dual-license/BSL-1.0_or_MIT.txt`
- **Expected:** `["mit OR boost-1.0"]`
- **Actual:** `["mit", "boost-1.0"]`
- **Root Cause:** OR expressions not preserved in detection combination
- **Python Reference:** `reference/scancode-toolkit/src/licensedcode/detect.py:determine_license_expression()`
- **Rust Code:** `src/license_detection/detection.rs:determine_license_expression()`
- **Proposed Fix:** Check rule license_expression for OR when combining matches

### Issue 4: IBM-MIT-style Wrong License

- **Test File:** `datadriven/external/fossology-tests/IBM/IBM-MIT-style.txt`
- **Expected:** `["x11-ibm"]`
- **Actual:** `["historical"]`
- **Root Cause:** Candidate selection choosing wrong rule
- **Python Reference:** `reference/scancode-toolkit/src/licensedcode/match_set.py:filter_dupes()`
- **Rust Code:** `src/license_detection/seq_match.rs:filter_dupes()`
- **Proposed Fix:** Verify matched_length precision matches Python

### Issue 5: Python-2.0 Extra Detections

- **Test File:** `datadriven/external/glc/Python-2.0.t1`
- **Expected:** `["python"]`
- **Actual:** `["psf-2.0", "python", "matplotlib-1.3.0", "python", "python", ...]`
- **Root Cause:** Over-matching with insufficient filtering
- **Python Reference:** `reference/scancode-toolkit/src/licensedcode/match.py:refine_matches()`
- **Rust Code:** `src/license_detection/match_refine.rs:refine_matches()`
- **Proposed Fix:** Strengthen overlap filtering for similar rules

### Issue 6: uboot.c SPDX Expression

- **Test File:** `datadriven/external/spdx/uboot.c`
- **Expected:** `["unknown-spdx OR unknown-spdx OR ..."]`
- **Actual:** `["unknown-spdx"]`
- **Root Cause:** SPDX parser not handling repeated OR
- **Python Reference:** `reference/scancode-toolkit/src/licensedcode/spdx.py`
- **Rust Code:** `src/license_detection/spdx_lid.rs`
- **Proposed Fix:** Parse repeated OR expressions in SPDX identifiers

---

## Validation Approach

### Per-Phase Validation

1. Run relevant golden test subset before changes
2. Implement fix
3. Run golden tests to verify improvement
4. Run full test suite to check for regressions
5. Document results

### Final Validation

```bash
# Run all golden tests
cargo test --release -q --lib license_detection::golden_test

# Count failures (should be 0)
cargo test --release -q --lib license_detection::golden_test 2>&1 | grep -c "mismatch"

# Run full test suite
cargo test --lib
```

---

## References

- **Architecture:** `docs/license-detection/ARCHITECTURE.md`
- **Previous Investigation:** `docs/license-detection/0015-filter-dupes-regressions.md`
- **Other Issues:** `docs/license-detection/0014-other-license-detection-issues.md`
- **Python Reference:** `reference/scancode-toolkit/src/licensedcode/`
