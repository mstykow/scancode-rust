# PLAN-045: Expression Selection Parity for Overlapping Matches

**Status: ⚠️ PLAN NEEDS REVISION - DIAGNOSIS IS INCORRECT**

## Executive Summary

**⚠️ CRITICAL: The plan's diagnosis does NOT match actual behavior.**

**Plan's Claim:**
- CDDL 1.0 and CDDL 1.1 rules both match overlapping text
- Both matches survive match refinement
- Detection-level deduplication keeps both detections

**Actual Behavior (verified by running tests):**
- Test file: `cddl-1.0_or_gpl-2.0-glassfish.txt`
- Expected: `cddl-1.0 OR gpl-2.0` (from YAML)
- Actual: `cddl-1.1 OR gpl-2.0 WITH classpath-exception-2.0`
- **Only ONE match is detected, NOT two overlapping matches**

**Actual Issue:** The WRONG rule is matching. The CDDL 1.1 rule matches when the CDDL 1.0 rule should match. This is NOT a deduplication issue.

**Root Cause:** Unknown - needs further investigation. Possibilities:
1. CDDL 1.0 rule not matching at all (tokenization/matching issue)
2. CDDL 1.1 rule matching with higher score and filtering out CDDL 1.0
3. Rule selection logic preferring CDDL 1.1 over CDDL 1.0

---

## Investigation Results (VERIFIED 2026-02-25)

### Current Behavior Analysis

**Test file:** `testdata/license-golden/datadriven/lic1/cddl-1.0_or_gpl-2.0-glassfish.txt`

**Content differences from CDDL 1.1 version:**
- Has URL: `https://glassfish.dev.java.net/public/CDDL+GPL.html` (CDDL 1.0)
- Does NOT have URL: `https://glassfish.dev.java.net/public/CDDL+GPL_1_1.html` (CDDL 1.1)
- Does NOT have "GPL Classpath Exception" text section
- Uses "Sun Microsystems" copyright, not "Oracle"

**Expected behavior:**
- Match `cddl-1.0_or_gpl-2.0-glassfish.RULE` → expression: `cddl-1.0 OR gpl-2.0`

**Actual behavior:**
- Matches `cddl-1.1_or_gpl-2.0-classpath_and_apache-2.0-glassfish_1.RULE`
- Expression: `cddl-1.1 OR gpl-2.0 WITH classpath-exception-2.0`
- Only ONE match detected

### Verification Commands

```bash
# Run glassfish debug test
cargo test debug_glassfish_detection -- --nocapture

# Output shows:
# Expected: ["cddl-1.0 OR gpl-2.0"]
# Actual:   ["cddl-1.1 OR gpl-2.0 WITH classpath-exception-2.0"]
#
# Detection 1:
#   expression: Some("cddl-1.1 OR gpl-2.0 WITH classpath-exception-2.0")
#     match: cddl-1.1 OR gpl-2.0 WITH classpath-exception-2.0 score=86.0 matcher=3-seq lines=2-31
```

### Plan Diagnosis vs Actual Behavior

| Plan Claim | Actual Behavior | Correct? |
|------------|-----------------|----------|
| CDDL 1.0 and CDDL 1.1 both match | Only CDDL 1.1 matches | ❌ NO |
| Both survive match refinement | Only one match exists | ❌ NO |
| Detection deduplication keeps both | Only one detection exists | ❌ NO |

**Conclusion:** The plan's diagnosis is incorrect. The issue is NOT about deduplicating overlapping detections.

---

## Required Next Steps

### Step 1: Investigate Why CDDL 1.0 Rule Doesn't Match

**Hypothesis:** The CDDL 1.0 rule (`cddl-1.0_or_gpl-2.0-glassfish.RULE`) is not matching, or is being filtered out before detection.

**Investigation needed:**
1. Check if CDDL 1.0 rule matches at all during matching phase
2. Check if CDDL 1.0 match is filtered during match refinement
3. Compare token sets between the two rules and the query file

**Debug approach:**
```bash
# Check if CDDL 1.0 rule exists and is loaded
grep -r "cddl-1.0_or_gpl-2.0-glassfish" reference/scancode-toolkit/src/licensedcode/data/rules/

# Add debug logging to see all matches before refinement
# Check match_refine.rs for filtering logic
```

### Step 2: Understand Rule Selection

The CDDL 1.1 rule should NOT match this file because:
1. File has CDDL 1.0 URL, not CDDL 1.1 URL
2. File does NOT have Classpath Exception text
3. File uses Sun Microsystems copyright, not Oracle

This suggests the matching algorithm is not correctly distinguishing between similar rules.

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

## Status: NOT READY FOR IMPLEMENTATION

**The original implementation plan below is INCORRECT because:**
1. The diagnosis does not match actual behavior
2. The fix targets the wrong layer (detection vs matching)
3. Unit tests in the plan assume the wrong problem

**Before implementing any fix, we need to:**
1. Determine why CDDL 1.0 rule doesn't match (or is filtered)
2. Understand why CDDL 1.1 rule matches when it shouldn't
3. Verify the root cause is in matching, refinement, or detection

---

## Original Implementation Plan (PROBABLY INCORRECT)

*The following was the original plan. It is preserved for reference but should NOT be implemented without further investigation.*

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
| **Is the plan ready for implementation?** | ❌ NO |
| **Why?** | Diagnosis does not match actual behavior |
| **Actual issue?** | WRONG rule is matching (CDDL 1.1 instead of CDDL 1.0) |
| **NOT an issue?** | Duplicate overlapping detections |
| **Next step required?** | Investigate why CDDL 1.0 rule doesn't match |

---

## Verification Checklist (UPDATED)

- [ ] **BLOCKED:** Determine why CDDL 1.0 rule doesn't match
- [ ] **BLOCKED:** Understand why CDDL 1.1 rule matches when it shouldn't
- [ ] **BLOCKED:** Identify correct root cause before implementing fix
- [ ] ~~`detections_overlap_significantly()` helper added~~ (WRONG FIX)
- [ ] ~~`compare_detection_quality()` helper added~~ (WRONG FIX)  
- [ ] ~~`apply_detection_preferences()` updated~~ (WRONG FIX)
- [ ] CDDL glassfish golden tests pass
- [ ] No regressions in other golden tests
- [ ] Code passes `cargo clippy`
- [ ] Code formatted with `cargo fmt`

| Function | File | Lines | Purpose |
|----------|------|-------|---------|
| `filter_contained_matches()` | `match.py` | 1075-1184 | Remove contained matches by qspan equality |
| `filter_overlapping_matches()` | `match.py` | 1187-1523 | Remove overlapping matches |
| `get_detections_by_id()` | `detection.py` | ~950 | Group detections by identifier |

## Appendix: Rust Code References

| Function | File | Lines | Purpose |
|----------|------|-------|---------|
| `apply_detection_preferences()` | `detection.rs` | 1146-1184 | **MODIFY** - Add region deduplication |
| `compute_detection_score()` | `detection.rs` | 634-658 | Detection scoring |
| `compute_detection_coverage()` | `detection.rs` | 1091-1115 | Detection coverage |
| `get_matcher_priority()` | `detection.rs` | 1123-1135 | Matcher preference |
| `create_detection_from_group()` | `detection.rs` | 830-882 | Detection creation |
