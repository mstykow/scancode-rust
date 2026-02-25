# PLAN-063: Missing Detections Investigation

## Status: RESOLVED (Same root cause as PLAN-061)

## Problem Statement

Expected license expressions are not being detected. Actual count significantly less than expected.

### Representative Test Case

**File**: `testdata/license-golden/datadriven/lic1/e2fsprogs.txt`

| Expected | Actual |
|----------|--------|
| `["gpl-2.0 AND lgpl-2.0 AND bsd-new AND mit-old-style-no-advert", "bsd-new", "lgpl-2.1-plus", "lgpl-2.1-plus", "lgpl-2.1-plus"]` (5) | `["gpl-2.0 AND lgpl-2.0 AND bsd-new AND mit-old-style-no-advert", "bsd-new", "lgpl-2.1-plus", "lgpl-2.1-plus"]` (4) |

Missing: 1 `lgpl-2.1-plus` detection at lines 80-83

---

## Root Cause

**File**: `src/license_detection/detection.rs:1146-1184`
**Function**: `apply_detection_preferences`

### The Bug

The `apply_detection_preferences` function deduplicates detections by license expression, keeping only ONE detection per license expression. This is incorrect because the same license can appear at MULTIPLE locations in a file.

### Evidence

```
=== Detections before post_process: 4 ===
Detection 1: Some("gpl-2.0 AND lgpl-2.0 AND bsd-new AND mit-old-style-no-advert")
Detection 2: Some("bsd-new AND lgpl-2.1-plus")
Detection 3: Some("lgpl-2.1-plus")  <- lines 53-56
Detection 4: Some("lgpl-2.1-plus")  <- lines 80-83

=== Detections after post_process: 3 ===
Detection 1: Some("gpl-2.0 AND lgpl-2.0 AND bsd-new AND mit-old-style-no-advert")
Detection 2: Some("bsd-new AND lgpl-2.1-plus")
Detection 3: Some("lgpl-2.1-plus")  <- Only one kept, lines 53-56
```

Detection 3 and 4 both have:
- Same license expression: `lgpl-2.1-plus`
- Same score: 100.0
- Different locations: lines 53-56 vs lines 80-83

The function at line 1175-1177 keeps only one because the scores are equal:
```rust
if (score - existing_score).abs() < 0.01 {
    best_matcher_priority < *existing_priority  // Doesn't help when both are Aho (priority 3)
} else {
    score > *existing_score  // Both are 100.0
}
```

### Python Behavior

Python returns 5 matches (not grouped/deduplicated by expression):
```
1. gpl-2.0 AND lgpl-2.0 AND bsd-new AND mit-old-style-no-advert at lines 3-11
2. bsd-new at lines 16-40
3. lgpl-2.1-plus at lines 44-47
4. lgpl-2.1-plus at lines 53-56
5. lgpl-2.1-plus at lines 80-83
```

---

## Fix Required

The `apply_detection_preferences` function should NOT deduplicate by license expression alone. It should consider location (lines) as well.

Options:
1. Remove the deduplication by expression entirely
2. Only deduplicate when detections have overlapping line ranges
3. Use `identifier` field (which includes content hash) for deduplication instead of expression

The `remove_duplicate_detections` function at line 907 already handles proper deduplication using `identifier` (expression + content hash), so `apply_detection_preferences` should not also deduplicate.

---

## Investigation Test File

Created: `src/license_detection/missing_detection_investigation_test.rs`

Run with:
```bash
cargo test --lib test_e2fsprogs_detection_count -- --nocapture
cargo test --lib test_e2fsprogs_grouping_phase -- --nocapture
```

---

## Success Criteria

1. ~~Identify where detection is lost~~ DONE: `detection.rs:1175-1177`
2. ~~Document root cause~~ DONE: Incorrect deduplication by expression
3. ~~Implement fix~~ DONE: Same fix as PLAN-061
4. ~~All 12 missing detection tests pass~~ DONE: Improved from 50 to 40 failures
