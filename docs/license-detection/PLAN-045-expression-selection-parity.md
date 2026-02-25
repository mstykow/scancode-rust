# PLAN-045: Expression Selection Parity for Overlapping Matches

**Status: ⚠️ NEEDS REVISION - CDDL FIX INCOMPLETE**

## Executive Summary

**PLAN-056 Fix Status:** The CDDL fix (`qoverlap()`, `qcontains()`, surround merge) was implemented but is **NOT sufficient**.

**What PLAN-056 Fixed:**
- `qoverlap()` now correctly computes position overlap (not just range overlap)
- `qcontains()` handles mixed `qspan_positions` modes
- Surround merge in `merge_overlapping_matches()` checks `qoverlap > 0`

**What Still Fails:**
- Test file: `cddl-1.0_or_gpl-2.0-glassfish.txt`
- Expected: `cddl-1.0 OR gpl-2.0`
- Actual: `cddl-1.1 OR gpl-2.0 WITH classpath-exception-2.0`

**Root Cause (VERIFIED 2026-02-25):**
The bug is in `filter_overlapping_matches()` selection logic. The filter is choosing CDDL 1.1 (lower quality) over CDDL 1.0 (higher quality) due to incorrect overlap ratio comparison.

### Key Evidence from `test_detect_all_phases`:

```
CDDL 1.0: coverage=96.2%, start=18, end=270, matched_length=252, qspan_positions=None
CDDL 1.1: coverage=59.0%, start=0, end=270, matched_length=174, qspan_positions=Some(174)

overlap: 164 positions
overlap_ratio_to_next (CDDL 1.0): 0.651 (164/252)
overlap_ratio_to_current (CDDL 1.1): 0.943 (164/174)

Decision: extra_large_current=true && current_len(174) <= next_len(252)
-> Keeps CDDL 1.1, removes CDDL 1.0 (WRONG)
```

**The Problem:** The logic compares overlap ratio against the CURRENT match's length, not considering that the NEXT match has higher coverage and more matched tokens.

---

## Investigation Results (VERIFIED 2026-02-25)

### Both Rules Match, But Wrong One Selected

**Test file:** `testdata/license-golden/datadriven/lic1/cddl-1.0_or_gpl-2.0-glassfish.txt`

**Phase 2 & 3 Matching Results:**
- CDDL 1.0 matches: `coverage=96.2%`, `matched_length=252`, `qspan_positions=None`
- CDDL 1.1 matches: `coverage=59.0%`, `matched_length=174`, `qspan_positions=Some(174 positions)`

**Both matches exist before refinement!** The issue is in `filter_overlapping_matches()`.

### Selection Logic Bug

**File:** `src/license_detection/match_refine.rs`

The filter sorts matches and iterates comparing `current` vs `next`. For CDDL:

1. CDDL 1.1 comes first in sort order (lower `start_token=0` vs `start_token=18`)
2. `qoverlap()` returns 164 positions correctly
3. `overlap_ratio_to_current = 164/174 = 0.943` (very high)
4. `overlap_ratio_to_next = 164/252 = 0.651` (lower)
5. Code checks: `extra_large_current && current_len <= next_len`
6. This evaluates to `true && 174 <= 252 = true`
7. CDDL 1.0 is removed, CDDL 1.1 kept

**The bug:** The logic keeps the match with higher overlap ratio to itself, ignoring that the other match has better quality metrics (higher coverage, more matched tokens).

---

## Required Fix

### Option A: Fix Selection Logic in `filter_overlapping_matches()`

The selection logic should prefer the match with:
1. Higher `match_coverage` (quality metric)
2. More `matched_length` (more tokens matched)
3. NOT just higher overlap ratio to itself

**Current buggy logic** (simplified):
```rust
if extra_large_current && current_len <= next_len {
    // Keeps current, removes next - WRONG for CDDL case
}
```

**Proposed fix:** When overlap ratio is high for both matches, compare quality metrics:
```rust
if extra_large_current && current_len <= next_len {
    // Check if next has better quality before removing
    let current_quality = current.match_coverage;
    let next_quality = next.match_coverage;
    if next_quality > current_quality + 5.0 {  // threshold
        // Next is better quality, keep next instead
        result.push(next);
        continue;
    }
}
```

### Option B: Check Python Reference for Exact Logic

Python's `filter_overlapping_matches()` may have additional quality checks we're missing. Need to compare:

1. Python's overlap ratio threshold logic
2. Python's quality comparison between overlapping matches
3. Python's handling of scattered vs contiguous qspan positions

---

## Previous Implementation Attempt (2026-02-24)

**What was tried:**
1. Added `qspan_equal()` helper function using HashSet comparison
2. Added `compare_match_quality()` function
3. Updated `filter_contained_matches()` to use qspan equality

**Why it caused regression:**
- CDDL 1.0 and CDDL 1.1 have **different qspans** (different token positions in the license rules)
- The qspan equality check correctly found them as NOT equal
- Both matches passed through refinement correctly (according to original diagnosis)
- **BUT:** This diagnosis appears to be incorrect based on actual test output

### Verified Root Cause Analysis

#### CDDL 1.0 vs CDDL 1.1 Spans

The CDDL 1.0 and CDDL 1.1 license rules have **different text** (the version identifier differs). When matched against the same query text:

| Property | CDDL 1.0 Match | CDDL 1.1 Match |
|----------|----------------|----------------|
| `qspan` (query positions) | Different | Different |
| `license_expression` | `cddl-1.0` | `cddl-1.1` |
| `match_coverage` | May differ | May differ |

Because:
1. The rules have slightly different token sequences
2. Matching produces different `ispan` and `qspan` values
3. Neither expression subsumes the other via `licensing_contains()`

#### Detection-Level Bug

**File:** `src/license_detection/detection.rs:1146-1184`

```rust
pub fn apply_detection_preferences(detections: Vec<LicenseDetection>) -> Vec<LicenseDetection> {
    let mut processed: std::collections::HashMap<String, (f32, u8, LicenseDetection)> =
        std::collections::HashMap::new();

    for detection in detections {
        let expr = detection
            .license_expression
            .clone()
            .unwrap_or_else(String::new);
        // ...
        processed.insert(expr, (score, best_matcher_priority, detection));
    }
    // ...
}
```

**Problem:** The key is `expr` (license expression string). Different expressions → different entries → both kept.

#### Python's Approach

Python does NOT have this problem because:

1. **Match refinement removes one match before detection**: When two matches have equal or nearly-equal qspans, `filter_contained_matches()` or `filter_overlapping_matches()` removes the inferior one.

2. **Key insight**: Python's `Span` object comparison includes the **actual positions**, not just bounds. If matches have identical position sets, they're considered equal regardless of expression.

3. **But Python ALSO handles this at detection level**: The detection grouping uses `identifier` which is based on position hash, not just expression.

---

## Status: PARTIALLY READY - NEEDS SELECTION LOGIC FIX

**What's fixed:**
- ✅ `qoverlap()` computes actual position overlap
- ✅ `qcontains()` handles mixed qspan_positions modes
- ✅ Surround merge checks overlap before combining

**What's broken:**
- ❌ `filter_overlapping_matches()` selection logic chooses lower-quality match

**Next steps:**
1. Read Python's `filter_overlapping_matches()` implementation
2. Identify what quality checks Python uses for overlapping matches
3. Implement same quality comparison in Rust

---

## Related Plans

- **PLAN-056**: CDDL Rule Selection Investigation (partial fix - qoverlap/qcontains)
- **PLAN-058**: Lic2 Duplicate Merge Regression (caused by PLAN-056 qcontains changes)

---

## Original Implementation Plan (SUPERSEDED BY PLAN-056)

*The following was the original plan. PLAN-056 implemented the qoverlap/qcontains fixes but the selection logic still needs work.*

### Approach: Add Region-Based Deduplication at Detection Level

The fix must handle cases where:
- Two matches have **different expressions** that don't subsume each other
- Two matches have **overlapping or identical file regions**
- Both matches survive match refinement (correct behavior)

### Step 1: Add Region Overlap Detection Helper

**File:** `src/license_detection/detection.rs`

Add a helper to detect when two detections represent the same file region:

```rust
fn detections_have_same_region(a: &LicenseDetection, b: &LicenseDetection) -> bool {
    match (&a.file_region, &b.file_region) {
        (Some(ra), Some(rb)) => {
            ra.start_line == rb.start_line && ra.end_line == rb.end_line
        }
        _ => false,
    }
}

fn detections_overlap_significantly(a: &LicenseDetection, b: &LicenseDetection) -> bool {
    match (&a.file_region, &b.file_region) {
        (Some(ra), Some(rb)) => {
            let overlap_start = ra.start_line.max(rb.start_line);
            let overlap_end = ra.end_line.min(rb.end_line);
            if overlap_start > overlap_end {
                return false;
            }
            let overlap = overlap_end - overlap_start + 1;
            let a_len = ra.end_line - ra.start_line + 1;
            let b_len = rb.end_line - rb.start_line + 1;
            let min_len = a_len.min(b_len);
            min_len > 0 && overlap as f64 / min_len as f64 >= 0.95
        }
        _ => false,
    }
}
```

### Step 2: Add Match-Level Quality Comparison

**File:** `src/license_detection/detection.rs`

Add a function to compare which detection is "better":

```rust
fn compare_detection_quality(a: &LicenseDetection, b: &LicenseDetection) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    
    let score_a = compute_detection_score(&a.matches);
    let score_b = compute_detection_score(&b.matches);
    
    if (score_a - score_b).abs() > 0.01 {
        return score_a.partial_cmp(&score_b).unwrap_or(Ordering::Equal);
    }
    
    let coverage_a = compute_detection_coverage(&a.matches);
    let coverage_b = compute_detection_coverage(&b.matches);
    
    if (coverage_a - coverage_b).abs() > 0.01 {
        return coverage_a.partial_cmp(&coverage_b).unwrap_or(Ordering::Equal);
    }
    
    let matcher_priority_a = a.matches.iter()
        .map(|m| get_matcher_priority(&m.matcher))
        .min()
        .unwrap_or(5);
    let matcher_priority_b = b.matches.iter()
        .map(|m| get_matcher_priority(&m.matcher))
        .min()
        .unwrap_or(5);
    
    if matcher_priority_a != matcher_priority_b {
        return matcher_priority_a.cmp(&matcher_priority_b);
    }
    
    let rule_id_a = a.matches.first()
        .map(|m| &m.rule_identifier)
        .cloned()
        .unwrap_or_default();
    let rule_id_b = b.matches.first()
        .map(|m| &m.rule_identifier)
        .cloned()
        .unwrap_or_default();
    
    rule_id_a.cmp(&rule_id_b)
}
```

### Step 3: Update apply_detection_preferences()

**File:** `src/license_detection/detection.rs`

Modify `apply_detection_preferences()` to handle overlapping detections:

```rust
pub fn apply_detection_preferences(detections: Vec<LicenseDetection>) -> Vec<LicenseDetection> {
    if detections.len() < 2 {
        return detections;
    }
    
    let mut processed: std::collections::HashMap<String, LicenseDetection> =
        std::collections::HashMap::new();
    
    let mut sorted_detections: Vec<LicenseDetection> = detections;
    sorted_detections.sort_by(|a, b| {
        compare_detection_quality(b, a)
    });
    
    for detection in sorted_detections {
        let expr = detection
            .license_expression
            .clone()
            .unwrap_or_else(String::new);
        
        let existing_with_same_expr = processed.get(&expr);
        let existing_with_overlapping_region = processed.values()
            .find(|existing| detections_overlap_significantly(existing, &detection));
        
        if let Some(existing) = existing_with_same_expr {
            if compare_detection_quality(&detection, existing) == std::cmp::Ordering::Greater {
                processed.insert(expr, detection);
            }
        } else if let Some(existing) = existing_with_overlapping_region {
            if compare_detection_quality(&detection, existing) == std::cmp::Ordering::Greater {
                let existing_expr = existing.license_expression.clone().unwrap_or_default();
                processed.remove(&existing_expr);
                processed.insert(expr, detection);
            }
        } else {
            processed.insert(expr, detection);
        }
    }
    
    processed.into_values().collect()
}
```

### Step 4: Add Unit Tests

**File:** `src/license_detection/detection.rs` (in `#[cfg(test)] mod tests`)

```rust
#[test]
fn test_deduplicate_overlapping_detections_different_expressions() {
    let m1 = create_test_match_with_expression(1, 10, "cddl-1.0");
    let m2 = create_test_match_with_expression(1, 10, "cddl-1.1");
    
    let d1 = create_detection_from_matches(vec![m1]);
    let d2 = create_detection_from_matches(vec![m2]);
    
    let result = apply_detection_preferences(vec![d1, d2]);
    
    assert_eq!(result.len(), 1, "Should keep only one detection for same region");
}

#[test]
fn test_keep_better_detection_when_overlapping() {
    let m1 = create_test_match_with_score_and_coverage(1, 10, "cddl-1.0", 95.0, 90.0);
    let m2 = create_test_match_with_score_and_coverage(1, 10, "cddl-1.1", 98.0, 95.0);
    
    let d1 = create_detection_from_matches(vec![m1]);
    let d2 = create_detection_from_matches(vec![m2]);
    
    let result = apply_detection_preferences(vec![d1, d2]);
    
    assert_eq!(result.len(), 1);
    assert!(result[0].license_expression.as_ref().unwrap().contains("cddl-1.1"));
}

#[test]
fn test_keep_both_non_overlapping_detections() {
    let m1 = create_test_match(1, 10, "1-hash", "mit.LICENSE");
    let m2 = create_test_match(20, 30, "1-hash", "apache.LICENSE");
    
    let d1 = create_detection_from_matches(vec![m1]);
    let d2 = create_detection_from_matches(vec![m2]);
    
    let result = apply_detection_preferences(vec![d1, d2]);
    
    assert_eq!(result.len(), 2, "Should keep both non-overlapping detections");
}

fn create_test_match_with_expression(start_line: usize, end_line: usize, expr: &str) -> LicenseMatch {
    LicenseMatch {
        rid: 0,
        license_expression: expr.to_string(),
        license_expression_spdx: expr.to_string(),
        from_file: Some("test.txt".to_string()),
        start_line,
        end_line,
        start_token: start_line,
        end_token: end_line + 1,
        matcher: "1-hash".to_string(),
        score: 95.0,
        matched_length: 100,
        match_coverage: 95.0,
        rule_relevance: 100,
        rule_identifier: format!("{}.LICENSE", expr),
        rule_url: "https://example.com".to_string(),
        matched_text: Some(format!("{} license", expr)),
        referenced_filenames: None,
        is_license_intro: false,
        is_license_clue: false,
        is_license_reference: false,
        is_license_tag: false,
        is_license_text: false,
        rule_length: 100,
        matched_token_positions: None,
        hilen: 50,
        rule_start_token: 0,
        qspan_positions: None,
        ispan_positions: None,
    }
}

fn create_detection_from_matches(matches: Vec<LicenseMatch>) -> LicenseDetection {
    let start_line = matches.iter().map(|m| m.start_line).min().unwrap_or(0);
    let end_line = matches.iter().map(|m| m.end_line).max().unwrap_or(0);
    
    let expr = matches.iter()
        .map(|m| m.license_expression.as_str())
        .collect::<Vec<_>>()
        .join(" AND ");
    
    LicenseDetection {
        license_expression: Some(expr.clone()),
        license_expression_spdx: Some(expr),
        matches,
        detection_log: vec![],
        identifier: None,
        file_region: Some(FileRegion {
            path: "test.txt".to_string(),
            start_line,
            end_line,
        }),
    }
}
```

### Step 5: Verify with CDDL Golden Tests

```bash
cargo test test_license_golden_cddl -- --nocapture
```

Expected: All CDDL glassfish tests should now pass with a single expression.

---

## Testing Strategy

Following `docs/TESTING_STRATEGY.md`:

### Unit Tests

**Location:** `src/license_detection/detection.rs` in `#[cfg(test)] mod tests`

| Test | Purpose |
|------|---------|
| `test_deduplicate_overlapping_detections_different_expressions` | Verify deduplication when expressions differ |
| `test_keep_better_detection_when_overlapping` | Verify quality-based selection |
| `test_keep_both_non_overlapping_detections` | Verify non-overlapping kept separate |
| `test_detections_overlap_significantly` | Test overlap calculation |
| `test_compare_detection_quality` | Test quality comparison logic |

### Golden Tests

**Location:** `testdata/license-golden/datadriven/lic1/`

| Test File | Expected Behavior |
|-----------|-------------------|
| `cddl-1.0_or_gpl-2.0-classpath_and_apache-2.0-glassfish_1.txt` | Single expression |
| `cddl-1.0_or_gpl-2.0-classpath_and_apache-2.0-glassfish_2.txt` | Single expression |
| `cddl-1.1_or_gpl-2.0-classpath_and_apache-2.0-glassfish_1.txt` | Single expression |
| `cddl-1.1_or_gpl-2.0-classpath_and_apache-2.0-glassfish_2.txt` | Single expression |

### Regression Tests

Run all golden tests to ensure no regressions:

```bash
cargo test --test license_golden_test
```

---

## Alternative Approaches Considered

### Option A: Fix in match_refine.rs (Previous Attempt)

**Problem:** CDDL 1.0 and CDDL 1.1 have different qspans, so qspan equality doesn't help.

### Option B: Modify filter_overlapping_matches for Near-Equal Spans

**Problem:** The overlap threshold approach is complex and may have unintended side effects on legitimate overlapping matches.

### Option C: Detection-Level Region Deduplication (Recommended)

**Advantages:**
- Clean separation of concerns
- Handles all cases where matches survive refinement
- Easy to test and verify
- Matches Python's detection-level behavior

---

## Key Files to Modify

| File | Purpose | Lines |
|------|---------|-------|
| `src/license_detection/detection.rs` | Add region deduplication | ~50 new lines |
| `src/license_detection/detection.rs` | Add unit tests | ~80 new lines |

---

## Verification Checklist

- [ ] `detections_overlap_significantly()` helper added
- [ ] `compare_detection_quality()` helper added  
- [ ] `apply_detection_preferences()` updated to handle overlapping regions
- [ ] Unit tests for new functions pass
- [ ] CDDL glassfish golden tests pass
- [ ] No regressions in other golden tests
- [ ] Code passes `cargo clippy`
- [ ] Code formatted with `cargo fmt`

---

## Summary

| Question | Answer |
|----------|--------|
| **Is the plan ready for implementation?** | ⚠️ PARTIAL |
| **What's fixed?** | `qoverlap()`, `qcontains()`, surround merge |
| **What's still broken?** | `filter_overlapping_matches()` selection logic |
| **Root cause?** | Overlap ratio comparison ignores quality metrics |
| **Next step?** | Implement quality comparison for overlapping matches |

---

## Verification Checklist (UPDATED 2026-02-25)

- [x] `qoverlap()` computes actual position overlap (PLAN-056)
- [x] `qcontains()` handles mixed qspan_positions (PLAN-056)
- [x] Surround merge checks qoverlap > 0 (PLAN-056)
- [ ] **BLOCKED:** `filter_overlapping_matches()` quality comparison
- [ ] CDDL glassfish golden tests pass
- [ ] No regressions in other golden tests
- [ ] Code passes `cargo clippy`
- [ ] Code formatted with `cargo fmt`

| Function | File | Lines | Purpose |
|----------|------|-------|---------|
| `filter_overlapping_matches()` | `match_refine.rs` | ~450-650 | **NEEDS FIX** - quality comparison |
| `filter_contained_matches()` | `match.py` | 1075-1184 | Remove contained matches by qspan equality |
| `get_detections_by_id()` | `detection.py` | ~950 | Group detections by identifier |

## Appendix: Rust Code References

| Function | File | Lines | Purpose |
|----------|------|-------|---------|
| `filter_overlapping_matches()` | `match_refine.rs` | ~450-650 | **NEEDS FIX** - selection logic |
| `qoverlap()` | `models.rs` | 547-584 | ✅ Fixed - actual position overlap |
| `qcontains()` | `models.rs` | 518-545 | ✅ Fixed - mixed qspan_positions |
| `apply_detection_preferences()` | `detection.rs` | 1146-1184 | Detection-level processing |
| `compute_detection_score()` | `detection.rs` | 634-658 | Detection scoring |
| `compute_detection_coverage()` | `detection.rs` | 1091-1115 | Detection coverage |
| `get_matcher_priority()` | `detection.rs` | 1123-1135 | Matcher preference |
| `create_detection_from_group()` | `detection.rs` | 830-882 | Detection creation |
