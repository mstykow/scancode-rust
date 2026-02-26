# PLAN-082: gpl-2.0-plus_and_gpl-2.0-plus.txt Investigation

## Status: IMPLEMENTATION PLAN READY

## Problem Statement

**File**: `testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_gpl-2.0-plus.txt`

| Expected (Python) | Actual (Rust) |
|-------------------|---------------|
| `["gpl-2.0-plus", "gpl-2.0-plus"]` | `["gpl-2.0-plus"]` |

**Issue**: Missing one `gpl-2.0-plus` detection. File has two separate GPL-2.0+ sections (lines 10-27 and lines 34-51) but Rust only detects one.

## Root Cause Analysis

### Python Behavior (Correct)

1. **File-level detections** (`.files[0].license_detections`): Returns 2 detections
   - Detection 1: lines 10-27, identifier `gpl_2_0_plus-8ea3236b-6b72-a885-5aac-3270330faa3a`
   - Detection 2: lines 34-51, identifier `gpl_2_0_plus-8ea3236b-6b72-a885-5aac-3270330faa3a`
   - **Both have the SAME identifier** but are kept as separate detections

2. **Top-level detections** (`.license_detections`): Returns 1 unique detection
   - Uses `get_unique_detections()` to group by identifier
   - Sets `detection_count: 2` to indicate multiple occurrences
   - Keeps `file_regions` list to track each occurrence's location

### Rust Behavior (Buggy)

1. **Detection creation**: Correctly creates 2 detection groups
   - Group 1: lines 10-27
   - Group 2: lines 34-51

2. **Post-processing**: `remove_duplicate_detections()` incorrectly merges them
   - Located at `src/license_detection/detection.rs:906-925`
   - Uses a HashMap keyed by identifier
   - When identifier matches, keeps only ONE detection
   - Result: Only 1 detection survives

### The Bug

**`remove_duplicate_detections()` should NOT deduplicate file-level detections.**

The Python code shows that at the FILE level:
- Detections with the same identifier are kept separate
- The identifier is computed from matched text content (rule_id, score, tokens)
- Two identical license texts at different locations will have the SAME identifier
- But they represent DIFFERENT occurrences and should both be reported

Deduplication only happens at the **top-level aggregation** (`get_unique_detections()`), not at the file level.

## Code Locations

### Bug Location
- `src/license_detection/detection.rs:906-925` - `remove_duplicate_detections()`
- `src/license_detection/detection.rs:1152` - called in `post_process_detections()`

### Python Reference
- `reference/scancode-toolkit/src/licensedcode/detection.py:1017-1027` - `get_detections_by_id()`
- `reference/scancode-toolkit/src/licensedcode/detection.py:918-961` - `get_unique_detections()` (only for top-level)
- Key insight: `get_detections_by_id()` groups by identifier but is used ONLY by `get_unique_detections()` for top-level aggregation, not per-file

## Implementation Plan

### Step 1: Remove `remove_duplicate_detections()` from `post_process_detections()`

**File**: `src/license_detection/detection.rs:1147-1156`

**Before**:
```rust
pub fn post_process_detections(
    detections: Vec<LicenseDetection>,
    min_score: f32,
) -> Vec<LicenseDetection> {
    let filtered = filter_detections_by_score(detections, min_score);
    let deduplicated = remove_duplicate_detections(filtered);
    let preferred = apply_detection_preferences(deduplicated);
    let ranked = rank_detections(preferred);
    sort_detections_by_line(ranked)
}
```

**After**:
```rust
pub fn post_process_detections(
    detections: Vec<LicenseDetection>,
    min_score: f32,
) -> Vec<LicenseDetection> {
    let filtered = filter_detections_by_score(detections, min_score);
    let preferred = apply_detection_preferences(filtered);
    let ranked = rank_detections(preferred);
    sort_detections_by_line(ranked)
}
```

**Rationale**: File-level detections should never be deduplicated by identifier. Each occurrence of a license at a different location is a separate detection.

### Step 2: Delete or Deprecate `remove_duplicate_detections()` Function

**File**: `src/license_detection/detection.rs:898-925`

**Option A (Recommended)**: Delete the function entirely since it's no longer used.

**Option B**: Keep it but mark as deprecated with documentation explaining why it shouldn't be used for file-level deduplication.

If keeping, add documentation:
```rust
/// Remove duplicate detections (same identifier).
///
/// **WARNING**: This function should NOT be used for file-level deduplication.
/// In Python's ScanCode, detections with the same identifier but different
/// file locations are kept as separate detections at the file level.
/// Deduplication by identifier only happens at the top-level aggregation
/// via `get_unique_detections()`.
///
/// This function is kept for potential future use in top-level aggregation.
#[deprecated(
    note = "Do not use for file-level deduplication. See PLAN-082 for details."
)]
pub fn remove_duplicate_detections(detections: Vec<LicenseDetection>) -> Vec<LicenseDetection> {
    // ... existing implementation
}
```

### Step 3: Update Existing Tests

**File**: `src/license_detection/detection.rs` tests at lines 3275-3321

The test `test_remove_duplicate_detections_same_identifier_removed` has **wrong expectations**. It expects deduplication but this behavior is incorrect for file-level detections.

**Before**:
```rust
#[test]
fn test_remove_duplicate_detections_same_identifier_removed() {
    let identifier = "mit-abc123".to_string();
    let detections = vec![
        LicenseDetection {
            // ... detection 1 with same identifier
        },
        LicenseDetection {
            // ... detection 2 with same identifier
        },
    ];

    let result = remove_duplicate_detections(detections);
    assert_eq!(result.len(), 1, "Same identifier should dedupe");
    assert_eq!(result[0].identifier, Some(identifier));
}
```

**After** (if keeping the function):
```rust
#[test]
fn test_remove_duplicate_detections_same_identifier_kept_separate() {
    // Two detections with same identifier but different locations should be kept separate
    let identifier = "mit-abc123".to_string();
    let detections = vec![
        LicenseDetection {
            license_expression: Some("mit".to_string()),
            matches: vec![create_test_match_with_params(
                "mit", "rule-1", 1, 10,  // lines 1-10
                95.0, 100, 100, 100.0, 100, "mit.LICENSE",
            )],
            identifier: Some(identifier.clone()),
            file_region: Some(FileRegion { start_line: 1, end_line: 10 }),
            ..Default::default()
        },
        LicenseDetection {
            license_expression: Some("mit".to_string()),
            matches: vec![create_test_match_with_params(
                "mit", "rule-1", 50, 60,  // lines 50-60 (different location!)
                95.0, 100, 100, 100.0, 100, "mit.LICENSE",
            )],
            identifier: Some(identifier.clone()),
            file_region: Some(FileRegion { start_line: 50, end_line: 60 }),
            ..Default::default()
        },
    ];

    let result = remove_duplicate_detections(detections);
    // NOTE: If the function is deprecated, this test documents the WRONG behavior
    // that we are moving away from. File-level detections should NOT be deduplicated.
    assert_eq!(result.len(), 1, "DEPRECATED: This behavior is incorrect for file-level");
}
```

**If deleting the function**: Remove all tests that test `remove_duplicate_detections`:
- `test_remove_duplicate_detections_different_expressions` (line 3228)
- `test_remove_duplicate_detections_same_identifier_removed` (line 3275)
- `test_remove_duplicate_detections_same_expression_different_identifier` (line 3324)
- `test_remove_duplicate_detections_empty` (line 3375)

### Step 4: Add New Test for File-Level Deduplication Behavior

**File**: `src/license_detection/detection.rs` (in tests module)

```rust
#[test]
fn test_post_process_detections_keeps_same_license_different_locations() {
    // This tests the fix for PLAN-082:
    // Two detections of the same license at different file locations
    // should NOT be deduplicated at the file level.
    
    let identifier = "mit-abc123".to_string();
    
    // Create two detections with same identifier but different locations
    let detections = vec![
        LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: Some("MIT".to_string()),
            matches: vec![create_test_match_with_params(
                "mit", "mit.LICENSE", 1, 10,
                95.0, 100, 100, 100.0, 100, "mit.LICENSE",
            )],
            identifier: Some(identifier.clone()),
            file_region: Some(FileRegion { start_line: 1, end_line: 10 }),
            detection_log: Vec::new(),
        },
        LicenseDetection {
            license_expression: Some("mit".to_string()),
            license_expression_spdx: Some("MIT".to_string()),
            matches: vec![create_test_match_with_params(
                "mit", "mit.LICENSE", 50, 60,  // Different location!
                95.0, 100, 100, 100.0, 100, "mit.LICENSE",
            )],
            identifier: Some(identifier.clone()),  // Same identifier!
            file_region: Some(FileRegion { start_line: 50, end_line: 60 }),
            detection_log: Vec::new(),
        },
    ];

    let result = post_process_detections(detections, 0.0);
    
    // Both detections should be preserved
    assert_eq!(result.len(), 2, "Same license at different locations should be kept separate");
    
    // Verify they have the same identifier
    assert_eq!(result[0].identifier, Some(identifier.clone()));
    assert_eq!(result[1].identifier, Some(identifier.clone()));
    
    // Verify they have different locations
    let region1 = result[0].file_region.as_ref().unwrap();
    let region2 = result[1].file_region.as_ref().unwrap();
    assert_ne!(region1.start_line, region2.start_line);
}
```

### Step 5: Run Golden Tests to Verify Fix

```bash
# Run the specific golden test for PLAN-082
cargo test test_license_golden_datadriven_lic4 -- --nocapture 2>&1 | grep -A5 "gpl-2.0-plus_and_gpl-2.0-plus.txt"

# Expected: Test should now pass with 2 detections instead of 1

# Run all license golden tests
cargo test --lib license_detection::golden_test

# Run the specific investigation test if it exists
cargo test --lib test_plan_082
```

### Step 6: Verify PLAN-084 Also Fixed

PLAN-084 (`gpl-2.0-plus_and_gpl-2.0-plus_and_public-domain.txt`) has the same root cause. After fixing PLAN-082:

```bash
# Run golden test for PLAN-084
cargo test test_license_golden_datadriven_lic4 -- --nocapture 2>&1 | grep -A5 "gpl-2.0-plus_and_gpl-2.0-plus_and_public-domain"

# Expected: Should now have 7 detections instead of 5
```

## Verification Commands

### Before Fix
```bash
cd reference/scancode-playground && venv/bin/scancode --license --json-pp - \
  ../../testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_gpl-2.0-plus.txt 2>/dev/null | \
  jq '.files[0].license_detections | length'
# Output: 2 (correct Python behavior)

# Rust current (buggy) behavior:
cargo run -- testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_gpl-2.0-plus.txt --license --json - 2>/dev/null | \
  jq '.files[0].license_detections | length'
# Output: 1 (incorrect - bug)
```

### After Fix
```bash
cargo run -- testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_gpl-2.0-plus.txt --license --json - 2>/dev/null | \
  jq '.files[0].license_detections | length'
# Expected Output: 2 (matches Python)
```

## Regression Risk Assessment

### Low Risk Areas
- The fix is a **deletion** (removing incorrect deduplication), not adding complexity
- Golden tests will catch any regressions
- The behavior now matches Python exactly

### Potential Regression Scenarios

1. **Duplicate detection entries in output**:
   - Concern: Some callers might expect deduplication
   - Mitigation: Check all callers of `post_process_detections()`
   - Search: `rg "post_process_detections" src/`
   - Result: Called from `mod.rs:164` and `mod.rs:314` - both are file-level processing

2. **Top-level aggregation**:
   - Concern: If `remove_duplicate_detections` was used for top-level aggregation
   - Mitigation: Check if top-level aggregation exists and uses this function
   - Current state: No top-level aggregation implemented yet in Rust
   - Note: When implementing top-level aggregation, we'll need to implement `get_unique_detections()` equivalent

### Files to Check for Regressions

1. `src/license_detection/mod.rs:164` - main detection pipeline
2. `src/license_detection/mod.rs:314` - alternative code path
3. All golden tests in `src/license_detection/golden_test.rs`

## Summary of Changes

| File | Change | Lines |
|------|--------|-------|
| `src/license_detection/detection.rs` | Remove `remove_duplicate_detections()` call from `post_process_detections()` | ~1152 |
| `src/license_detection/detection.rs` | Delete or deprecate `remove_duplicate_detections()` function | 898-925 |
| `src/license_detection/detection.rs` | Update/remove tests for `remove_duplicate_detections()` | 3228-3379 |
| `src/license_detection/detection.rs` | Add test for correct behavior | new test |

## Related Issues

- **PLAN-084**: `gpl-2.0-plus_and_gpl-2.0-plus_and_public-domain.txt` - Same root cause, will be fixed by this change

## Test Verification

Run Python reference:
```bash
cd reference/scancode-playground && venv/bin/scancode --license --json-pp - \
  ../../testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_gpl-2.0-plus.txt 2>/dev/null | \
  jq '.files[0].license_detections | length'
# Output: 2

cd reference/scancode-playground && venv/bin/scancode --license --json-pp - \
  ../../testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_gpl-2.0-plus.txt 2>/dev/null | \
  jq '.files[0].license_detections[0].identifier, .files[0].license_detections[1].identifier'
# Output: "gpl_2_0_plus-8ea3236b-6b72-a885-5aac-3270330faa3a" (both same!)
```

## Additional Notes

The identifier is computed from:
1. Rule identifier
2. Match score
3. Tokenized matched text

Since both GPL-2.0+ texts are identical, they produce the same identifier. This is expected and correct behavior - the identifier represents the "content signature", not the "location signature".

The bug is conflating "same content" with "same occurrence".
