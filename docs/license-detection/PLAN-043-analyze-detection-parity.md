# PLAN-043: analyze_detection Parity Analysis

**Status**: ✅ IMPLEMENTED (2026-02-24)
**Impact**: No regression (3780 passed → 3780 passed)

## Summary

This document analyzes the differences between Python's `analyze_detection()` function and Rust's equivalent implementation, identifying all behavioral differences that affect license detection categorization.

## Python Implementation Overview

**File**: `reference/scancode-toolkit/src/licensedcode/detection.py:1760-1818`

```python
def analyze_detection(license_matches, package_license=False):
    if is_undetected_license_matches(license_matches=license_matches):
        return DetectionCategory.UNDETECTED_LICENSE.value

    elif has_unknown_intro_before_detection(license_matches=license_matches):
        return DetectionCategory.UNKNOWN_INTRO_BEFORE_DETECTION.value

    elif has_references_to_local_files(license_matches=license_matches):
        return DetectionCategory.UNKNOWN_FILE_REFERENCE_LOCAL.value

    elif not package_license and is_false_positive(
        license_matches=license_matches,
        package_license=package_license,
    ):
        return DetectionCategory.FALSE_POSITVE.value

    elif not package_license and has_correct_license_clue_matches(
        license_matches=license_matches
    ):
        return DetectionCategory.LICENSE_CLUES.value

    elif is_correct_detection_non_unknown(license_matches=license_matches):
        return DetectionCategory.PERFECT_DETECTION.value

    elif has_unknown_matches(license_matches=license_matches):
        return DetectionCategory.UNKNOWN_MATCH.value

    elif not package_license and is_low_quality_matches(license_matches=license_matches):
        return DetectionCategory.LOW_QUALITY_MATCH_FRAGMENTS.value

    elif is_match_coverage_less_than_threshold(
        license_matches=license_matches,
        threshold=IMPERFECT_MATCH_COVERAGE_THR,
    ):
        return DetectionCategory.IMPERFECT_COVERAGE.value

    elif has_extra_words(license_matches=license_matches):
        return DetectionCategory.EXTRA_WORDS.value

    else:
        return DetectionCategory.PERFECT_DETECTION.value
```

## Rust Implementation Overview

**File**: `src/license_detection/detection.rs:568-610`

```rust
fn analyze_detection(matches: &[LicenseMatch], package_license: bool) -> &'static str {
    if is_undetected_license_matches(matches) {
        return DETECTION_LOG_UNDETECTED_LICENSE;
    }

    if has_unknown_intro_before_detection(matches) {
        return DETECTION_LOG_UNKNOWN_INTRO_FOLLOWED_BY_MATCH;
    }

    if has_references_to_local_files(matches) {
        return DETECTION_LOG_UNKNOWN_REFERENCE_TO_LOCAL_FILE;
    }

    if !package_license && is_false_positive(matches) {
        return DETECTION_LOG_FALSE_POSITIVE;
    }

    if !package_license && is_low_quality_matches(matches) {
        return DETECTION_LOG_LICENSE_CLUES;
    }

    if is_correctDetection(matches) && !has_unknown_matches(matches) && !has_extra_words(matches) {
        return DETECTION_LOG_PERFECT_DETECTION;
    }

    if has_unknown_matches(matches) {
        return DETECTION_LOG_UNKNOWN_MATCH;
    }

    if !package_license && is_low_quality_matches(matches) {
        return DETECTION_LOG_LOW_QUALITY_MATCHES;
    }

    if is_match_coverage_below_threshold(matches, IMPERFECT_MATCH_COVERAGE_THR, true) {
        return DETECTION_LOG_IMPERFECT_COVERAGE;
    }

    if has_extra_words(matches) {
        return DETECTION_LOG_EXTRA_WORDS;
    }

    DETECTION_LOG_PERFECT_DETECTION
}
```

---

## Identified Differences

### Difference 1: Missing `has_correct_license_clue_matches` Check (CRITICAL)

**Location**: Between false positive check and perfect detection check

**Python** (lines 1786-1789):

```python
elif not package_license and has_correct_license_clue_matches(
    license_matches=license_matches
):
    return DetectionCategory.LICENSE_CLUES.value
```

**Rust**: MISSING entirely

**Python's `has_correct_license_clue_matches`** (detection.py:1265-1272):

```python
def has_correct_license_clue_matches(license_matches):
    """Return True if all the matches in ``license_matches`` List of LicenseMatch
    has True for the `is_license_clue` rule attribute.
    """
    return is_correct_detection(license_matches) and all(
        match.rule.is_license_clue for match in license_matches
    )
```

**Impact**:

- Matches that are perfect detections AND have all `is_license_clue=true` should be categorized as `license-clues` not `perfect-detection`
- This affects files where license clue rules match perfectly

**Fix Required**: Implement `has_correct_license_clue_matches()` and add the check after false positive check

---

### Difference 2: Duplicate `is_low_quality_matches` Check (BUG)

**Location**: Rust lines 585-587 and 597-599

**Rust**:

```rust
// First check (line 585-587):
if !package_license && is_low_quality_matches(matches) {
    return DETECTION_LOG_LICENSE_CLUES;
}

// ... then later (line 597-599):
if !package_license && is_low_quality_matches(matches) {
    return DETECTION_LOG_LOW_QUALITY_MATCHES;
}
```

**Python**: Only checks `is_low_quality_matches` once at the correct position (lines 1800-1801)

**Impact**:

- The second check (line 597-599) is unreachable because the first check returns early
- This means `LOW_QUALITY_MATCH_FRAGMENTS` category is NEVER returned in Rust
- The first check incorrectly returns `LICENSE_CLUES` instead of `LOW_QUALITY_MATCHES`

**Fix Required**: Remove the first `is_low_quality_matches` check and keep only the second one with correct category

---

### Difference 3: Wrong Detection Category Returned for Low Quality Matches (BUG)

**Location**: Rust line 585-587

**Python** (lines 1800-1801):

```python
elif not package_license and is_low_quality_matches(license_matches=license_matches):
    return DetectionCategory.LOW_QUALITY_MATCH_FRAGMENTS.value
```

**Rust** (lines 585-587):

```rust
if !package_license && is_low_quality_matches(matches) {
    return DETECTION_LOG_LICENSE_CLUES;  // WRONG CATEGORY!
}
```

**Impact**:

- Low quality matches are being labeled as `license-clues` instead of `low-quality-matches`
- These are semantically different categories in Python

**Fix Required**: Return `DETECTION_LOG_LOW_QUALITY_MATCHES` instead of `DETECTION_LOG_LICENSE_CLUES`

---

### Difference 4: `is_false_positive` Missing `package_license` Parameter

**Location**: Function signature

**Python** (detection.py:1162):

```python
def is_false_positive(license_matches, package_license=False):
    """..."""
    if package_license:
        return False  # Early exit for package licenses
    # ... rest of logic
```

**Rust** (detection.rs:311):

```rust
fn is_false_positive(matches: &[LicenseMatch]) -> bool {
    // No package_license parameter
```

**Impact**:

- For package license detection, false positives should NOT be filtered
- Rust always applies false positive filtering regardless of context
- This could incorrectly filter valid package license matches

**Fix Required**: Add `package_license: bool` parameter to `is_false_positive()` and check it at the start

---

### Difference 5: Missing `LOW_RELEVANCE` Detection Category

**Location**: DetectionCategory enum / constants

**Python** (detection.py:119):

```python
class DetectionCategory(Enum):
    # ... other values ...
    LOW_RELEVANCE = 'low-relevance'
```

**Rust**: No equivalent constant defined

**Python Usage** (detection.py:1754-1755):

```python
elif has_low_rule_relevance(license_matches=license_matches):
    ambi_license_detections[DetectionCategory.LOW_RELEVANCE.value] = detection
```

**Note**: `LOW_RELEVANCE` is used in `get_ambiguous_license_detections_by_type()` not `analyze_detection()`, so this is lower priority but should still be tracked.

**Impact**: Lower priority - used in ambiguity detection, not main detection flow

**Fix Required**: Add `DETECTION_LOG_LOW_RELEVANCE` constant and `has_low_rule_relevance()` function

---

### Difference 6: Missing `process_detections` Post-Processing Function

**Location**: After detection creation

**Python** (detection.py:2133-2177):

```python
def process_detections(detections, licensing=Licensing()):
    """
    Yield LicenseDetection objects given a list of LicenseDetection objects
    after postprocessing for the following:

    1. Include license clues as detections if there are other proper detections
       with the same license keys.
    """
    # ... implementation ...
```

**Rust**: Not implemented

**Impact**:

- License clues with same keys as proper detections should be "promoted" to proper detections
- Detection log entry `not-license-clues-as-more-detections-present` is never added

**Fix Required**: Implement `process_detections()` equivalent

---

### Difference 7: `is_match_coverage_less_than_threshold` vs `is_match_coverage_below_threshold`

**Location**: Function name consistency

**Python** (detection.py:1095):

```python
def is_match_coverage_less_than_threshold(license_matches, threshold, any_matches=True):
```

**Rust** (detection.rs:271):

```rust
fn is_match_coverage_below_threshold(
    matches: &[LicenseMatch],
    threshold: f32,
    any_matches: bool,
) -> bool {
```

**Impact**: None - just naming difference, logic is identical

---

## Step-by-Step Comparison Table

| Step | Python Check | Python Returns | Rust Check | Rust Returns | Parity |
|------|-------------|----------------|------------|--------------|--------|
| 1 | `is_undetected_license_matches` | `undetected-license` | Same | Same | ✅ |
| 2 | `has_unknown_intro_before_detection` | `unknown-intro-before-detection` | Same | `unknown-intro-followed-by-match` | ⚠️ Naming |
| 3 | `has_references_to_local_files` | `unknown-file-reference-local` | Same | `unknown-reference-to-local-file` | ⚠️ Naming |
| 4 | `!package_license && is_false_positive` | `possible-false-positive` | Same (missing param) | Same | ⚠️ Missing param |
| 5 | `!package_license && has_correct_license_clue_matches` | `license-clues` | **MISSING** | - | ❌ Missing |
| 6 | `is_correct_detection_non_unknown` | `perfect-detection` | Inline equivalent | Same | ✅ |
| 7 | `has_unknown_matches` | `unknown-match` | Same | Same | ✅ |
| 8 | `!package_license && is_low_quality_matches` | `low-quality-matches` | **Wrong position** | `license-clues` | ❌ Bug |
| 9 | `is_match_coverage_less_than_threshold` | `imperfect-match-coverage` | Same | Same | ✅ |
| 10 | `has_extra_words` | `extra-words` | Same | Same | ✅ |
| 11 | else | `perfect-detection` | Same | Same | ✅ |

---

## Required Implementation Changes

### 1. Implement `has_correct_license_clue_matches()` (HIGH PRIORITY)

```rust
fn has_correct_license_clue_matches(matches: &[LicenseMatch]) -> bool {
    is_correct_detection(matches) && matches.iter().all(|m| m.is_license_clue)
}
```

### 2. Fix `analyze_detection()` Order (HIGH PRIORITY)

```rust
fn analyze_detection(matches: &[LicenseMatch], package_license: bool) -> &'static str {
    if is_undetected_license_matches(matches) {
        return DETECTION_LOG_UNDETECTED_LICENSE;
    }

    if has_unknown_intro_before_detection(matches) {
        return DETECTION_LOG_UNKNOWN_INTRO_FOLLOWED_BY_MATCH;
    }

    if has_references_to_local_files(matches) {
        return DETECTION_LOG_UNKNOWN_REFERENCE_TO_LOCAL_FILE;
    }

    if !package_license && is_false_positive_with_param(matches, package_license) {
        return DETECTION_LOG_FALSE_POSITIVE;
    }

    // NEW: Add has_correct_license_clue_matches check HERE
    if !package_license && has_correct_license_clue_matches(matches) {
        return DETECTION_LOG_LICENSE_CLUES;
    }

    if is_correctDetection(matches) && !has_unknown_matches(matches) && !has_extra_words(matches) {
        return DETECTION_LOG_PERFECT_DETECTION;
    }

    if has_unknown_matches(matches) {
        return DETECTION_LOG_UNKNOWN_MATCH;
    }

    // FIXED: This should return LOW_QUALITY_MATCHES, not LICENSE_CLUES
    // And should only appear once at this position
    if !package_license && is_low_quality_matches(matches) {
        return DETECTION_LOG_LOW_QUALITY_MATCHES;
    }

    if is_match_coverage_below_threshold(matches, IMPERFECT_MATCH_COVERAGE_THR, true) {
        return DETECTION_LOG_IMPERFECT_COVERAGE;
    }

    if has_extra_words(matches) {
        return DETECTION_LOG_EXTRA_WORDS;
    }

    DETECTION_LOG_PERFECT_DETECTION
}
```

### 3. Fix `is_false_positive()` to Accept `package_license` Parameter

```rust
fn is_false_positive(matches: &[LicenseMatch], package_license: bool) -> bool {
    if package_license {
        return false;  // Never filter package licenses
    }
    // ... existing logic
}
```

### 4. Add Missing Detection Log Constant

```rust
pub const DETECTION_LOG_LOW_RELEVANCE: &str = "low-relevance";
```

### 5. Implement `has_low_rule_relevance()`

```rust
fn has_low_rule_relevance(matches: &[LicenseMatch]) -> bool {
    matches.iter().all(|m| m.rule_relevance < LOW_RELEVANCE_THRESHOLD)
}
```

### 6. Implement `process_detections()` (MEDIUM PRIORITY)

This function handles license clue promotion when same license keys appear in proper detections.

---

## Expected Impact on Golden Tests

### Immediate Impact

1. **Files with license clue matches that are perfect detections**:
   - Currently: `perfect-detection`
   - After fix: `license-clues` (if all matches have `is_license_clue=true`)

2. **Files with low quality matches**:
   - Currently: `license-clues`
   - After fix: `low-quality-matches`

3. **Package license files with potential false positives**:
   - Currently: May be filtered incorrectly
   - After fix: Will not be filtered when `package_license=true`

### Test Files to Check

- Any test files containing matches with `is_license_clue=true` flag
- Files with low coverage matches (< 60%)
- Package manifest files being scanned for license detection

---

## Summary of Changes Required

| Priority | Change | File | Lines |
|----------|--------|------|-------|
| HIGH | Add `has_correct_license_clue_matches()` | detection.rs | ~568 |
| HIGH | Fix `analyze_detection()` order | detection.rs | 568-610 |
| HIGH | Remove duplicate `is_low_quality_matches` check | detection.rs | 585-587 |
| HIGH | Add `package_license` param to `is_false_positive` | detection.rs | 311 |
| MEDIUM | Add `DETECTION_LOG_LOW_RELEVANCE` constant | detection.rs | ~50 |
| MEDIUM | Implement `has_low_rule_relevance()` | detection.rs | new |
| MEDIUM | Implement `process_detections()` | detection.rs | new |

---

## Verification Plan

1. Run golden tests before changes to establish baseline
2. Implement changes one at a time with tests
3. Compare detection categories for each golden test file
4. Verify `license-clues` vs `low-quality-matches` categorization
5. Verify package license detection is not affected by false positive filtering

---

## Verification Against Python Reference

### Difference 1: `has_correct_license_clue_matches` - VERIFIED CRITICAL

**Python Source** (detection.py:1265-1272):

```python
def has_correct_license_clue_matches(license_matches):
    """Return True if all the matches in ``license_matches`` List of LicenseMatch
    has True for the `is_license_clue` rule attribute.
    """
    return is_correct_detection(license_matches) and all(
        match.rule.is_license_clue for match in license_matches
    )
```

**Analysis**: This is a DISTINCT check from `is_low_quality_matches`:

- `has_correct_license_clue_matches`: Perfect coverage (100%) + all `is_license_clue=true`
- `is_low_quality_matches`: NOT perfect coverage + coverage ≤ 60% for all matches

These produce different categories:

- `has_correct_license_clue_matches` → `license-clues`
- `is_low_quality_matches` → `low-quality-matches`

**Current Rust Bug**: The first `is_low_quality_matches` check returns `LICENSE_CLUES`, conflating two distinct categories.

---

### Difference 2 & 3: Duplicate `is_low_quality_matches` Check - VERIFIED BUG

**Python Order** (detection.py:1800-1801):

```python
# Step 8 in the flow - AFTER unknown_matches check
elif not package_license and is_low_quality_matches(license_matches=license_matches):
    return DetectionCategory.LOW_QUALITY_MATCH_FRAGMENTS.value
```

**Rust Order** (detection.rs:585-587, 597-599):

```rust
// Line 585-587 - WRONG POSITION, WRONG CATEGORY
if !package_license && is_low_quality_matches(matches) {
    return DETECTION_LOG_LICENSE_CLUES;  // Should be LOW_QUALITY_MATCHES
}

// ... later ...
// Line 597-599 - UNREACHABLE because above returns early
if !package_license && is_low_quality_matches(matches) {
    return DETECTION_LOG_LOW_QUALITY_MATCHES;
}
```

**Python's `is_low_quality_matches`** (detection.py:1275-1286):

```python
def is_low_quality_matches(license_matches):
    """Return True if the license_matches are not part of a correct
    license detection and are mere license clues.
    """
    return not is_correct_detection(license_matches) and (
        is_match_coverage_less_than_threshold(
            license_matches=license_matches,
            threshold=CLUES_MATCH_COVERAGE_THR,  # 60
            any_matches=False,  # Returns True if NONE have coverage > threshold
        )
    )
```

**Rust's `is_low_quality_matches`** (detection.rs:389-396):

```rust
fn is_low_quality_matches(matches: &[LicenseMatch]) -> bool {
    if matches.is_empty() {
        return true;
    }
    !is_correctDetection(matches)
        && is_match_coverage_below_threshold(matches, CLUES_MATCH_COVERAGE_THR, false)
}
```

**Verification**: Rust implementation matches Python logic. The bug is in the ORDER and CATEGORY in `analyze_detection`.

---

### Difference 4: `is_false_positive` Missing Parameter - VERIFIED BUT MITIGATED

**Python** (detection.py:1162-1170):

```python
def is_false_positive(license_matches, package_license=False):
    """..."""
    if package_license:
        return False  # Early exit for package licenses
    # ... rest of logic
```

**Rust** (detection.rs:311):

```rust
fn is_false_positive(matches: &[LicenseMatch]) -> bool {
    // No package_license parameter
```

**Mitigating Factor**: The `analyze_detection` function checks `!package_license && is_false_positive(matches)`, so the early-exit happens at the call site. However, this is inconsistent and could cause issues if `is_false_positive` is called from elsewhere.

**Recommendation**: Still add the parameter for consistency and to match Python's interface exactly.

---

### Difference 5: `LOW_RELEVANCE` Category - VERIFIED LOWER PRIORITY

**Python Usage** (detection.py:1754-1755):

```python
# In get_ambiguous_license_detections_by_type(), NOT analyze_detection()
elif has_low_rule_relevance(license_matches=license_matches):
    ambi_license_detections[DetectionCategory.LOW_RELEVANCE.value] = detection
```

**Python's `has_low_rule_relevance`** (detection.py:1151-1159):

```python
def has_low_rule_relevance(license_matches):
    """Return True if all on the matches in ``license_matches`` List of LicenseMatch
    objects has a match with low score because of low rule relevance.
    """
    return all(
        license_match.rule.relevance < LOW_RELEVANCE_THRESHOLD  # 70
        for license_match in license_matches
    )
```

**Impact**: This is used for ambiguity detection, not main detection flow. Lower priority.

---

### Difference 6: `process_detections` - VERIFIED MEDIUM PRIORITY

**Python** (detection.py:2133-2177):

```python
def process_detections(detections, licensing=Licensing()):
    """
    Yield LicenseDetection objects given a list of LicenseDetection objects
    after postprocessing for the following:

    1. Include license clues as detections if there are other proper detections
       with the same license keys.
    """
    if len(detections) == 1:
        yield detections[0]
    else:
        detected_license_keys = set()

        for detection in detections:
            if detection.license_expression != None:
                detected_license_keys.update(
                    licensing.license_keys(expression=detection.license_expression)
                )

        for detection in detections:
            if detection.license_expression == None:
                if has_correct_license_clue_matches(detection.matches):
                    yield detection
                    continue

                license_expression = str(combine_expressions(
                    expressions=[
                        match.rule.license_expression
                        for match in detection.matches
                    ],
                    unique=True,
                    licensing=licensing,
                ))
                license_keys = licensing.license_keys(expression=license_expression)

                if all(
                    key in detected_license_keys
                    for key in license_keys
                ):
                    detection.license_expression = license_expression
                    detection.license_expression_spdx = detection.spdx_license_expression()
                    detection.detection_log.append(DetectionRule.NOT_LICENSE_CLUES.value)
                    detection.identifier = detection.identifier_with_expression

            yield detection
```

**Impact**: This promotes license clues to proper detections when the same license keys appear elsewhere. The detection log entry `not-license-clues-as-more-detections-present` is never added in Rust.

---

## Edge Case Analysis

### Edge Case 1: Empty matches list

**Python**: Most functions handle empty lists gracefully

- `is_correct_detection([])` → returns `False` (all() on empty is True, but iteration fails)
- `is_low_quality_matches([])` → `not False and ...` → depends on threshold check

**Rust** (detection.rs:389-396):

```rust
fn is_low_quality_matches(matches: &[LicenseMatch]) -> bool {
    if matches.is_empty() {
        return true;  // Explicit handling
    }
    // ...
}
```

**Potential Issue**: Rust returns `true` for empty matches, but Python's behavior may differ. Need to verify Python's exact behavior.

### Edge Case 2: Mix of `is_license_clue=true` and `is_license_clue=false`

**Scenario**: Some matches have `is_license_clue=true`, others have `is_license_clue=false`

**Expected Behavior**: `has_correct_license_clue_matches` returns `False` (requires ALL matches to have `is_license_clue=true`)

**Current Rust**: Falls through to other checks (unknown-match, low-quality-matches, etc.)

### Edge Case 3: `is_license_clue=true` with perfect coverage but unknown matches

**Scenario**: Match has `is_license_clue=true`, coverage=100%, but `rule_identifier` contains "unknown"

**Python's `has_correct_license_clue_matches`**: Returns `True` (only checks `is_correct_detection` and `is_license_clue`, not unknown status)

**But**: `is_correct_detection` checks coverage=100% and valid matchers, NOT unknown status

**Flow**: Would return `license-clues` before checking `has_unknown_matches`

**Question**: Should `has_correct_license_clue_matches` also check for unknown matches? Looking at Python's `get_ambiguous_license_detections_by_type` (detection.py:1736-1740):

```python
elif (
    has_correct_license_clue_matches(license_matches=detection.matches) and
    has_unknown_matches(license_matches=detection.matches)
):
    ambi_license_detections[DetectionCategory.LICENSE_CLUES.value] = detection
```

This suggests `has_correct_license_clue_matches` + `has_unknown_matches` together trigger special handling in ambiguity detection.

### Edge Case 4: Package license with low quality matches

**Python**: `!package_license && is_low_quality_matches(...)` - package licenses skip this check

**Current Rust**: Same check, so package licenses correctly skip low-quality categorization

**But**: Need to verify the category falls through to `perfect-detection` or other appropriate category.

---

## Test Cases to Add

### Test 1: `has_correct_license_clue_matches` with all is_license_clue=true

```rust
#[test]
fn test_has_correct_license_clue_matches_all_true() {
    let matches = vec![
        LicenseMatch {
            matcher: "2-aho".to_string(),
            match_coverage: 100.0,
            is_license_clue: true,
            // ... other fields
        },
        LicenseMatch {
            matcher: "2-aho".to_string(),
            match_coverage: 100.0,
            is_license_clue: true,
            // ... other fields
        },
    ];
    
    assert!(has_correct_license_clue_matches(&matches));
    assert_eq!(analyze_detection(&matches, false), DETECTION_LOG_LICENSE_CLUES);
}
```

### Test 2: `has_correct_license_clue_matches` with mixed is_license_clue

```rust
#[test]
fn test_has_correct_license_clue_matches_mixed() {
    let matches = vec![
        LicenseMatch {
            matcher: "2-aho".to_string(),
            match_coverage: 100.0,
            is_license_clue: true,
            // ...
        },
        LicenseMatch {
            matcher: "2-aho".to_string(),
            match_coverage: 100.0,
            is_license_clue: false,  // Not a clue
            // ...
        },
    ];
    
    assert!(!has_correct_license_clue_matches(&matches));
    assert_eq!(analyze_detection(&matches, false), DETECTION_LOG_PERFECT_DETECTION);
}
```

### Test 3: `is_low_quality_matches` returns correct category

```rust
#[test]
fn test_is_low_quality_matches_category() {
    let matches = vec![LicenseMatch {
        matcher: "2-aho".to_string(),
        match_coverage: 40.0,  // Below 60%
        is_license_clue: false,
        // ...
    }];
    
    assert!(is_low_quality_matches(&matches));
    assert_eq!(analyze_detection(&matches, false), DETECTION_LOG_LOW_QUALITY_MATCHES);
}
```

### Test 4: Package license bypasses false positive check

```rust
#[test]
fn test_package_license_bypasses_false_positive() {
    // Create a match that would normally be a false positive
    let matches = vec![LicenseMatch {
        matcher: "2-aho".to_string(),
        match_coverage: 100.0,
        rule_relevance: 50,  // Low relevance
        rule_identifier: "gpl_bare.LICENSE".to_string(),
        // ...
    }];
    
    // Non-package: should be false positive
    assert_eq!(analyze_detection(&matches, false), DETECTION_LOG_FALSE_POSITIVE);
    
    // Package: should NOT be false positive
    assert_ne!(analyze_detection(&matches, true), DETECTION_LOG_FALSE_POSITIVE);
}
```

### Test 5: `has_low_rule_relevance` for ambiguity detection

```rust
#[test]
fn test_has_low_rule_relevance() {
    let matches = vec![LicenseMatch {
        rule_relevance: 50,  // Below 70 threshold
        // ...
    }];
    
    assert!(has_low_rule_relevance(&matches));
}
```

### Test 6: Distinction between license-clues and low-quality-matches

```rust
#[test]
fn test_license_clues_vs_low_quality_matches_distinction() {
    // Case 1: Perfect coverage, all is_license_clue=true -> license-clues
    let clue_matches = vec![LicenseMatch {
        matcher: "2-aho".to_string(),
        match_coverage: 100.0,
        is_license_clue: true,
        // ...
    }];
    assert_eq!(analyze_detection(&clue_matches, false), DETECTION_LOG_LICENSE_CLUES);
    
    // Case 2: Low coverage (<60%), not correct detection -> low-quality-matches
    let low_quality_matches = vec![LicenseMatch {
        matcher: "2-aho".to_string(),
        match_coverage: 50.0,
        is_license_clue: false,
        // ...
    }];
    assert_eq!(analyze_detection(&low_quality_matches, false), DETECTION_LOG_LOW_QUALITY_MATCHES);
}
```

---

## Implementation Checklist

- [ ] Add `LOW_RELEVANCE_THRESHOLD` constant (value: 70)
- [ ] Add `has_correct_license_clue_matches()` function
- [ ] Remove first `is_low_quality_matches` check from `analyze_detection()`
- [ ] Move `is_low_quality_matches` check to correct position (after `has_unknown_matches`)
- [ ] Change `is_low_quality_matches` return value to `DETECTION_LOG_LOW_QUALITY_MATCHES`
- [ ] Add `package_license` parameter to `is_false_positive()`
- [ ] Add early return in `is_false_positive()` when `package_license=true`
- [ ] Add `DETECTION_LOG_LOW_RELEVANCE` constant
- [ ] Add `DETECTION_LOG_NOT_LICENSE_CLUES` constant
- [ ] Add `has_low_rule_relevance()` function
- [ ] Implement `process_detections()` equivalent
- [ ] Add all test cases from above
- [ ] Run golden tests and compare output

---

## References

- Python: `reference/scancode-toolkit/src/licensedcode/detection.py:1760-1818`
- Python: `reference/scancode-toolkit/src/licensedcode/detection.py:1265-1272` (has_correct_license_clue_matches)
- Python: `reference/scancode-toolkit/src/licensedcode/detection.py:1275-1286` (is_low_quality_matches)
- Python: `reference/scancode-toolkit/src/licensedcode/detection.py:1151-1159` (has_low_rule_relevance)
- Python: `reference/scancode-toolkit/src/licensedcode/detection.py:2133-2177` (process_detections)
- Rust: `src/license_detection/detection.rs:568-610`
- Related: `docs/license-detection/PLAN-007-is-license-intro-clue-fix.md`
