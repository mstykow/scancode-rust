# License Detection Debug Notes

## Current Status

**Last Updated:** After PLAN-005 seq_match implementation

### Unit Tests

All unit tests passing. Key improvements:

- `test_spdx_with_plus` - ✅ FIXED (PLAN-001)
- Deprecated rule filtering - ✅ IMPLEMENTED (PLAN-003)
- License intro filtering - ✅ IMPLEMENTED (PLAN-002)
- Overlapping match filtering - ✅ IMPLEMENTED (PLAN-004)
- Unicode tokenizer - ✅ IMPLEMENTED
- seq_match algorithm - ✅ IMPLEMENTED (PLAN-005, partial)

### Golden Tests

| Metric | Initial | After Unicode | After PLAN-005 | Total Change |
|--------|---------|---------------|----------------|--------------|
| Passed | 2,679 | 2,929 | 2,915 | **+236** |
| Failed | 1,684 | 1,434 | 1,448 | **-236** |

**Total Improvement:** 14% reduction in failing tests

Breakdown by directory (current):

| Directory | Passed | Failed |
|-----------|--------|--------|
| lic1 | 169 | 122 |
| lic2 | 702 | 151 |
| lic3 | 199 | 93 |
| lic4 | 214 | 136 |
| external | 1,629 | 938 |
| unknown | 2 | 8 |

Test data: 4,367 test files across 6 directories

---

## Investigation Reports

### Issue 3: Partial License Text Not Detected (Partially Fixed)

**Problem:** The `double_isc.txt` test produces `["isc", "isc AND unknown"]` instead of expected `["isc", "isc", "sudo"]`.

**PLAN-005 Implementation Status:**

| Step | Description | Status |
|------|-------------|--------|
| 1 | Remove 50% coverage filter | ✅ Done |
| 2 | Add multiple match detection loop | ✅ Done |
| 3 | Implement `find_longest_match()` | ✅ Done |
| 4 | Implement `match_blocks()` | ✅ Done |
| 5 | Implement `extend_match()` | ✅ Done (integrated into find_longest_match) |
| 6 | Fix line number calculation | ✅ Done (already correct) |

**Remaining Issue:**

The DARPA text is still not being detected. Root cause: **match filtering logic**.

The new `find_longest_match()` function requires tokens to be:

1. Below `len_legalese` threshold (high-value legalese tokens)
2. In the `matchables` set

This filtering is too strict compared to Python. Investigation needed.

**Files:** `src/license_detection/seq_match.rs`

---

### PLAN-005: seq_match Algorithm (Implemented with Regression)

**Commit:** `c0bd26b6`

**Changes:**

- Replaced greedy `align_sequences` with divide-and-conquer `match_blocks`
- Added `find_longest_match` using dynamic programming LCS algorithm
- Added multiple match detection loop (`while qstart <= qfinish`)
- Removed 50% coverage filter

**Golden Test Results:**

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Passed | 2,919 | 2,915 | -4 |
| Failed | 1,448 | 1,452 | +4 |

**Breakdown:**

- 27 tests now pass that failed before (improved)
- 31 tests now fail that passed before (regression)

**New Failures Analysis:**

The 31 new failures are due to stricter token filtering in `find_longest_match`:

- Only "legalese" tokens (ID < `len_legalese`) are considered
- Position must be in `matchables` set
- This is more restrictive than the old greedy algorithm

Examples:

- `version.c` - GPL headers with code not detected
- `public-domain.txt` - Short texts lack enough legalese tokens
- `FSFULLR.t1` - FSF unlimited texts not matched

**New Passes Analysis:**

The 27 new passes include:

- Perl dual-license texts (`Perl_ref2`, `Perl_ref3`)
- BSD variants (`0bsd.txt`)
- Complex multi-license texts (`tzfile.h`)

**Next Steps:** Investigate match filtering logic to match Python behavior exactly.

---

### Unicode Tokenizer (Implemented)

**Change:** Updated tokenizer pattern from `[A-Za-z0-9]+` (ASCII) to `[^_\W]+` (Unicode) to match Python's `re.UNICODE` behavior.

**Command to run golden tests:**

```bash
cargo test --release license_detection::golden_test 2>&1 | grep -E "passed|failed|failures"
```

**Results:**

| Metric | ASCII (Before) | Unicode (After) | Change |
|--------|----------------|-----------------|--------|
| Passed | 2,965 | 2,929 | **-36** |
| Failed | 1,398 | 1,434 | **+36** |

**Breakdown by suite:**

| Suite | ASCII Passed | Unicode Passed | Change |
|-------|--------------|----------------|--------|
| lic1 | 175 | 172 | -3 |
| lic2 | 708 | 701 | -7 |
| lic3 | 203 | 199 | -4 |
| lic4 | 214 | 218 | +4 |
| external | 1,663 | 1,637 | -26 |
| unknown | 2 | 2 | 0 |

**Tests that PASS with ASCII but FAIL with Unicode (136 tests):**

**Root Cause Analysis: `apsl-2.0.txt` Example**

**UPDATE: This is NOT a Unicode tokenization issue!**

Debug output shows:

- **Expected:** `["apsl-2.0"]`
- **Actual (Unicode):** `["apsl-2.0", "apsl-1.0 AND apsl-2.0"]`

The `apsl-2.0` license IS detected correctly with Unicode. The problem is an **extra detection** from the title line "APPLE PUBLIC SOURCE LICENSE" matching a short `apsl-1.0` rule.

**Verification:**

The Python rule files (`apsl-2.0_6.RULE`, `apsl-2.0_7.RULE`) DO contain the accented French text:

```text
Les parties ont exigé que le présent contrat...
```

So Unicode tokenization matches correctly. The test failure is a **detection refinement issue** (overlapping matches, short rule filtering), not a tokenization issue.

**Why Tests Differ Between ASCII and Unicode:**

The actual tokenization differences are:

| Tokenizer | Test File Token | Rule File Token | Match? |
|-----------|-----------------|-----------------|--------|
| ASCII | `exig` (fragmented) | `exigé` | Partial match possible |
| Unicode | `exigé` (proper) | `exigé` | Exact match ✓ |

Unicode is **more correct** and the rule files support it properly.

**Tests that FAIL with ASCII but PASS with Unicode (100 tests):**

These improve because Unicode properly handles text that ASCII fragmented incorrectly:

```text
datadriven/external/atarashi/ECL-2.0.h
datadriven/external/fossology-tests/Artistic/Hero.java
... (100 total)
```

**Recommendation:**

Keep Unicode tokenization for Python parity. The test differences are **detection refinement issues**, not tokenization issues. The specific failing tests need separate investigation for why extra/incorrect detections occur.

**Files:** `src/license_detection/tokenize.rs`

---

### Python Functions Not Implemented (Investigated)

#### 1. `has_low_rule_relevance()`

| Aspect | Details |
|--------|---------|
| **Purpose** | Returns True if ALL matches have `rule.relevance < 70` |
| **Where called** | `get_ambiguous_license_detections_by_type()` - post-scan audit categorization only |
| **What it affects** | Flags detections as `LOW_RELEVANCE` for human review - does NOT affect detection results |
| **Complexity** | Trivial |
| **Recommendation** | **NOT NEEDED** - Purely an audit/review feature, not core detection logic |

#### 2. `is_license_reference_local_file()`

| Aspect | Details |
|--------|---------|
| **Purpose** | Returns True if match has non-empty `referenced_filenames` |
| **Where called** | `filter_license_references()` → expression calculation for certain detection categories |
| **What it affects** | Filters "reference" matches from expression calculation |
| **Complexity** | Trivial |
| **Recommendation** | **IMPLEMENT** - But only as part of full license reference resolution system |

#### 3. `use_referenced_license_expression()`

| Aspect | Details |
|--------|---------|
| **Purpose** | Decides whether to merge license expression from referenced file |
| **Where called** | `update_detection_from_referenced_files()` - core reference resolution |
| **What it affects** | Critical for correct detection of "See LICENSE file" patterns |
| **Complexity** | Moderate (requires full reference resolution infrastructure) |
| **Recommendation** | **IMPLEMENT** - Essential for correct detection of license references |

**Key Finding:** Functions 2 and 3 are part of a **missing feature**: license reference resolution. When a file contains "See LICENSE file", Python ScanCode detects it, looks up the referenced file, and merges the license expression. This is **not implemented** in scancode-rust.

---

### SPDX Case Sensitivity (Investigated)

**Verdict: NO FIX NEEDED**

The Rust implementation correctly handles SPDX key case sensitivity, matching Python's behavior:

| Operation | Python | Rust | Match |
|-----------|--------|------|-------|
| Lowercase at index build | `spdx_license_key.lower()` | `to_lowercase()` | ✓ |
| Lowercase at lookup | `_symbol.key.lower()` | `normalize_spdx_key()` | ✓ |
| Handle underscore→hyphen | Via SPDX parsing | Explicit in `normalize_spdx_key()` | ✓ |

**Key Code Locations:**

- Index building: `builder.rs:308-313` - inserts lowercased keys
- Lookup: `spdx_lid.rs:173-182` - `normalize_spdx_key()` lowercases + normalizes underscores

The Rust implementation actually goes **beyond parity** by normalizing underscores to hyphens per SPDX spec.

---

## Prioritized Action Items

| Priority | Issue | Effort | Impact | Status |
|----------|-------|--------|--------|--------|
| 1 | Remove 50% coverage filter in seq_match | Low | High | Pending |
| 2 | Fix seq_match line number calculation | Low | Medium | Pending |
| 3 | Implement proper match_blocks algorithm | Medium | High | Pending |
| 4 | Implement license reference resolution | High | High | Pending |
| 5 | Add multiple match detection loop | Medium | Medium | Pending |

---

## Completed Issues

### Issue 0: SPDX `+` Suffix (Fixed in `9b47b558`)

**Problem:** SPDX identifiers with `+` suffix (e.g., `GPL-2.0+`) were not detected.

**Solution:**

- Added `spdx_license_key` and `other_spdx_license_keys` fields to Rule
- Built `rid_by_spdx_key` lookup table in LicenseIndex
- Updated `find_best_matching_rule()` to use SPDX key lookup

### Issue 1: License Intro Filtering (Fixed in `f93270b6`)

**Problem:** License expressions incorrectly included "unknown" from intro matches.

**Solution:**

- Added `is_license_intro`, `is_license_clue` fields to LicenseMatch
- Implemented `filter_license_intros()` function
- Updated detection pipeline to filter intros

### Issue 2: Deprecated Rules (Fixed in `3b5ea424`)

**Problem:** Deprecated rules were being used for detection.

**Solution:**

- Added `with_deprecated` parameter to loader functions
- Deprecated items filtered by default

### Issue 4: Overlapping Match Filtering (Fixed in `15b07829`)

**Problem:** Complex overlap scenarios between matches caused incorrect expression combinations.

**Solution:**

- Added `overlap()`, `overlap_ratio()`, `union_span()`, `intersects()` methods to Span
- Added `matcher_order()`, `hilen()`, `surround()` methods to LicenseMatch
- Implemented `filter_overlapping_matches()` with 4 overlap thresholds (10%, 40%, 70%, 90%)
- Implemented `restore_non_overlapping()` to recover non-conflicting discarded matches
- Updated `refine_matches()` pipeline

**Golden test improvement:** +24 passed

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
