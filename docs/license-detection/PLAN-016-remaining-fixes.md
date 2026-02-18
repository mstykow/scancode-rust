# PLAN-016: Remaining License Detection Fixes

## Status: PARTIALLY IMPLEMENTED - 26 tests fixed

### Summary of Progress

| Phase | Failures | Tests Fixed |
|-------|----------|-------------|
| Baseline | 103 | - |
| After Phase D | 89 | 14 |
| After Phase E | **77** | **12** |
| **Total** | - | **26** |

---

## Completed Work

### Phase A-C (Previously Completed)

- Implemented `matched_qspans` tracking
- Fixed `hilen()`, implemented `qdensity()`/`idensity()` methods
- Implemented 6 missing filters, fixed matcher string bug

### Phase D: Issues 2 & 5 (Commit: c80c7985)

**Issue 2 - Aho-Corasick Token Boundary Bug** ✅:

- Root cause: Automaton matching across token boundaries
- Fix: Added `byte_start % 2 != 0` check in `aho_match.rs`

**Issue 5 - GPL Variant Confusion** ✅:

- Root cause: Line-based overlap instead of token-based
- Fix: Added `qoverlap()` method, changed to token-based sorting

### Phase E: Issues 1, 3A, 6A/B (Commit: 76fc515e)

**Issue 1 - Match Over-Merging** ✅:

- Root cause: Wrong threshold (3 vs 4) and dual-criteria grouping
- Fix: Changed `should_group_together()` to use line-only with threshold 4
- Removed unused `TOKENS_THRESHOLD` and `LINES_GAP_THRESHOLD` constants

**Issue 3A - Combined Rules Not Matched** ✅:

- Root cause: `matchable_tokens()` using only high matchables
- Fix: Changed `matchables(false)` to `matchables(true)` (one-line fix)

**Issue 6A - Token-based Coverage** ✅:

- Root cause: `compute_covered_positions()` using lines instead of tokens
- Fix: Changed to use `start_token..end_token` range

**Issue 6B - Hispan Threshold** ✅:

- Root cause: Missing `hispan >= 5` check
- Fix: Added hispan computation and threshold check

---

## Current State

| Metric | Value |
|--------|-------|
| lic1 passed | 214 |
| lic1 failed | **77** |

---

## Remaining Issues

### Issue 4: Query Run Double-Matching (NOT YET IMPLEMENTED)

**Problem**: `QueryRun` holds stale references to `query.high_matchables` after `subtract()`.

**Status**: ✅ READY FOR IMPLEMENTATION

**Implementation**: Use lazy evaluation by storing reference to parent `Query`

---

### Remaining Failure Analysis (77 tests)

**Pattern 1: Expression Over-Combination (~30 tests)**

- OR expressions combined with AND: `"(gpl-1.0-plus OR artistic-1.0) AND gpl-1.0 AND artistic-1.0"`
- Expected: Separate expressions
- Likely cause: Expression combination logic in `expression.rs`

**Pattern 2: Over-Grouping (~20 tests)**

- Expressions merged when should be separate: `"gpl-2.0 AND gpl-2.0-plus"`
- Expected: `["gpl-2.0", "gpl-2.0-plus"]`
- Possible cause: Expression combination or grouping still off

**Pattern 3: Missing Detections (~15 tests)**

- Empty or fewer expressions than expected
- `gpl-2.0_30.txt`: Expected `["gpl-1.0-plus"]`, Actual: `[]`
- `gpl_or_mit_1.txt`: Expected `["mit OR gpl-2.0"]`, Actual: `[]`
- Possible cause: Query run issues or combined rule matching

**Pattern 4: Unknown License Issues (~12 tests)**

- Extra `unknown` or `unknown-license-reference` matches
- May need further refinement

---

## Next Steps

1. Analyze remaining 77 failures to identify root causes
2. Implement Issue 4 (Query Run lazy evaluation)
3. Investigate expression combination in `expression.rs`
4. Fine-tune unknown match filtering if needed

---

## Verification Commands

```bash
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic1
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings
```
