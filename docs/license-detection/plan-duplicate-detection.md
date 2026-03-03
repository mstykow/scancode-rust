# Implementation Plan: Missing Duplicate Detections

**Status:** Verified - Needs Refinement  
**Created:** 2026-03-03  
**Priority:** High  
**Category:** License Detection Correctness  
**Verified:** 2026-03-03 - Code locations confirmed, root cause analysis validated

## Executive Summary

Rust incorrectly merges/deduplicates license matches that Python keeps as separate detections. When the same license appears multiple times in a file at different locations, Python creates multiple detections (one per location), but Rust creates only one detection (merged/deduplicated).

**Example:**
```
File: gpl-2.0_or_bsd-new_intel_kernel.c

Python expected: ["bsd-new OR gpl-2.0", "gpl-2.0", "bsd-new", "bsd-new"]
Rust actual:     ["bsd-new OR gpl-2.0"]  (missing 3 detections)
```

**Affected Files:**
- Kernel files with `MODULE_LICENSE` and `EXPORT_SYMBOL_GPL` macros
- Python license files (python-*.txt) where the license text appears twice
- Multi-license files where the same license appears at different locations
- Files with `ms-pl` appearing 3 times but only 1 detected

## Root Cause Analysis

### 1. The Core Problem: `filter_contained_matches()` Over-Filtering

**Location:** `src/license_detection/match_refine/handle_overlaps.rs:40-96`

The `filter_contained_matches()` function removes matches that are "contained" within other matches. The issue is that **matches at non-overlapping locations are being incorrectly marked as "contained"** and removed.

#### Python vs Rust Comparison

**Python `filter_contained_matches()` (match.py:1075-1184):**
```python
def filter_contained_matches(matches, ...):
    # Sort by (qspan.start, -hilen, -len, matcher_order)
    sorter = lambda m: (m.qspan.start, -m.hilen(), -m.len(), m.matcher_order)
    matches = sorted(matches, key=sorter)
    
    while i < len(matches) - 1:
        j = i + 1
        while j < len(matches):
            # BREAK when no overlap possible
            if next_match.qend > current_match.qend:
                break  # <-- CRITICAL: stops when matches don't overlap
            
            # Remove contained (qspan fully inside another)
            if current_match.qcontains(next_match):
                discarded_append(matches_pop(j))
                continue
            
            if next_match.qcontains(current_match):
                discarded_append(matches_pop(i))
                i -= 1
                break
            
            j += 1
        i += 1
```

**Rust `filter_contained_matches()` (handle_overlaps.rs:40-96):**
```rust
pub fn filter_contained_matches(matches: &[LicenseMatch]) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    matches.sort_by(|a, b| {
        a.qstart()
            .cmp(&b.qstart())
            .then_with(|| b.hilen.cmp(&a.hilen))
            .then_with(|| b.matched_length.cmp(&a.matched_length))
            .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
    });

    while i < matches.len().saturating_sub(1) {
        j = i + 1;
        while j < matches.len() {
            // ISSUE: This break condition is wrong
            if next.end_token > current.end_token {
                break;  // <-- SHOULD be: if next.qstart() >= current.qend()
            }
            // ...
        }
    }
}
```

**The Bug:** 
- Python breaks when `next_match.qend > current_match.qend` (meaning the next match extends past the current match's end, so no containment is possible)
- Rust breaks when `next.end_token > current.end_token`
- These are **NOT equivalent** because `qend` is the **qspan end** while `end_token` is a different field
- When matches are at completely different locations (non-overlapping), the break condition should trigger and stop filtering
- Instead, Rust continues processing and may incorrectly filter non-overlapping matches

### 2. Secondary Issue: `qcontains()` Implementation Differences

**Location:** `src/license_detection/models/license_match.rs:342-369`

The `qcontains()` method checks if one match's qspan contains another's. There may be edge cases where:
- Token position handling differs from Python's Span-based containment
- The fallback to line-based containment when tokens are 0 may behave differently

### 3. Tertiary Issue: Detection Grouping Logic

**Location:** `src/license_detection/detection/grouping.rs:7-64`

The `group_matches_by_region()` function groups matches that are within `LINES_THRESHOLD=4` lines of each other. However:
- The test comparison (`golden_test.rs:164-168`) flattens all matches from all detections
- If matches are incorrectly grouped, fewer detections are created
- This interacts with the filtering issue above

### 4. Test Comparison Difference

**Location:** `src/license_detection/golden_test.rs:164-168`

```rust
let actual: Vec<&str> = detections
    .iter()
    .flat_map(|d| d.matches.iter())
    .map(|m| m.license_expression.as_str())
    .collect();
```

**Python test comparison (`licensedcode_test_utils.py:215`):**
```python
detected_expressions = [match.rule.license_expression for match in matches]
```

Both extract expressions from **matches**, not detections. The difference is:
- Python: `idx.match()` returns matches after `refine_matches()` (no grouping)
- Rust: `detect()` returns matches from grouped detections

This is **correct behavior** - the issue is in the filtering/grouping, not the test comparison.

## Specific Code Locations Needing Changes

### Primary Fix: `filter_contained_matches()` Break Condition

**File:** `src/license_detection/match_refine/handle_overlaps.rs`  
**Lines:** 65-67

**Current (Wrong):**
```rust
if next.end_token > current.end_token {
    break;
}
```

**Should Be:**
```rust
// Break when no overlap possible (next starts at or after current ends)
// Python: if next_match.qend > current_match.qend: break
// But we need to check qstart >= qend for NO OVERLAP
if next.qstart() >= current.qend() {
    break;
}
```

**Wait - deeper analysis needed:** The Python condition `if next_match.qend > current_match.qend: break` means:
- If next match's END is past current match's END, break
- This is because: sorted by qstart, if next extends past current's end, containment is not possible (next could contain current, not the other way)
- But it still checks `next_match.qcontains(current_match)` after the break condition

Actually, the Python code has:
```python
if next_match.qend > current_match.qend:
    break  # No containment possible going forward
```

This is correct because matches are sorted by `qspan.start`. If `next.qend > current.qend`, then `next` extends past `current`, so `next` cannot be contained in `current`. But `current` could still be contained in `next`, so Python checks that BEFORE breaking...

Wait, let me re-read Python:

```python
while j < len(matches):
    # BREAK when next extends past current
    if next_match.qend > current_match.qend:
        break  # <-- THIS IS WRONG for the containment check!
    
    # These checks happen BEFORE the break would prevent them
    if current_match.qcontains(next_match):
        discarded_append(matches_pop(j))
        continue
    
    if next_match.qcontains(current_match):
        discarded_append(matches_pop(i))
        break  # different break
    
    j += 1
```

Actually wait, the break happens BEFORE the qcontains checks. So if `next.qend > current.qend`, Python breaks immediately without checking containment.

This means Python's logic is:
1. Sorted by qspan.start
2. For each pair (current, next):
   - If next.qend > current.qend, break (no more j iterations for this i)
   - Check if current contains next → discard next
   - Check if next contains current → discard current, break

The issue is that this break condition assumes all remaining matches will also extend past current.qend, which is true because they're sorted by qstart.

So the Rust equivalent should be:
```rust
if next.qend() > current.qend() {
    break;
}
```

But we need to verify what `qend()` returns in Rust vs Python.

**Python qend (match.py:426-427):**
```python
@property
def qend(self):
    return self.qspan.end
```

**Rust qspan_bounds() (license_match.rs:476-488):**
```rust
pub fn qspan_bounds(&self) -> (usize, usize) {
    if let Some(positions) = &self.qspan_positions {
        if positions.is_empty() {
            return (0, 0);
        }
        (
            *positions.iter().min().unwrap(),
            *positions.iter().max().unwrap() + 1,
        )
    } else {
        (self.start_token, self.end_token)
    }
}
```

**IMPORTANT: Rust does NOT have a `qend()` method!** The code uses `end_token` directly in the break condition. When `qspan_positions` is None, `end_token` is equivalent to Python's `qend`. When `qspan_positions` is set, Rust should use the max position from `qspan_bounds()` instead.

The break condition uses `end_token` directly, which is correct when `qspan_positions` is None (the common case). However, if matches have `qspan_positions` set, the break condition may be incorrect.

### Re-analyzing the Root Cause

Let me look at the test file `gpl-2.0_or_bsd-new_intel_kernel.c`:
- Expected: 4 expressions (`bsd-new OR gpl-2.0`, `gpl-2.0`, `bsd-new`, `bsd-new`)
- This suggests 4 separate matches that should NOT be contained in each other

The file has:
1. Header with dual BSD/GPL license text (lines 1-45)
2. Contains both GPL and BSD license sections

The expected output shows:
1. `bsd-new OR gpl-2.0` - the header declaring dual license
2. `gpl-2.0` - GPL license text
3. `bsd-new` - BSD license text (first occurrence)
4. `bsd-new` - BSD license text (second occurrence? or another BSD reference)

If Rust only produces `["bsd-new OR gpl-2.0"]`, it means:
- Either the GPL and BSD matches are being filtered as "contained"
- Or they're being merged into one detection
- Or they're never being matched in the first place

### The Real Issue: `post_process_detections()` Deduplication

**Location:** `src/license_detection/detection/mod.rs:349-372`

```rust
pub fn post_process_detections(
    detections: Vec<LicenseDetection>,
    min_score: f32,
) -> Vec<LicenseDetection> {
    let filtered = filter_detections_by_score(detections, min_score);
    // NOTE: We do NOT call remove_duplicate_detections here.
    // ...
    let preferred = apply_detection_preferences(filtered);
    let ranked = rank_detections(preferred);
    sort_detections_by_line(ranked)
}
```

The code correctly does NOT call `remove_duplicate_detections`, but the issue is in how detections are created.

### The Actual Root Cause: Detection Grouping

**Location:** `src/license_detection/detection/grouping.rs:21-64`

```rust
pub(super) fn group_matches_by_region_with_threshold(
    matches: &[LicenseMatch],
    proximity_threshold: usize,
) -> Vec<DetectionGroup> {
    // Groups matches where line_gap <= threshold (default 4)
}
```

If all the matches are within 4 lines of each other, they'll be grouped into ONE detection!

Looking at the kernel file (45 lines), the GPL and BSD sections might all be within proximity threshold, causing them to be grouped together.

**BUT** - the expected output shows 4 separate expressions. Python must be creating 4 separate matches/groups.

### Key Insight: Python Does NOT Group into Detections

**Python test (`licensedcode_test_utils.py:207-215`):**
```python
matches = idx.match(location=test_file, min_score=0, unknown_licenses=unknown_detection)
detected_expressions = [match.rule.license_expression for match in matches]
```

Python extracts expressions from **matches** (not detections). There's no grouping step in the Python test!

**Rust test (`golden_test.rs:164-168`):**
```rust
let detections = engine.detect(&text, unknown_licenses)?;
let actual: Vec<&str> = detections
    .iter()
    .flat_map(|d| d.matches.iter())
    .map(|m| m.license_expression.as_str())
    .collect();
```

Rust extracts from matches within detections. This SHOULD be equivalent IF:
1. Each detection contains the correct matches
2. Matches aren't being filtered incorrectly

### The Real Difference: Python Returns Matches, Rust Returns Detections

Looking at Python's `idx.match()`:
```python
# index.py:1131-1137
matches, _discarded = match.refine_matches(
    matches=matches,
    query=qry,
    min_score=min_score,
    filter_false_positive=True,
    merge=True,
)
matches.sort()
return matches  # Returns matches directly!
```

Python's `idx.match()` returns matches after `refine_matches()` - **no grouping, no detection creation**.

But Rust's `detect()`:
1. Calls `refine_matches()` ✓
2. Groups matches by region ✗ (Python doesn't do this)
3. Creates detections from groups ✗ (Python doesn't do this)
4. Returns detections ✗ (Python returns matches)

**This is the fundamental difference!**

The Rust test extracts from `detection.matches`, but the grouping step may have merged matches incorrectly.

## Implementation Steps

### Step 1: Verify the Break Condition is Correct

Compare Python and Rust break conditions in `filter_contained_matches()`:

**Python:**
```python
if next_match.qend > current_match.qend:
    break
```

**Rust:**
```rust
if next.end_token > current.end_token {
    break;
}
```

These should be equivalent. Verify `qend()` returns `end_token` consistently.

### Step 2: Check `qcontains()` for Edge Cases

The `qcontains()` method should return `false` for matches at non-overlapping locations. Verify:
- Matches at different lines with `start_token == 0 && end_token == 0`
- Matches with `qspan_positions` set vs not set

### Step 3: Investigate Detection Grouping

The `group_matches_by_region()` function groups matches within `LINES_THRESHOLD=4` lines. For the kernel test case:
- GPL section: lines 4-15
- BSD section: lines 17-45

Gap between GPL end (15) and BSD start (17) is 2 lines, which is ≤ 4, so they get grouped!

**BUT** Python doesn't group at all - it returns matches directly.

### Step 4: Consider Not Grouping for Golden Tests

The golden tests compare against Python's `idx.match()` output, which returns matches directly without grouping.

Options:
1. **Option A**: Change Rust test to not group (return matches directly like Python)
2. **Option B**: Change Python test to use detections (not viable - can't change reference)
3. **Option C**: Make grouping produce the same match list as Python

**Option A is correct** - the test should match Python's behavior.

### Step 5: Fix the Pipeline

The correct fix is to ensure that after `refine_matches()`, the matches are in the same state as Python's `idx.match()` output.

Looking at the Rust pipeline:
```rust
// Step 5: Final refine WITH false positive filtering
let refined = refine_matches(&self.index, refined_matches, &query);
let mut sorted = refined;
sort_matches_by_line(&mut sorted);

// GROUPING HAPPENS HERE - Python doesn't do this
let groups = group_matches_by_region(&sorted);
let detections: Vec<LicenseDetection> = groups.iter()...
```

The issue is that `group_matches_by_region()` is called, but Python's test compares matches BEFORE grouping.

### Step 6: Add Investigation Test

Create a test that compares:
1. Matches after `refine_matches()` (before grouping)
2. Matches in detections after grouping

This will show if grouping is the issue.

## Test Cases to Verify the Fix

### Test Case 1: Non-overlapping Same License

```rust
// File with same license text at two different locations
// Expected: 2 matches, NOT merged
#[test]
fn test_non_overlapping_same_license_not_merged() {
    // GPL-2.0 text at lines 1-20
    // GPL-2.0 text at lines 50-70
    // Should produce 2 separate matches
}
```

### Test Case 2: Contained Match Filtering

```rust
#[test]
fn test_filter_contained_matches_non_overlapping() {
    let m1 = create_match("gpl-2.0", 0, 100);  // lines 0-100
    let m2 = create_match("gpl-2.0", 200, 300); // lines 200-300 (non-overlapping)
    
    let (kept, discarded) = filter_contained_matches(&[m1.clone(), m2.clone()]);
    
    assert_eq!(kept.len(), 2, "Non-overlapping matches should both be kept");
    assert_eq!(discarded.len(), 0);
}
```

### Test Case 3: Overlapping Different Expressions

```rust
#[test]
fn test_overlapping_different_expressions_kept() {
    let m1 = create_match("gpl-2.0", 0, 50);
    let m2 = create_match("bsd-new", 30, 80);  // Overlaps m1
    
    let (kept, _) = filter_contained_matches(&[m1, m2]);
    
    // Both should be kept - neither contains the other
    assert_eq!(kept.len(), 2);
}
```

### Test Case 4: Golden Test for Kernel File

```rust
#[test]
fn test_gpl_bsd_kernel_detections() {
    let content = include_str!("../../testdata/license-golden/.../gpl-2.0_or_bsd-new_intel_kernel.c");
    let engine = LicenseDetectionEngine::new(...).unwrap();
    let detections = engine.detect(content, false).unwrap();
    
    let expressions: Vec<_> = detections
        .iter()
        .flat_map(|d| d.matches.iter())
        .map(|m| m.license_expression.as_str())
        .collect();
    
    assert_eq!(expressions, vec!["bsd-new OR gpl-2.0", "gpl-2.0", "bsd-new", "bsd-new"]);
}
```

## Risk Assessment

### Low Risk Changes
- Adding unit tests for `filter_contained_matches()` edge cases
- Adding investigation/debugging code

### Medium Risk Changes
- Modifying break condition in `filter_contained_matches()` if truly incorrect
- Adjusting detection grouping threshold

### High Risk Changes
- Changing the fundamental pipeline structure (removing grouping)
- Any change that affects the majority of golden tests

### Backward Compatibility
- Changes must maintain correct behavior for files that already work
- Run full golden test suite before/after to catch regressions

## Related Documents

- [0017-phase1-duplicate-detection-plan.md](0017-phase1-duplicate-detection-plan.md) - Original analysis
- [0015-filter-dupes-regressions.md](0015-filter-dupes-regressions.md) - Related regression investigation
- [PLAN-019-unique-detection.md](PLAN-019-unique-detection.md) - Unique detection design

## Next Steps

1. **Create investigation test** comparing matches before/after grouping
2. **Run Python ScanCode** on specific failing test files to understand expected behavior
3. **Trace specific matches** through the pipeline to identify where they're lost
4. **Fix identified issue** with targeted code change
5. **Verify fix** with unit tests and golden test suite

## Critical Discovery: Python Returns Raw Matches, Rust Returns Detections

After verifying the code:

**Python test (`licensedcode_test_utils.py:207-215`):**
```python
matches = idx.match(location=test_file, min_score=0, unknown_licenses=unknown_detection)
detected_expressions = [match.rule.license_expression for match in matches]
```

**Python `idx.match()` (index.py:1131-1139):**
```python
matches, _discarded = match.refine_matches(
    matches=matches,
    query=qry,
    min_score=min_score,
    filter_false_positive=True,
    merge=True,
)
matches.sort()
return matches  # Returns raw matches, NO grouping
```

**Rust `detect()` (mod.rs:320-336):**
```rust
// Step 5: Final refine WITH false positive filtering
let refined = refine_matches(&self.index, refined_matches, &query);
let mut sorted = refined;
sort_matches_by_line(&mut sorted);

// GROUPING HAPPENS HERE - Python doesn't do this
let groups = group_matches_by_region(&sorted);
let detections: Vec<LicenseDetection> = groups.iter()...
```

**The fundamental difference:**
- Python's `idx.match()` returns matches directly after `refine_matches()` - no grouping, no detection creation
- Rust's `detect()` groups matches by region (`group_matches_by_region()`) and creates detections

**The golden test comparison:**
- Rust test extracts from `detection.matches` (matches inside detections)
- If grouping merges matches incorrectly, the test will fail

**Two possible fixes:**
1. **Option A**: Add a `detect_raw()` method that returns matches without grouping (for golden tests)
2. **Option B**: Fix `group_matches_by_region()` to produce groups whose flattened matches match Python's output

**Recommended approach: Option A** - Create a method that returns raw matches like Python's `idx.match()` for accurate golden test comparison.
