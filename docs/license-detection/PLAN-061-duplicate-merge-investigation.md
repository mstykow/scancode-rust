# PLAN-061: Duplicate Detections Merged Investigation

## Status: RESOLVED

## Problem Statement

Multiple license instances in a file are being incorrectly merged into one detection. Expected N expressions, got N-1 or fewer.

### Representative Test Case

**File**: `testdata/license-golden/datadriven/lic1/edl-1.0.txt`

| Expected | Actual (Before Fix) |
|----------|---------------------|
| `["bsd-new", "bsd-new"]` | `["bsd-new"]` |

---

## Root Cause

**Location**: `src/license_detection/detection.rs:1146-1184`

**Function**: `apply_detection_preferences()`

**Bug**: The function was using `license_expression` as a HashMap key to deduplicate detections. This incorrectly merged detections with the same expression but at **different locations**.

```rust
// BEFORE (buggy):
let expr = detection.license_expression.clone().unwrap_or_else(String::new);
// ...
processed.insert(expr, (score, best_matcher_priority, detection));  // WRONG: dedupes by expression
```

**Why it's wrong**: Detections with the same license expression at different file locations should remain separate. For example, `edl-1.0.txt` has two BSD-New license references:
- Line 1: Short header "Eclipse Distribution License - v 1.0"
- Lines 7-13: Full license text

Both resolve to `bsd-new` expression but are at completely different locations.

**Python behavior**: Python's `get_unique_detections()` groups by `detection.identifier` (expression + content hash), NOT by expression alone. Different locations get different identifiers because the matched text differs.

---

## Investigation Trace

### Pipeline Analysis

| Stage | Rust Input | Rust Output | Status |
|-------|------------|-------------|--------|
| Raw aho matches | - | 3 matches | ✓ Correct |
| After merge_overlapping_matches | 3 | 3 | ✓ Correct |
| After filter_contained_matches | 3 | 2 | ✓ Correct |
| After filter_overlapping_matches | 2 | 2 | ✓ Correct |
| After refine_matches | 2 | 2 | ✓ Correct |
| After group_matches_by_region | 2 matches | 2 groups | ✓ Correct |
| After remove_duplicate_detections | 2 | 2 | ✓ Correct |
| After apply_detection_preferences | 2 | **1** | ✗ BUG HERE |

### Computed Identifiers

Both detections had different identifiers:
- Detection 0: `bsd_new-fe7f8a7d-b17f-b6f2-0394-5be383bf04b3`
- Detection 1: `bsd_new-ebc66859-1421-b110-ab83-75f066df9fb9`

But `apply_detection_preferences` ignored the identifier and deduplicated by expression alone.

---

## Fix

Changed `apply_detection_preferences()` to NOT deduplicate by expression:

```rust
// AFTER (fixed):
pub fn apply_detection_preferences(detections: Vec<LicenseDetection>) -> Vec<LicenseDetection> {
    detections  // No deduplication - different locations stay separate
}
```

The function's previous deduplication logic was incorrect and not aligned with Python behavior. Python has no equivalent function that deduplicates by expression.

---

## Test Verification

| Test | Before Fix | After Fix |
|------|------------|-----------|
| `test_edl_1_0_duplicate_detection` | 1 detection | 2 detections ✓ |
| `test_apache_2_0_and_apache_2_0` | 1 detection | 2 detections ✓ |
| `test_aladdin_md5_and_not_rsa_md5` | - | 2 detections ✓ |
| `test_remove_duplicate_detections_*` | Pass | Pass ✓ |

---

## Key Files

| Rust File | Python File | Purpose |
|-----------|-------------|---------|
| `src/license_detection/detection.rs:1146` | N/A | `apply_detection_preferences` was incorrect |
| `src/license_detection/detection.rs:907` | `detection.py:1017` | `remove_duplicate_detections` groups by identifier (correct) |

---

## Lessons Learned

1. **Deduplication must use location-aware identifiers**, not just expressions
2. The `identifier` field (expression + content hash) correctly distinguishes different instances
3. Python groups by `identifier`, ensuring different locations stay separate
4. Always verify against Python reference behavior, not assumptions

---

## Resolution

- **Fixed**: `src/license_detection/detection.rs:1146-1153`
- **Tests Added**: `src/license_detection/duplicate_merge_investigation_test.rs`
- **Status**: Complete
