# PLAN-039: Fix Stale Test `test_is_false_positive_single_license_reference_short`

**Status:** Draft  
**Priority:** P3 (Low - Test Fix)  
**Estimated Effort:** Low (15 minutes)  
**Affected Tests:** 1 test  
**Last Updated:** 2026-02-23  

---

## 1. Problem Statement

The test `test_is_false_positive_single_license_reference_short` fails after the implementation of PLAN-031 (score formula) and PLAN-034 (copyright check). The test expects a match with `is_license_reference = true` and `rule_length = 1` to be filtered as a false positive, but the `is_false_positive()` function no longer performs this check.

### Test Failure

```
test license_detection::detection::tests::test_is_false_positive_single_license_reference_short ... FAILED

assertion failed: is_false_positive(&matches)
```

---

## 2. Root Cause Analysis

### 2.1 Historical Context

The test was created in commit `9331a295` (PLAN-015 Priority 5) to verify a "Check 5" that filtered single `is_license_reference` matches with short rule length:

```rust
// Check 5: Single is_license_reference match with short rule length
// This filters false positives like "borceux" matching the word "GPL"
if is_single
    && matches[0].is_license_reference
    && matches[0].rule_length <= FALSE_POSITIVE_RULE_LENGTH_THRESHOLD
{
    return true;
}
```

### 2.2 Check Was Intentionally Removed

Commit `1b6dece6` (PLAN-026) **intentionally removed** this check because it was incorrectly filtering valid short license reference matches:

> "Remove extra is_license_reference rule_length <= 3 check. This was incorrectly filtering valid short license reference matches. Fixes lgpl_21.txt detection: [] -> ["lgpl-2.0-plus"]"

**Results after removal:**
- lic1: 58->57 failures
- lic2: 49->48 failures
- lic4: 50->48 failures

### 2.3 Test Was Never Updated

When PLAN-026 removed the check, the test that verified it was never updated. The test now expects behavior that was intentionally removed.

### 2.4 Python Reference Comparison

The Python `is_false_positive()` function at `detection.py:1162-1239` does NOT have a separate check for `is_license_reference`. It only checks `is_license_tag`:

```python
matches_is_license_tag_flags = all(
    license_match.rule.is_license_tag for license_match in license_matches
)

# ...

if matches_is_license_tag_flags and all_match_rule_length_one:
    return True
```

**Key insight**: Python distinguishes between `is_license_tag` (SPDX identifiers in package manifests) and `is_license_reference` (bare names/URLs). The Rust implementation correctly follows Python by only checking `is_license_tag`.

### 2.5 Why Test Still Exists

The test was added to verify a custom Rust check that was later found to be incorrect. The check was removed, but the test was not updated to reflect the removal.

---

## 3. Proposed Fix

### 3.1 Option A: Update Test Expectation (Recommended)

The test expectation should be changed from `assert!(is_false_positive(&matches))` to `assert!(!is_false_positive(&matches))`.

**Reasoning:**
- The check was intentionally removed in PLAN-026
- The removal fixed real detection failures (lgpl_21.txt)
- Python reference does not have this check
- The test should verify current behavior, not old (removed) behavior

**Updated test:**

```rust
#[test]
fn test_is_false_positive_single_license_reference_short() {
    let mut m = create_test_match_with_params(
        "borceux",
        "2-aho",
        1,
        10,
        100.0,
        1,
        1,
        100.0,
        80,
        "borceux.LICENSE",
    );
    m.is_license_reference = true;
    m.rule_length = 1;
    let matches = vec![m];
    // This should NOT be filtered as false positive since:
    // 1. Python doesn't have this check (only checks is_license_tag)
    // 2. The Rust check was removed in PLAN-026 because it incorrectly
    //    filtered valid matches like lgpl-2.0-plus
    assert!(
        !is_false_positive(&matches),
        "Short is_license_reference matches should NOT be filtered (PLAN-026 removed this check)"
    );
}
```

### 3.2 Option B: Remove Test Entirely

Since the test was specifically for a check that no longer exists, we could remove the test entirely.

**Reasoning for removal:**
- The test verifies behavior that was intentionally removed
- There are similar tests for `is_license_tag` which IS the correct check

**Reasons to keep (Option A preferred):**
- The test documents expected behavior for `is_license_reference` matches
- It serves as a regression test to prevent re-adding the incorrect check

### 3.3 Option C: Rename Test for Clarity

Rename the test to clearly indicate it tests that `is_license_reference` is NOT filtered:

```rust
#[test]
fn test_is_false_positive_single_license_reference_short_not_filtered() {
    // ...
}
```

---

## 4. Verification Steps

### 4.1 Run the Fixed Test

```bash
cargo test --lib test_is_false_positive_single_license_reference_short
```

Expected: Test passes with the updated assertion.

### 4.2 Run Related Tests

```bash
cargo test --lib test_is_false_positive_single_license_reference
```

This runs all three related tests:
1. `test_is_false_positive_single_license_reference_short` - Should now pass
2. `test_is_false_positive_single_license_reference_long_rule` - Should still pass
3. `test_is_false_positive_single_license_reference_full_relevance` - Should still pass

### 4.3 Run Full Test Suite

```bash
cargo test --lib
```

Ensure no other tests are affected.

### 4.4 Run Golden Tests

```bash
cargo test --license-detection-golden
```

Verify no regressions in golden test results.

---

## 5. Implementation Checklist

- [ ] Update test expectation in `src/license_detection/detection.rs`
- [ ] Add comment explaining why the check was removed
- [ ] Run `cargo test --lib test_is_false_positive_single_license_reference_short`
- [ ] Run `cargo test --lib` to verify all tests pass
- [ ] Run `cargo clippy` to verify no warnings

---

## 6. Related Documents

- **PLAN-015**: Originally added the `is_license_reference` check (commit `9331a295`)
- **PLAN-026**: Removed the check as incorrect (commit `1b6dece6`)
- **PLAN-031**: Score formula fix (after which test started failing)
- **PLAN-034**: Copyright check fix
- **Python reference**: `reference/scancode-toolkit/src/licensedcode/detection.py:1162-1239`

---

## 7. Summary

The test `test_is_false_positive_single_license_reference_short` fails because it expects behavior that was intentionally removed in PLAN-026. The `is_license_reference` check was removed because it incorrectly filtered valid short license reference matches. The Python reference does not have this check either.

**Fix**: Update the test expectation to verify that short `is_license_reference` matches are NOT filtered as false positives.
