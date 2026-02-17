# PLAN-015: Consolidated License Detection Fixes

## Status: Validated (2026-02-17)

---

## Validation Results

### Issue 1: `is_license_intro_match()` and `is_license_clue_match()` Use Wrong Logic

**Status: PARTIALLY CORRECT - Fix needs adjustment**

| Aspect | Finding |
|--------|---------|
| Python Reference Accuracy | ✅ Lines 1250-1262 correctly quoted |
| Rust Current Code Accuracy | ✅ Lines 272-279 correctly identified |
| Proposed Fix | ⚠️ **INCOMPLETE** - Missing `has_unknown` check |

**Problems with proposed fix:**

1. The Rust `Rule` struct does NOT have a `has_unknown` field. Python's `is_unknown_intro()` requires:
   ```python
   license_match.rule.has_unknown and (...)  # has_unknown check is REQUIRED
   ```

2. Python's `has_unknown` is a computed property: `license_expression and 'unknown' in license_expression`

3. The proposed fix should check for `has_unknown` via expression:
   ```rust
   fn is_license_intro_match(match_item: &LicenseMatch) -> bool {
       // Must check has_unknown (computed from license_expression containing "unknown")
       let has_unknown = match_item.license_expression.contains("unknown");
       has_unknown && (
           match_item.is_license_intro
           || match_item.is_license_clue
           || match_item.license_expression == "free-unknown"
       )
   }
   ```

4. **Additional Issue**: The `is_unknown_intro()` function at `detection.rs:490-492` ALSO uses wrong logic:
   ```rust
   // CURRENT (WRONG):
   fn is_unknown_intro(m: &LicenseMatch) -> bool {
       m.matcher.starts_with("5-unknown") && m.rule_identifier.contains("intro")
   }
   ```
   This should also be fixed to use boolean fields.

### Issue 2: SPDX-LID Matches Have Zero Token Positions (Regression)

**Status: CORRECT**

| Aspect | Finding |
|--------|---------|
| Rust Current Code | ✅ Lines 279-280 correctly identified (hardcoded 0, 0) |
| Python Reference | ✅ Python tracks actual token positions via QueryRun.qspan |
| Proposed Fix | ✅ Option B (line-based fallback) is sound |

**Recommended implementation order:**
1. First implement Option B (quick fix) - line-based fallback in `qcontains()`
2. Later implement Option A (proper fix) - track actual token positions

### Issue 3: `is_license_reference` Rules Bypass False Positive Detection

**Status: INCORRECT - Python reference is wrong**

| Aspect | Finding |
|--------|---------|
| Python Reference Quote | ❌ **NOT FOUND** in `is_false_positive()` at detection.py:1162-1239 |
| Actual Python Behavior | ⚠️ `is_license_reference` is handled in `is_candidate_false_positive()` at match.py:2651-2688 |
| Proposed Fix | ⚠️ Needs revision based on correct Python behavior |

**Correct Python reference:**

Python's `is_candidate_false_positive()` at `match.py:2651-2688`:
```python
def is_candidate_false_positive(match, max_length=20, trace=...):
    is_candidate = (
        # only tags, refs, or clues
        (
            match.rule.is_license_reference
            or match.rule.is_license_tag
            or match.rule.is_license_intro
            or match.rule.is_license_clue
        )
        # but not tags that are SPDX license identifiers
        and not match.matcher == '1-spdx-id'
        # exact matches only
        and match.coverage() == 100
        # not too long
        and match.len() <= max_length
    )
    return is_candidate
```

**This is called by `filter_false_positive_license_lists_matches()`, NOT `is_false_positive()`**.

The fix for `is_false_positive()` should be based on the actual Python code at detection.py:1162-1239, which does NOT have `is_license_reference` checks.

**Recommended Fix:**

The `is_license_reference` single-token filtering happens in `is_candidate_false_positive()` which is used by the license list filter. The current Rust `is_false_positive()` implementation is correct per Python. The actual issue is:
1. Single `borceux` matches with `is_license_reference: true` and `rule_length: 1` should be filtered by `filter_false_positive_license_lists_matches()`
2. This filter requires MIN_SHORT_FP_LIST_LENGTH = 15 matches to activate

For single spurious matches, a different approach is needed - possibly adding to `is_false_positive()` or creating a new filter.

### Issue 4: `filter_false_positive_license_lists_matches` Threshold Too High

**Status: CORRECT**

Both Python and Rust use `MIN_SHORT_FP_LIST_LENGTH = 15`. The recommendation to lower to 5 is reasonable for catching smaller license lists.

### Issue 5: Match Grouping Too Aggressive

**Status: NEEDS INVESTIGATION**

This issue is vague. Should be investigated after Issues 1-3 are fixed.

---

## Missing Issues Discovered

### Issue 6: `is_unknown_intro()` Function Uses Wrong Logic

**Location**: `src/license_detection/detection.rs:490-492`

**Problem**: The `is_unknown_intro()` function (distinct from `is_license_intro_match()`) also uses string-based heuristics:

```rust
// CURRENT (WRONG):
fn is_unknown_intro(m: &LicenseMatch) -> bool {
    m.matcher.starts_with("5-unknown") && m.rule_identifier.contains("intro")
}
```

**Python Reference**: `is_unknown_intro()` at detection.py:1250-1262:
```python
def is_unknown_intro(license_match):
    return (
        license_match.rule.has_unknown and
        (
            license_match.rule.is_license_intro or license_match.rule.is_license_clue or
            license_match.rule.license_expression == 'free-unknown'
        )
    )
```

**Fix Required**:
```rust
fn is_unknown_intro(m: &LicenseMatch) -> bool {
    let has_unknown = m.license_expression.contains("unknown");
    has_unknown && (
        m.is_license_intro
        || m.is_license_clue
        || m.license_expression == "free-unknown"
    )
}
```

**Affected Tests**: Same as Issue 1 - both functions are used in detection logic.

---

## Updated Implementation Order

1. **Issue 1 + Issue 6** - Fix BOTH `is_license_intro_match()` AND `is_unknown_intro()` functions (highest impact, must be done together)
2. **Issue 2** - Fix SPDX-LID token positions regression
3. **Issue 3** - Re-investigate after understanding correct Python behavior (may need separate filter for single reference matches)
4. **Issue 5** - Investigate match grouping (if still needed after 1-3)
5. **Issue 4** - Lower filter threshold (if still needed)

---

## Updated Expected Impact

| Issue | Tests Fixed | Priority | Notes |
|-------|-------------|----------|-------|
| Issue 1+6 | ~15-25 | P1 | Must fix both functions together |
| Issue 2 | ~2-3 | P1 | Regression fix |
| Issue 3 | ~5-10 | P2 | Needs correct implementation |
| Issue 5 | ~5-10 | P3 | Investigate after P1 fixes |
| Issue 4 | ~2-3 | P4 | Lower priority |

**Total expected improvement**: ~25-40 additional tests passing

---

## Background

After implementing PLAN-007 through PLAN-014, the golden test results showed:
- lic1: 174 passed, 117 failed → 177 passed, 114 failed (only +3 passed)
- External failures: 919 → 895 (only -24 failures)

Analysis of each plan revealed that several fixes were either not implemented correctly, targeted the wrong problem, or caused regressions. This plan consolidates the remaining work needed.

---

## Issue 1: `is_license_intro_match()` and `is_license_clue_match()` Use Wrong Logic

### Problem

**Severity: Critical** - Affects ~15-20 tests

The functions at `src/license_detection/detection.rs:272-279` use string-based heuristics instead of the boolean fields from `LicenseMatch`:

```rust
// CURRENT (WRONG):
fn is_license_intro_match(match_item: &LicenseMatch) -> bool {
    match_item.matcher.starts_with("5-unknown") || match_item.rule_identifier.contains("intro")
}

fn is_license_clue_match(match_item: &LicenseMatch) -> bool {
    match_item.matcher == "5-unknown" || match_item.rule_identifier.contains("clue")
}
```

This is used in `group_matches_by_region_with_threshold()` to determine match grouping.

### Python Reference

Python's `is_unknown_intro()` at `detection.py:1250-1262`:
```python
def is_unknown_intro(license_match):
    return (
        license_match.rule.has_unknown and
        (
            license_match.rule.is_license_intro or
            license_match.rule.is_license_clue or
            license_match.rule.license_expression == 'free-unknown'
        )
    )
```

Python's `has_correct_license_clue_matches()` at `detection.py:1265-1272`:
```python
def has_correct_license_clue_matches(matches):
    return any(m.rule.is_license_clue for m in matches)
```

### Fix Required

Replace the functions with:

```rust
/// Check if a match is a license intro for grouping purposes.
/// Based on Python: is_unknown_intro() at detection.py:1250-1262
fn is_license_intro_match(match_item: &LicenseMatch) -> bool {
    match_item.is_license_intro
        || match_item.is_license_clue
        || match_item.license_expression == "free-unknown"
}

/// Check if a match is a license clue for grouping purposes.
/// Based on Python: has_correct_license_clue_matches() at detection.py:1265-1272
fn is_license_clue_match(match_item: &LicenseMatch) -> bool {
    match_item.is_license_clue
}
```

### Affected Tests

Tests in FAILURES.md mentioning these functions:
- `CRC32.java`
- `checker-2200.txt`
- `cjdict-liconly.txt`
- `cpl-1.0_5.txt`
- `diaspora_copyright.txt`
- `discourse_COPYRIGHT.txt`
- `e2fsprogs.txt`
- `genivi.c`
- `gfdl-1.1_1.RULE`
- `gfdl-1.3_2.RULE`
- `godot_COPYRIGHT.txt`
- `gpl-2.0_and_lgpl-2.0.txt`
- `gpl-2.0_or_bsd-new_intel_kernel.c`

### TODOs

- [ ] Replace `is_license_intro_match()` implementation
- [ ] Replace `is_license_clue_match()` implementation
- [ ] Add unit tests for both functions
- [ ] Run `cargo test --release --lib license_detection::detection`
- [ ] Verify golden test improvement

---

## Issue 2: SPDX-LID Matches Have Zero Token Positions (Regression)

### Problem

**Severity: Critical** - Causes regression, affects ~2-3 tests

The SPDX-LID matcher at `src/license_detection/spdx_lid.rs:279-280` hardcodes token positions to 0:

```rust
start_token: 0,
end_token: 0,
```

The `qcontains()` method at `models.rs` checks containment using token positions. When both matches have `(0, 0)`, `qcontains()` returns true in both directions, causing `filter_contained_matches()` to incorrectly filter SPDX matches as duplicates.

### Example

`gpl_or_mit_1.txt`:
- Expected: `["mit OR gpl-2.0"]`
- Actual: `[]` (both SPDX matches filtered out)

### Python Reference

Python tracks actual token positions for SPDX-LID matches via `QueryRun.qspan`.

### Fix Options

**Option A (Preferred)**: Track actual token positions for SPDX-LID matches
- Requires computing token positions from the match location
- Most accurate, matches Python behavior

**Option B**: Use line-based fallback for zero token positions
- In `qcontains()`, if both spans are `(0, 0)`, fall back to line-based containment
- Simpler but less accurate

**Option C**: Skip containment filtering for SPDX-LID matches
- Add a flag to skip containment check for certain matcher types
- Quick fix but doesn't address root cause

### Recommended Fix

Implement Option B as quick fix, then Option A for proper fix:

```rust
// In qcontains() method:
pub fn qcontains(&self, other: &LicenseMatch) -> bool {
    // Handle zero token positions (SPDX-LID matches) with line-based fallback
    if self.start_token == 0 && self.end_token == 0 && other.start_token == 0 && other.end_token == 0 {
        // Fall back to line-based containment
        return self.start_line <= other.start_line && self.end_line >= other.end_line;
    }
    self.start_token <= other.start_token && self.end_token >= other.end_token
}
```

### Affected Tests

- `gpl_or_mit_1.txt` (returns empty due to filtering)
- Other tests with SPDX-LID matches

### TODOs

- [ ] Implement fallback in `qcontains()` for zero token positions
- [ ] Add unit tests for SPDX match containment edge cases
- [ ] Consider proper token position tracking for SPDX-LID (future)
- [ ] Verify regression is fixed

---

## Issue 3: `is_license_reference` Rules Bypass False Positive Detection

### Problem

**Severity: Medium** - Affects ~10-15 tests

The `is_false_positive()` function checks `is_license_tag` for single-token filtering, but NOT `is_license_reference`. Rules like `borceux_1.RULE` have `is_license_reference: true` with `relevance: 50` and bypass all false positive checks.

### Example

`gpl-2.0-plus_11.txt`: Expected `["gpl-2.0-plus"]`, actual `["borceux AND gpl-2.0-plus"]`

The `borceux` match:
- `is_license_reference: true`
- `rule_relevance: 50`
- `matched_length: 1`, `rule_length: 1`
- Passes through `is_false_positive()` because relevance >= 60 check is for ALL matches, not single match

### Python Reference

Python's `is_false_positive()` at `detection.py:1162-1220`:
```python
# Has specific handling for is_license_reference rules
if match.rule.is_license_reference and match.rule.length == 1:
    return True
```

Also, Python has `filter_false_positive_license_lists_matches()` that filters sequences of `is_license_reference` matches.

### Fix Required

Add `is_license_reference` check to `is_false_positive()`:

```rust
fn is_false_positive(matches: &[LicenseMatch]) -> bool {
    if matches.is_empty() {
        return false;
    }

    // ... existing checks ...

    // Add: Check for single-token is_license_reference matches with low relevance
    let is_single_reference = matches.len() == 1
        && matches[0].is_license_reference
        && matches[0].rule_length == 1
        && matches[0].rule_relevance < 90;

    if is_single_reference {
        return true;
    }

    // ... rest of function ...
}
```

### Affected Tests

- `gpl-2.0-plus_11.txt`
- `gpl-2.0-plus_17.txt`
- `gpl-2.0-plus_28.txt`
- `gpl_26.txt`
- `gpl_35.txt`
- `complex.el`
- Tests with false `borceux` matches

### TODOs

- [ ] Add `is_license_reference` check to `is_false_positive()`
- [ ] Determine correct relevance threshold (Python uses 90?)
- [ ] Add unit tests
- [ ] Verify affected tests pass

---

## Issue 4: `filter_false_positive_license_lists_matches` Threshold Too High

### Problem

**Severity: Low** - Affects edge cases only

The implemented filter requires `MIN_SHORT_FP_LIST_LENGTH = 15` candidates to filter, but most failing tests have only 1-2 spurious matches.

### Analysis

The filter is designed for files like SPDX license list JSON files with 50+ license identifiers. For single spurious matches, the `is_false_positive()` fix (Issue 3) should handle them.

### Recommendation

Lower threshold to 5 candidates, but prioritize Issue 3 first:

```rust
const MIN_SHORT_FP_LIST_LENGTH: usize = 5;  // Was 15
```

### TODOs

- [ ] Lower threshold after Issue 3 is fixed
- [ ] Add test case for small license lists
- [ ] Verify no regression

---

## Issue 5: Match Grouping Too Aggressive

### Problem

**Severity: Medium** - Affects ~5-10 tests

Matches are being grouped together that should be separate detections. The `should_group_together()` function may need adjustment.

### Example

`gpl-2.0_82.RULE`: Python produces 3 detections, Rust produces 1.

### Analysis

This may be related to Issue 1 (intro/clue detection) or may need separate investigation.

### TODOs

- [ ] Investigate after Issue 1 is fixed
- [ ] Compare Python's `get_matching_regions()` behavior
- [ ] Determine if token/line thresholds need adjustment

---

## Implementation Order

1. **Issue 1** - Fix `is_license_intro_match()` / `is_license_clue_match()` (highest impact)
2. **Issue 2** - Fix SPDX-LID token positions regression
3. **Issue 3** - Add `is_license_reference` to `is_false_positive()`
4. **Issue 5** - Investigate match grouping (if still needed after 1-3)
5. **Issue 4** - Lower filter threshold (if still needed)

---

## Expected Impact

| Issue | Tests Fixed | Priority |
|-------|-------------|----------|
| Issue 1 | ~15-20 | P1 |
| Issue 2 | ~2-3 (regression fix) | P1 |
| Issue 3 | ~10-15 | P2 |
| Issue 5 | ~5-10 | P3 |
| Issue 4 | ~2-3 | P4 |

**Total expected improvement**: ~25-35 additional tests passing

---

## Verification Checklist

After implementing each issue:

- [ ] Unit tests pass for the module
- [ ] Clippy shows no warnings
- [ ] Golden test suite shows improvement
- [ ] No regression in previously passing tests
