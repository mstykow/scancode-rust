# Phase 5: Wrong License Selection - Implementation Plan

**Status:** Planning  
**Created:** 2025-03-01  
**Related:** [0016-feature-parity-roadmap.md](0016-feature-parity-roadmap.md), [0015-filter-dupes-regressions.md](0015-filter-dupes-regressions.md)

## Executive Summary

### Problem Statement

Rust detects a different license than Python for the same text when multiple similar rules match. The root cause is **precision loss in the `matched_length` field of `DupeGroupKey`**, causing incorrect grouping of duplicate candidates.

### Impact

- **~20 golden test failures** where wrong license is selected
- Critical for feature parity with Python ScanCode

### Root Cause

The `filter_dupes()` function groups candidates by a `DupeGroupKey` to eliminate duplicates. Python uses **1-decimal-place precision** for `matched_length`, while Rust uses **0-decimal-place precision**. This causes candidates that should be in different groups to be incorrectly merged.

---

## Detailed Analysis

### Python Implementation

**File:** `reference/scancode-toolkit/src/licensedcode/match_set.py`

#### ScoresVector Creation (line 435-448)

```python
scores = (
    ScoresVector(
        is_highly_resemblant=round(resemblance, 1) >= high_resemblance_threshold,
        containment=round(containment, 1),
        resemblance=round(amplified_resemblance, 1),
        matched_length=round(matched_length / 20, 1),  # 1 decimal place
    ),
    ...
)
```

Key insight: `matched_length` is `matched_length / 20` rounded to **1 decimal place**.

#### group_key Function (line 467-476)

```python
def group_key(item):
    (sv_round, _sv_full), _rid, rule, _inter = item
    return (
        rule.license_expression,
        sv_round.is_highly_resemblant,
        sv_round.containment,       # Already 1 decimal (e.g., 8.0, 7.5)
        sv_round.resemblance,       # Already 1 decimal (e.g., 6.4, 5.2)
        sv_round.matched_length,    # Already 1 decimal (e.g., 6.9, 6.7)
        rule.length,
    )
```

Key insight: The group key uses the **rounded values directly** (as floats with 1 decimal).

### Rust Implementation

**File:** `src/license_detection/seq_match.rs`

#### ScoresVector Creation (line 368-374, 428-434)

```rust
let svr = ScoresVector {
    is_highly_resemblant: (resemblance * 10.0).round() / 10.0 >= HIGH_RESEMBLANCE_THRESHOLD,
    containment: (containment * 10.0).round() / 10.0,
    resemblance: (amplified_resemblance * 10.0).round() / 10.0,
    matched_length: (matched_length as f32 / 20.0).round(),  // 0 decimal places!
    rid,
};
```

**BUG:** `matched_length` is rounded to **0 decimal places** (e.g., `6.9` → `7.0`).

#### DupeGroupKey Creation (line 64-71)

```rust
let key = DupeGroupKey {
    license_expression: candidate.rule.license_expression.clone(),
    is_highly_resemblant: candidate.score_vec_rounded.is_highly_resemblant,
    containment: (candidate.score_vec_rounded.containment * 10.0).round() as i32,
    resemblance: (candidate.score_vec_rounded.resemblance * 10.0).round() as i32,
    matched_length: ((candidate.score_vec_full.matched_length / 20.0) * 10.0).round() as i32,
    rule_length: candidate.rule.tokens.len(),
};
```

**Analysis:** The DupeGroupKey calculation actually produces correct integer values:
- For 138 tokens: `((138/20)*10).round()` = 69
- For 133 tokens: `((133/20)*10).round()` = 67
- These are DIFFERENT, so grouping works correctly

**However**, the source of truth should be `score_vec_rounded.matched_length`, not recomputing from `score_vec_full`. Using the rounded value ensures consistency between sorting and grouping.

### Precision Loss Example

| Tokens | Python sv_round.matched_length | Rust score_vec_rounded.matched_length | Rust DupeGroupKey.matched_length |
|--------|-------------------------------|---------------------------------------|----------------------------------|
| 138    | `round(138/20, 1)` = **6.9** | `round(138/20)` = **7.0** | `((138/20)*10).round()` = **69** |
| 133    | `round(133/20, 1)` = **6.7** | `round(133/20)` = **7.0** | `((133/20)*10).round()` = **67** |

**Analysis:**

1. **Python grouping**: Uses `sv_round.matched_length` (float 6.9 vs 6.7) directly in group key → Different groups → Both candidates survive

2. **Rust grouping**: The DupeGroupKey calculation `((score_vec_full.matched_length / 20.0) * 10.0).round() as i32` actually gives **different values (69 vs 67)**. So grouping is NOT the bug.

3. **The real bug**: `score_vec_rounded.matched_length` precision affects **SORTING**:
   - In Python: 6.9 vs 6.7 affects candidate ranking when sorting by ScoresVector
   - In Rust: 7.0 == 7.0 means the matched_length doesn't differentiate these candidates during sorting
   - This causes wrong candidates to rank higher after filter_dupes

**Root cause**: The ScoresVector.matched_length precision loss causes incorrect sorting, leading to wrong license selection when candidates have similar but different matched_lengths.

---

## Affected Test Cases

| Test File | Expected | Actual (Rust) | Root Cause |
|-----------|----------|---------------|------------|
| `IBM-MIT-style.txt` | `x11-ibm` | `historical` | Wrong group merging |
| `MIT-CMU-style.txt` | `x11-dec1` | `cmu-uc` | matched_length precision |
| `bsd.f` | `bsd-simplified` | `bsd-new` | Candidate selection |
| `MIT.t19` | `proprietary-license` | `mit` | Custom condition not detected |
| `BSD-3-Clause.t26` | `bsd-new` | `bsd-x11` | Similar rule confusion |

---

## Implementation Plan

### Step 1: Fix `matched_length` Precision in ScoresVector

**File:** `src/license_detection/seq_match.rs`

**Location:** Lines 272-278, 368-374, 428-434 (three places where `ScoresVector` is created)

**Current Code:**
```rust
matched_length: (matched_length as f32 / 20.0).round(),
```

**Fixed Code:**
```rust
matched_length: ((matched_length as f32 / 20.0) * 10.0).round() / 10.0,
```

This matches Python's `round(matched_length / 20, 1)`.

### Step 2: Fix DupeGroupKey matched_length (consistency fix)

**File:** `src/license_detection/seq_match.rs`

**Location:** Line 69

**Current Code:**
```rust
matched_length: ((candidate.score_vec_full.matched_length / 20.0) * 10.0).round() as i32,
```

**Fixed Code:**
```rust
matched_length: (candidate.score_vec_rounded.matched_length * 10.0).round() as i32,
```

**Analysis:** The current calculation happens to produce correct integer values, but recomputing from `score_vec_full` is inconsistent with the design intent. Using `score_vec_rounded.matched_length` ensures consistency between the rounded ScoresVector (used for sorting) and the DupeGroupKey (used for grouping).

This converts the 1-decimal-place float to an integer for the hash key:
- `6.9` → `69`
- `6.7` → `67`

### Step 3: Fix containment and resemblance in DupeGroupKey

**File:** `src/license_detection/seq_match.rs`

**Location:** Lines 67-68

**Current Code:**
```rust
containment: (candidate.score_vec_rounded.containment * 10.0).round() as i32,
resemblance: (candidate.score_vec_rounded.resemblance * 10.0).round() as i32,
```

**Analysis:** These are already correct because:
- `score_vec_rounded.containment` is already 1 decimal (e.g., `8.0`)
- `* 10.0` gives `80.0`, rounded = `80`

**No change needed** - these are correct.

### Step 4: Verify rule_length in DupeGroupKey

**File:** `src/license_detection/seq_match.rs`

**Location:** Line 70

**Current Code:**
```rust
rule_length: candidate.rule.tokens.len(),
```

**Python Code:**
```python
rule.length,
```

**Analysis:** In Python, `rule.length` is the total token count. In Rust, `rule.tokens.len()` is the same. **No change needed**.

---

## Code Changes Summary

### File: `src/license_detection/seq_match.rs`

#### Change 1: Line 276 (in `compute_set_similarity`)

```diff
- matched_length: (matched_length as f32 / 20.0).round(),
+ matched_length: ((matched_length as f32 / 20.0) * 10.0).round() / 10.0,
```

#### Change 2: Line 372 (in `compute_candidates_with_msets`, first ScoresVector)

```diff
- matched_length: (matched_length as f32 / 20.0).round(),
+ matched_length: ((matched_length as f32 / 20.0) * 10.0).round() / 10.0,
```

#### Change 3: Line 432 (in `compute_candidates_with_msets`, second ScoresVector)

```diff
- matched_length: (matched_length as f32 / 20.0).round(),
+ matched_length: ((matched_length as f32 / 20.0) * 10.0).round() / 10.0,
```

#### Change 4: Line 69 (in `DupeGroupKey`)

```diff
- matched_length: ((candidate.score_vec_full.matched_length / 20.0) * 10.0).round() as i32,
+ matched_length: (candidate.score_vec_rounded.matched_length * 10.0).round() as i32,
```

---

## Testing Strategy

### Unit Tests

Add unit tests in `src/license_detection/seq_match.rs` to verify precision:

```rust
#[test]
fn test_matched_length_precision() {
    // Test that matched_length uses 1 decimal place
    let ml1 = 138usize;
    let ml2 = 133usize;
    
    let svr1 = ScoresVector {
        matched_length: ((ml1 as f32 / 20.0) * 10.0).round() / 10.0,
        // ... other fields
    };
    let svr2 = ScoresVector {
        matched_length: ((ml2 as f32 / 20.0) * 10.0).round() / 10.0,
        // ... other fields
    };
    
    // Should be different: 6.9 vs 6.7
    assert_ne!(svr1.matched_length, svr2.matched_length);
    assert!((svr1.matched_length - 6.9).abs() < 0.01);
    assert!((svr2.matched_length - 6.7).abs() < 0.01);
}

#[test]
fn test_dupe_group_key_different_groups() {
    // Test that candidates with different matched_length values
    // end up in different groups
    // ...
}
```

### Integration Tests

Run existing debug tests to verify fixes:

```bash
# Run specific debug tests
cargo test test_mit_cmu_style_filter_dupes_debug --lib -- --nocapture
cargo test test_bsd_f_filter_dupes_debug --lib -- --nocapture
cargo test test_mit_t21_filter_dupes_debug --lib -- --nocapture
```

### Golden Tests

Run the golden test suite to verify overall improvement:

```bash
# Run all golden tests
cargo test --release -q --lib license_detection::golden_test

# Run specific failing tests
cargo test --release -q --lib license_detection::golden_test::test_golden_external_part1
cargo test --release -q --lib license_detection::golden_test::test_golden_external_part2
```

### Expected Test Improvements

After the fix, these tests should pass:

| Test File | Expected Result |
|-----------|-----------------|
| `IBM-MIT-style.txt` | `x11-ibm` detected |
| `MIT-CMU-style.txt` | `x11-dec1` detected |
| `bsd.f` | `bsd-simplified` detected |
| `BSD-3-Clause.t26` | `bsd-new` detected |

Note: `MIT.t19` expects `proprietary-license` which is a special case - see Investigation Notes below.

---

## Investigation Notes

### MIT.t19 Case

This test has MIT-like text with additional conditions:
- Line 10-13 add custom conditions to the MIT license
- Python detects it as `proprietary-license` due to the modifications
- Rust detects it as `mit`

**Analysis:** This may require investigation beyond the `matched_length` precision fix. The text has:
```
1- The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.
2- For paid software, you MUST include a reference to the original project...
```

This is a modified MIT with extra conditions, which should trigger `proprietary-license` detection.

**Action:** After fixing the precision issue, investigate if additional logic is needed for this case.

### bsd.f Case

This is a BSD-2-Clause (simplified) license with Fortran-style comments (lines start with `c`):

```fortran
c Copyright (c) 2012, Devscripts developers
c
c Redistribution and use in source and binary forms...
```

**Analysis:** The test expects `bsd-simplified` but Rust detects `bsd-new`. This may be:
1. A candidate selection issue after precision fix
2. A tokenizer issue with Fortran comments
3. A rule matching issue

**Action:** Verify after precision fix. If still failing, investigate tokenizer behavior for Fortran comments.

---

## Risk Assessment

### Low Risk

- The precision fix is a straightforward change
- Well-understood root cause with clear mapping to Python code
- Isolated to `seq_match.rs`

### Medium Risk

- May affect other tests that currently pass (regression risk)
- Requires full golden test suite run to validate

### Mitigation

1. Run full test suite before and after changes
2. Compare results to identify any regressions
3. If regressions occur, analyze whether they reveal other pre-existing issues

---

## Implementation Checklist

- [ ] Change `matched_length` precision in `compute_set_similarity` (line 276)
- [ ] Change `matched_length` precision in first `ScoresVector` in `compute_candidates_with_msets` (line 372)
- [ ] Change `matched_length` precision in second `ScoresVector` in `compute_candidates_with_msets` (line 432)
- [ ] Change `DupeGroupKey.matched_length` calculation (line 69)
- [ ] Add unit tests for precision
- [ ] Run debug tests to verify fix
- [ ] Run full golden test suite
- [ ] Document results
- [ ] Investigate remaining failures (MIT.t19, etc.)

---

## References

- **Python filter_dupes:** `reference/scancode-toolkit/src/licensedcode/match_set.py:461-498`
- **Python ScoresVector:** `reference/scancode-toolkit/src/licensedcode/match_set.py:436-447`
- **Previous Investigation:** `docs/license-detection/0015-filter-dupes-regressions.md`
- **Roadmap:** `docs/license-detection/0016-feature-parity-roadmap.md`
