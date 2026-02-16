# License Detection Debug Notes

## Current Status

**Last Updated:** After comprehensive test audit (commit pending)

### Unit Tests

| Module | Tests | Status |
|--------|-------|--------|
| `models.rs` | 28 | ✅ All passing |
| `query.rs` | 53 | ✅ All passing |
| `tokenize.rs` | 42 | ✅ All passing |
| `index/mod.rs` | 12 | ✅ All passing |
| `index/builder.rs` | 25 | ✅ All passing |
| `index/dictionary.rs` | 14 | ✅ All passing |
| `index/token_sets.rs` | 7 | ✅ All passing |
| `hash_match.rs` | 12 | ✅ All passing |
| `aho_match.rs` | 16 | ✅ All passing |
| `seq_match.rs` | 20 | ✅ All passing |
| `unknown_match.rs` | 17 | ✅ All passing |
| `spdx_lid.rs` | 57 | ✅ All passing |
| `detection.rs` | 123 | ✅ All passing |
| `expression.rs` | 73 | ✅ All passing |
| `match_refine.rs` | 44 | ✅ All passing |
| `spans.rs` | 22 | ✅ All passing |
| `spdx_mapping.rs` | 33 | ✅ All passing |
| `loader_test.rs` | 38 | ✅ All passing |
| `thresholds.rs` | 18 | ✅ All passing |
| `legalese.rs` | 8 | ✅ All passing |
| **Total** | **682** | ✅ **All passing** |

### Golden Tests

**Status:** 142 passed, 149 failed

Test data: 4,367 test files across 6 directories (lic1, lic2, lic3, lic4, external, unknown)

---

## Open Issues

### Issue 1: Unknown License Intros Appear in Expressions (Medium Priority)

**Problem:** Files produce detections with "unknown" in license expressions when they shouldn't.

**Example:** `COPYING.gplv3`

- **Expected:** `["gpl-3.0"]`
- **Actual:** 8 detections including `"gpl-3.0 AND unknown"`

**Root Cause:** Rust builds license expressions from ALL matches, including license intro matches that should be filtered.

**Python Behavior:** Python filters intros through two-step process (`detection.py`):

1. **`analyze_detection()`** (line 1760): Returns category `UNKNOWN_INTRO_BEFORE_DETECTION` when an unknown intro is followed by a proper license match.

2. **`get_detected_license_expression()`** (lines 1510-1514): Filters intros before building expression:

   ```python
   elif analysis == DetectionCategory.UNKNOWN_INTRO_BEFORE_DETECTION.value:
       matches_for_expression = filter_license_intros(license_matches)
   ```

3. **`is_license_intro()`** (lines 1349-1365): A match is an intro if:
   - Rule has `is_license_intro` OR `is_license_clue` OR `license_expression == 'free-unknown'`
   - AND matcher is exact (`MATCH_AHO_EXACT`) OR coverage is 100%

**Recommended Fix:**

1. **Reorder `create_detection_from_group()`** in `src/license_detection/detection.rs`:
   - Analyze category FIRST
   - Filter matches based on category
   - THEN build expression

2. **Fix `is_unknown_intro()` to match Python**:

   ```rust
   fn is_unknown_intro(m: &LicenseMatch) -> bool {
       let is_intro = m.is_license_intro || m.is_license_clue || 
                      m.license_expression == "free-unknown";
       let is_exact = m.matcher == "2-aho" || m.match_coverage >= 99.99;
       is_intro && is_exact
   }
   ```

3. **Ensure `is_license_intro` is populated** in `LicenseMatch` from rule data.

**Files Affected:**

| File | Location |
|------|----------|
| `src/license_detection/detection.rs` | `create_detection_from_group()` - reorder logic |
| `src/license_detection/models.rs` | `LicenseMatch` - needs `is_license_intro`, `is_license_clue` fields |
| `reference/scancode-toolkit/src/licensedcode/detection.py` | Lines 1349-1365, 1510-1514 |

---

### Issue 2: Deprecated Rules Handling (Low Priority)

**Problem:** Test `camellia_bsd.c` expected `bsd-2-clause-first-lines`, got `freebsd-doc` (a deprecated rule).

**Python Behavior:** Python **skips deprecated rules by default** (`models.py:1103-1104`):

```python
# always skip deprecated rules
rules = [r for r in rules if not r.is_deprecated]
```

Deprecated rules have `replaced_by` pointing to the new license key.

**Previous Fix Attempt:** Skip deprecated rules during index building.

**Result:** 19-test regression (160 → 141 passed).

**Why it failed:**

- Some tests explicitly expect deprecated license expressions (e.g., `freebsd-doc_*.txt` tests)
- The `freebsd-doc.LICENSE` file is NOT deprecated - only `freebsd-doc_5.RULE` is deprecated
- Tests should match against the non-deprecated LICENSE file

**Recommended Fix:**

1. **Keep skipping deprecated rules** (matches Python's default behavior)
2. **Update golden tests** that expect deprecated expressions to expect the replacement expressions
3. **Alternative:** Add `--with-deprecated` flag for backwards compatibility

**Files Affected:**

| File | Location |
|------|----------|
| `src/license_detection/index/builder.rs` | Rule loading - filter deprecated |
| `src/license_detection/rules/loader.rs` | Load `is_deprecated` from rule files |
| `testdata/license-golden/datadriven/lic1/freebsd-doc_*.txt.EXPECTED` | May need updates |

---

## Test Coverage Gaps

The following areas have been identified as needing additional test coverage or implementation work:

### Unicode Support in Tokenizer

The current tokenizer regex `[A-Za-z0-9]+\+?[A-Za-z0-9]*` only matches ASCII characters, unlike Python's `[^\W]` with `re.UNICODE` flag. This is a known limitation.

### Python Functions Not Yet Implemented

| Function | Python Location | Purpose |
|----------|-----------------|---------|
| `filter_overlapping_matches` | `match.py` | Complex overlap ratio logic |
| `restore_non_overlapping` | `match.py` | Restore non-overlapping matches |
| `has_low_rule_relevance()` | `detection.py` | Low relevance detection |
| `filter_license_intros()` | `detection.py` | Filter intro matches |
| `is_license_reference_local_file()` | `detection.py` | Local file reference detection |
| `use_referenced_license_expression()` | `detection.py` | Use referenced expression |

### SPDX Mapping Limitations

1. `other_spdx_license_keys` field not supported - Python has alternative SPDX identifiers
2. Case sensitivity in reverse lookup - Python lowercases SPDX keys
3. No integration tests with real SPDX license data from `resources/licenses/`

---

## Fixed Issues

### Unit Test RID Assumptions (Fixed in test audit)

**Problem:** Tests relied on hardcoded RID values that changed after sorting.

**Fix:** Updated tests to find rules by identifier/expression instead of by RID. Added helper functions `find_rid_by_identifier()` and `find_rid_by_expression()`.

**Files:** `src/license_detection/index/builder.rs`

### YAML Frontmatter Parsing (Fixed in `c21e272`)

**Problem:** Naive `split("---")` incorrectly split on dashes anywhere in content, truncating text for 199 rule files.

**Fix:** Replaced with regex `(?m)^-{3,}\s*$` matching only at line boundaries.

**Impact:**

- 25 license files now load correctly (including tcp-wrappers, ofl-1.1)
- 199 rule files now have full text content instead of truncated

**Note:** Golden tests correctly match Python output.

**Files:** `src/license_detection/rules/loader.rs`

### Pipeline Short-Circuit (Fixed in `b714310`)

**Problem:** Matchers were skipped when high-coverage matches found, missing partial licenses.

**Fix:** Removed `has_perfect_match`/`has_high_coverage` short-circuit logic. All matchers now always run.

**File:** `src/license_detection/mod.rs`

### Line Number Calculation (Fixed in `0d72a6e`)

**Problem:** All matches appeared to span the entire document.

**Fix:** Added `QueryRun::line_for_pos()` and used match positions instead of query run boundaries.

**Files:** `src/license_detection/query.rs`, `src/license_detection/aho_match.rs`, `src/license_detection/hash_match.rs`

### GPL False Positive Filter (Fixed in `0d72a6e`)

**Problem:** GPL matches with `matched_length <= 3` caused false positives.

**Fix:** Added `filter_short_gpl_matches()` to remove short GPL matches.

**File:** `src/license_detection/match_refine.rs`

### Rule Sorting (Fixed in `a0952ea`)

**Problem:** Rule IDs differed between Python and Rust due to missing sort.

**Fix:** Added `all_rules.sort()` before assigning rule IDs in `builder.rs`.

**Files:** `src/license_detection/index/builder.rs`, `src/license_detection/models.rs`

### Test File Organization (Fixed in `be6ba3d`)

**Problem:** Test files at wrong locations, not matching AGENTS.md pattern.

**Fix:**

- Moved `src/license_detection_golden_test.rs` → `src/license_detection/golden_test.rs`
- Merged `src/license_detection_test.rs` into inline tests in `mod.rs`
- Followed parser pattern: `<module>/golden_test.rs` for golden tests, inline `#[cfg(test)] mod tests` for unit tests
