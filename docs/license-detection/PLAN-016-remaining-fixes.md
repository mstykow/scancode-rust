# PLAN-016: Remaining License Detection Fixes

## Status: Implementation Complete - Analysis Needed for Remaining Issues

### Summary of Progress

- Baseline: 103 failures
- Current: 102 failures
- Net improvement: 1 test fixed

### Phase A Results (2026-02-17)

- Priority 1: Implemented `matched_qspans` tracking for Phase 1 and Phase 2 matches
- Priority 2: Post-loop logic for `has_unknown_intro_before_detection()` - Implemented
- Query runs remain disabled (cause regression when enabled)

### Phase B Results (2026-02-17)

- Fixed `hilen()` to return high-value token count
- Implemented `qdensity()` and `idensity()` methods

### Phase C Results (2026-02-17)

- Implemented all 6 missing filters
- Fixed matcher string comparison bug (`"4-seq"` → `"3-seq"`)

---

## Current State

| Metric | Value |
|--------|-------|
| lic1 passed | 189 |
| lic1 failed | 102 |

---

## Analysis of Remaining Failures

### Failure Categories

| Category | Count | Description |
|----------|-------|-------------|
| Match over-merging | ~40 | Rust combines matches that Python keeps separate |
| False positives | ~18 | Rust detects licenses Python doesn't |
| False negatives | ~5 | Python detects, Rust doesn't |
| Expression structure | ~25 | Same licenses, different expression format |
| Encoding errors | ~3 | UTF-8 decode failures |
| GPL variant confusion | ~11 | gpl-2.0 vs gpl-2.0-plus distinctions |

### Common Patterns

1. **`cc-by-nc-sa-2.0` false positive** - Appears in many files where it shouldn't
2. **Unknown license references** - Rust generates extra `unknown-license-reference` matches
3. **OR expressions split** - Expected `["A OR B"]`, Actual: `["A", "B"]`
4. **OR to AND conversion** - Expected `["A OR B"]`, Actual: `["A AND (A OR B)"]`

---

## Root Cause Analysis

### Issue 1: Match Merging

**Problem**: Rust's `should_group_together()` uses dual-criteria (token + line thresholds) but Python uses only line threshold.

**Python**: `is_in_group_by_threshold = cur.start_line <= prev.end_line + lines_threshold`

**Rust**: `token_gap <= 10 && line_gap <= 3`

**Note**: Attempted to match Python's logic but it caused regression (103 → 105). Need deeper investigation.

### Issue 2: Expression Combination

**Problem**: Python's expression combination preserves OR expressions, Rust always uses AND.

**Example**: `cddl-1.0_or_gpl-2.0-glassfish.txt`

- Expected: `["cddl-1.0 OR gpl-2.0"]`
- Actual: `["gpl-2.0 AND cddl-1.0", "unknown-license-reference", "unknown"]`

### Issue 3: Combined Rule Matching

**Problem**: Combined rules (e.g., `cddl-1.0_or_gpl-2.0-glassfish`) should match as a single expression, but Rust matches partial rules.

**Root cause**: Query runs are disabled, preventing proper combined rule matching.

### Issue 4: Query Runs Still Problematic

**Problem**: Even with `matched_qspans` tracking, enabling query runs causes regression.

**Hypothesis**: The `is_matchable()` check may not be reading fresh data from Query after `subtract()`.

---

## Next Steps

### Immediate Investigation

1. Why does matching Python's `should_group_together()` logic cause regression?
2. What's the correct expression combination logic?
3. How does Python handle combined rule matching?

### Deferred (Requires Architecture Changes)

- Query runs (need proper reference handling)
- `filter_matches_missing_required_phrases()` (complex, requires Rule integration)

---

## Run Golden Tests

```bash
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic1
```
