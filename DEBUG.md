# License Detection Debug Notes

## Current Status

**Last Updated:** After implementing PLAN-001, PLAN-002, PLAN-003, PLAN-004

### Unit Tests

All unit tests passing. Key improvements:

- `test_spdx_with_plus` - ✅ FIXED (PLAN-001)
- Deprecated rule filtering - ✅ IMPLEMENTED (PLAN-003)
- License intro filtering - ✅ IMPLEMENTED (PLAN-002)
- Overlapping match filtering - ✅ IMPLEMENTED (PLAN-004)

### Golden Tests

| Metric | Initial | After Plans 1-3 | After Plan 4 | Total Change |
|--------|---------|-----------------|--------------|--------------|
| Passed | 2,679 | 2,928 | 2,952 | **+273** |
| Failed | 1,684 | 1,435 | 1,411 | **-273** |

**Total Improvement:** 16% reduction in failing tests

Breakdown by directory (final):

| Directory | Passed | Failed |
|-----------|--------|--------|
| lic1 | 173 | 118 |
| lic2 | 704 | 149 |
| lic3 | 201 | 91 |
| lic4 | 216 | 134 |
| external | 1,656 | 911 |
| unknown | 2 | 8 |

Test data: 4,367 test files across 6 directories

---

## Completed Issues

### Issue 0: SPDX `+` Suffix (FIXED in `9b47b558`)

**Problem:** SPDX identifiers with `+` suffix (e.g., `GPL-2.0+`) were not detected.

**Solution:**

- Added `spdx_license_key` and `other_spdx_license_keys` fields to Rule
- Built `rid_by_spdx_key` lookup table in LicenseIndex
- Updated `find_best_matching_rule()` to use SPDX key lookup

### Issue 2: Deprecated Rules (FIXED in `3b5ea424`)

**Problem:** Deprecated rules were being used for detection.

**Solution:**

- Added `with_deprecated` parameter to loader functions
- Deprecated items filtered by default

### Issue 1: License Intro Filtering (PARTIALLY FIXED in `f93270b6`)

**Problem:** License expressions incorrectly included "unknown" from intro matches.

**Solution:**

- Added `is_license_intro`, `is_license_clue` fields to LicenseMatch
- Implemented `filter_license_intros()` function
- Updated detection pipeline to filter intros

**Remaining Issue:** The `double_isc.txt` test still shows "unknown" because the DARPA text isn't being matched as "sudo" - this is a separate seq_match algorithm issue.

### Issue 4: Overlapping Match Filtering (FIXED in `15b07829`)

**Problem:** Complex overlap scenarios between matches caused incorrect expression combinations.

**Solution:**

- Added `overlap()`, `overlap_ratio()`, `union_span()`, `intersects()` methods to Span
- Added `matcher_order()`, `hilen()`, `surround()` methods to LicenseMatch
- Implemented `filter_overlapping_matches()` with 4 overlap thresholds (10%, 40%, 70%, 90%)
- Implemented `restore_non_overlapping()` to recover non-conflicting discarded matches
- Updated `refine_matches()` pipeline

**Golden test improvement:** +24 passed

---

## Open Issues

### Issue 3: Partial License Text Not Detected

**Problem:** The `double_isc.txt` test produces `["isc", "isc AND unknown"]` instead of expected `["isc", "isc", "sudo"]`.

**Root Cause:** The DARPA text at the end of the file matches the `sudo` license text, but the `seq_match` algorithm isn't detecting it properly. This is a sequence alignment / partial matching issue.

**Files to Investigate:**

| File | Purpose |
|------|---------|
| `src/license_detection/seq_match.rs` | Sequence alignment matching |
| `reference/scancode-toolkit/src/licensedcode/data/licenses/sudo.LICENSE` | Contains the DARPA text |

---

## Test Coverage Gaps

The following areas have been identified as needing additional test coverage or implementation work:

### Unicode Support in Tokenizer

**Problem:** The current tokenizer regex `[A-Za-z0-9]+\+?[A-Za-z0-9]*` only matches ASCII characters, unlike Python's `[^\W]` with `re.UNICODE` flag.

**Not a Quick Win:** Attempted change from `[A-Za-z0-9]` to `[^\W_]` caused **regression of 28 tests** (2952→2924 passed). The golden test expected values appear to have been generated against ASCII-only tokenizer behavior, not Python's Unicode behavior. This requires deeper investigation to understand the mismatch.

### Python Functions Not Yet Implemented

| Function | Python Location | Purpose |
|----------|-----------------|---------|
| `has_low_rule_relevance()` | `detection.py` | Low relevance detection |
| `is_license_reference_local_file()` | `detection.py` | Local file reference detection |
| `use_referenced_license_expression()` | `detection.py` | Use referenced expression |

### SPDX Mapping Limitations

1. Case sensitivity in reverse lookup - Python lowercases SPDX keys
2. No integration tests with real SPDX license data from `resources/licenses/`

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
