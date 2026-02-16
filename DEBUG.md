# License Detection Debug Notes

## Current Status

**Last Updated:** After detection sorting fix

### Unit Tests

All unit tests passing. Key improvements:

- `test_spdx_with_plus` - ✅ FIXED (PLAN-001)
- Deprecated rule filtering - ✅ IMPLEMENTED (PLAN-003)
- License intro filtering - ✅ IMPLEMENTED (PLAN-002)
- Overlapping match filtering - ✅ IMPLEMENTED (PLAN-004)
- Unicode tokenizer - ✅ IMPLEMENTED
- seq_match algorithm - ✅ IMPLEMENTED (PLAN-005)
- Match filtering - ✅ FIXED (PLAN-006)
- Detection sorting - ✅ IMPLEMENTED

### Golden Tests

| Metric | Initial | Current | Total Change |
|--------|---------|---------|--------------|
| Passed | 2,679 | 2,957 | **+278** |
| Failed | 1,684 | 1,406 | **-278** |

**Total Improvement:** 16.5% reduction in failing tests

Breakdown by directory:

| Directory | Passed | Failed |
|-----------|--------|--------|
| lic1 | 174 | 117 |
| lic2 | 708 | 145 |
| lic3 | 207 | 85 |
| lic4 | 218 | 132 |
| external | 1,648 | 919 |
| unknown | 2 | 8 |

Test data: 4,367 test files across 6 directories

**Command to run golden tests:**

```bash
cargo test --release license_detection::golden_test 2>&1 | grep -E "passed|failed|failures"
```

---

## Open Issues

### Issue: `double_isc.txt` Not Detecting `sudo` License

**Problem:** The test produces `["isc", "isc AND unknown"]` instead of expected `["isc", "isc", "sudo"]`.

**Status:** Under investigation

The DARPA text at the end of the file (lines 36-38) should match the `sudo` license but isn't being detected. All PLAN-005/006 steps are implemented, but the issue persists.

**Possible causes:**

- Token filtering in `find_longest_match` may be too strict
- The DARPA text may not have enough "legalese" tokens
- Match candidate selection may not include `sudo` as a candidate

**Files:** `src/license_detection/seq_match.rs`

---

## Completed Issues

### Detection Sorting (Fixed in `193b2a85`)

**Problem:** Detections returned in non-deterministic order due to HashMap iteration.

**Solution:** Added `sort_detections_by_line()` function that sorts detections by minimum match line number, matching Python's `qstart` ordering.

**Golden test improvement:** +43 passed (44 fixed, 1 regression)

---

### PLAN-006: Match Filtering Fix (Fixed in `65312944`)

**Problem:** Rust passed only `high_matchables` to `match_blocks`, but Python passes `high_matchables | low_matchables`. This prevented `extend_match` from extending matches into low-token areas.

**Solution:** Changed `query_run.matchables(false)` to `query_run.matchables(true)`.

**Golden test impact:** -1 passed (16 fixed, 19 regressed - later fixed by detection sorting)

---

### PLAN-005: seq_match Algorithm (Implemented in `c0bd26b6`)

**Problem:** Greedy alignment algorithm missed non-contiguous matches.

**Solution:**

- Replaced greedy `align_sequences` with divide-and-conquer `match_blocks`
- Added `find_longest_match` using dynamic programming LCS algorithm
- Added multiple match detection loop (`while qstart <= qfinish`)
- Removed 50% coverage filter

**Golden test impact:** -4 passed (27 fixed, 31 regressed - later fixed by PLAN-006 and detection sorting)

---

### Unicode Tokenizer (Implemented)

**Problem:** ASCII-only tokenizer fragmented non-ASCII text.

**Solution:** Updated pattern from `[A-Za-z0-9]+` to `[^_\W]+` to match Python's `re.UNICODE`.

**Golden test impact:** -36 passed (but more correct for Python parity)

---

### Issue 0: SPDX `+` Suffix (Fixed in `9b47b558`)

**Problem:** SPDX identifiers with `+` suffix (e.g., `GPL-2.0+`) were not detected.

**Solution:**

- Added `spdx_license_key` and `other_spdx_license_keys` fields to Rule
- Built `rid_by_spdx_key` lookup table in LicenseIndex
- Updated `find_best_matching_rule()` to use SPDX key lookup

---

### Issue 1: License Intro Filtering (Fixed in `f93270b6`)

**Problem:** License expressions incorrectly included "unknown" from intro matches.

**Solution:**

- Added `is_license_intro`, `is_license_clue` fields to LicenseMatch
- Implemented `filter_license_intros()` function
- Updated detection pipeline to filter intros

---

### Issue 2: Deprecated Rules (Fixed in `3b5ea424`)

**Problem:** Deprecated rules were being used for detection.

**Solution:**

- Added `with_deprecated` parameter to loader functions
- Deprecated items filtered by default

---

### Issue 4: Overlapping Match Filtering (Fixed in `15b07829`)

**Problem:** Complex overlap scenarios between matches caused incorrect expression combinations.

**Solution:**

- Added `overlap()`, `overlap_ratio()`, `union_span()`, `intersects()` methods to Span
- Added `matcher_order()`, `hilen()`, `surround()` methods to LicenseMatch
- Implemented `filter_overlapping_matches()` with 4 overlap thresholds
- Implemented `restore_non_overlapping()` to recover non-conflicting discarded matches

**Golden test improvement:** +24 passed

---

### Other Completed Fixes

| Fix | Commit | Description |
|-----|--------|-------------|
| Unit Test RID Assumptions | test audit | Tests find rules by identifier, not hardcoded RID |
| YAML Frontmatter Parsing | `c21e272` | Regex-based parsing for rule files |
| Pipeline Short-Circuit | `b714310` | All matchers always run |
| Line Number Calculation | `0d72a6e` | `QueryRun::line_for_pos()` for accurate lines |
| GPL False Positive Filter | `0d72a6e` | `filter_short_gpl_matches()` removes short GPL |
| Rule Sorting | `a0952ea` | Consistent rule IDs via sorting |
| Test File Organization | `be6ba3d` | Follows AGENTS.md pattern |

---

## Investigation Reports (Not Yet Implemented)

### Python Functions Not Implemented

#### 1. `has_low_rule_relevance()`

| Aspect | Details |
|--------|---------|
| **Purpose** | Returns True if ALL matches have `rule.relevance < 70` |
| **What it affects** | Flags detections as `LOW_RELEVANCE` for human review |
| **Recommendation** | **NOT NEEDED** - Audit feature, not core detection |

#### 2. `is_license_reference_local_file()`

| Aspect | Details |
|--------|---------|
| **Purpose** | Returns True if match has non-empty `referenced_filenames` |
| **What it affects** | Filters "reference" matches from expression calculation |
| **Recommendation** | **IMPLEMENT** - Part of license reference resolution |

#### 3. `use_referenced_license_expression()`

| Aspect | Details |
|--------|---------|
| **Purpose** | Decides whether to merge license expression from referenced file |
| **What it affects** | Critical for "See LICENSE file" patterns |
| **Recommendation** | **IMPLEMENT** - Essential for license references |

---

### SPDX Case Sensitivity

**Verdict: NO FIX NEEDED**

Rust correctly handles SPDX key case sensitivity, matching Python's behavior. The implementation goes beyond parity by normalizing underscores to hyphens per SPDX spec.

---

## Prioritized Action Items

| Priority | Issue | Effort | Impact |
|----------|-------|--------|--------|
| 1 | Investigate `double_isc.txt` sudo detection | Medium | High |
| 2 | Implement license reference resolution | High | High |
