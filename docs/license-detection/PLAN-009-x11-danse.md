# PLAN-009: x11_danse.txt

## Status: ROOT CAUSE IDENTIFIED - Fix Ready

## Test File
`testdata/license-golden/datadriven/lic4/x11_danse.txt`

## Issue
Extra `unknown-license-reference` and wrong ordering.

**Expected:** `["x11 AND other-permissive"]`
**Actual:** `["unknown-license-reference AND other-permissive AND x11"]`

## Root Cause Analysis

### The Bug

In `detect()` at `src/license_detection/mod.rs:336-343`, the code calls BOTH:
1. `create_detection_from_group(group)` - which CORRECTLY filters `unknown-license-reference`
2. `populate_detection_from_group_with_spdx()` - which OVERWRITES the correct result

```rust
let detections: Vec<LicenseDetection> = groups
    .iter()
    .map(|group| {
        let mut detection = create_detection_from_group(group);  // CORRECT expression
        populate_detection_from_group_with_spdx(&mut detection, group, &self.spdx_mapping);  // OVERWRITES!
        detection
    })
    .collect();
```

### Why It Happens

1. `create_detection_from_group()` at detection.rs:829-881:
   - Analyzes matches → returns `DETECTION_LOG_UNKNOWN_INTRO_FOLLOWED_BY_MATCH`
   - Filters out the `unknown-license-reference` match (line 845-846)
   - Computes expression from FILTERED matches: `"other-permissive AND x11"`
   - Stores correct expression in `detection.license_expression`

2. `populate_detection_from_group_with_spdx()` at detection.rs:803-815:
   - Calls `populate_detection_from_group()` which:
     - Computes expression from ALL matches (line 767): `"unknown-license-reference AND other-permissive AND x11"`
     - OVERWRITES the correct expression!

### The Filtered Match

The `unknown-license-reference` match (from `license-intro_94.RULE`) should be filtered because:
- `is_license_intro: true` on the rule
- `license_expression: "unknown-license-reference"` contains "unknown"
- Matches the condition for `is_unknown_intro()` at detection.rs:477-481
- Triggers `DETECTION_LOG_UNKNOWN_INTRO_FOLLOWED_BY_MATCH` which filters intros

## Fix

### Location
`src/license_detection/detection.rs`

### Option A: Fix `populate_detection_from_group` (Recommended)

Modify `populate_detection_from_group()` to use the same filtering logic as `create_detection_from_group()`:

```rust
pub fn populate_detection_from_group(detection: &mut LicenseDetection, group: &DetectionGroup) {
    if group.matches.is_empty() {
        return;
    }

    let log_category = analyze_detection(&group.matches, false);

    let matches_for_expression = if log_category == DETECTION_LOG_UNKNOWN_INTRO_FOLLOWED_BY_MATCH {
        filter_license_intros(&group.matches)
    } else if log_category == DETECTION_LOG_UNKNOWN_REFERENCE_TO_LOCAL_FILE {
        filter_license_intros_and_references(&group.matches)
    } else {
        group.matches.clone()
    };

    detection.matches = group.matches.clone();  // Store RAW matches
    detection.detection_log.push(log_category.to_string());

    // Use FILTERED matches for expression
    if let Ok(expr) = determine_license_expression(&matches_for_expression) {
        detection.license_expression = Some(expr.clone());
        if let Ok(spdx_expr) = determine_spdx_expression(&matches_for_expression) {
            detection.license_expression_spdx = Some(spdx_expr);
        }
    }

    detection.identifier = None;

    if group.start_line > 0 {
        detection.file_region = Some(FileRegion {
            path: String::new(),
            start_line: group.start_line,
            end_line: group.end_line,
        });
    }
}
```

### Option B: Simplify by Removing Redundant Call

In `detect()`, remove the redundant call to `populate_detection_from_group_with_spdx`:

```rust
let detections: Vec<LicenseDetection> = groups
    .iter()
    .map(|group| {
        let mut detection = create_detection_from_group(group);
        // Only do SPDX mapping, don't re-populate
        if let Some(ref scancode_expr) = detection.license_expression
            && let Ok(spdx_expr) = determine_spdx_expression_from_scancode(scancode_expr, &self.spdx_mapping)
        {
            detection.license_expression_spdx = Some(spdx_expr);
        }
        detection
    })
    .collect();
```

**Recommendation:** Option A is better because it ensures both `create_detection_from_group` and `populate_detection_from_group` behave consistently.

## Verification

1. Run the failing test:
   ```bash
   cargo test test_x11_danse_expected_expression --lib -- --nocapture
   ```

2. The expression should be `"x11 AND other-permissive"` (without `unknown-license-reference`)

3. The detection log should be `["unknown-intro-followed-by-match"]`

## Test Data Analysis

The test file contains:
1. Line 3: "COPYRIGHT AND PERMISSION NOTICE:" → matches `license-intro_94.RULE` (is_license_intro: yes, license_expression: unknown-license-reference)
2. Lines 5-31: MIT-style license text → matches `other-permissive_339.RULE` (is_license_text: yes)
3. Lines 33-38: DANSE notice → matches `x11_danse2.RULE` (is_license_notice: yes, license_expression: x11 AND other-permissive)

The `license-intro_94.RULE` match should be filtered because it's an "unknown intro followed by proper detection".

## Failing Test

`test_x11_danse_expected_expression` in `src/license_detection/x11_danse_test.rs:678-700`
