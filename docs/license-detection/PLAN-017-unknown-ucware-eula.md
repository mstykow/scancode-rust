# PLAN-017: unknown/ucware-eula.txt

## Status: ROOT CAUSE IDENTIFIED - INVESTIGATION TEST DOES NOT MATCH MAIN PIPELINE

## Test File
`testdata/license-golden/datadriven/unknown/ucware-eula.txt`

## Issue
**Expected:** `["unknown-license-reference", "unknown-license-reference", "unknown", "warranty-disclaimer", "unknown", "swrule"]`
**Actual (golden test):** `["unknown", "warranty-disclaimer", "unknown"]`

## Root Cause Analysis

### Pipeline Mismatch

The investigation test (`test_plan_017_rust_detection`) does NOT follow the main pipeline:

**Investigation test (INCORRECT):**
```rust
all_matches.extend(hash_match(...));
all_matches.extend(spdx_lid_match(...));
all_matches.extend(aho_match(...));
all_matches.extend(seq_match_with_candidates(...));
all_matches.extend(unknown_match(&index, &query, &all_matches));  // Called with ALL matches
let refined = refine_matches(&index, all_matches, &query);  // Single refine call
```

**Main pipeline (CORRECT):**
```rust
let merged_matches = refine_matches_without_false_positive_filter(&self.index, all_matches, &query);
let (good_matches, weak_matches) = split_weak_matches(&merged_matches);  // KEY STEP!
let mut all_matches = good_matches;
if unknown_licenses {
    let unknown_matches = unknown_match(&self.index, &query, &all_matches);  // Called with ONLY good matches
    all_matches.extend(filtered_unknown);
}
all_matches.extend(weak_matches);  // Weak added back AFTER unknown detection
let refined = refine_matches(&self.index, all_matches, &query);
```

### The Key Difference

1. **Main pipeline calls `unknown_match` AFTER removing weak matches:**
   - Weak matches (`unknown-license-reference`, `swrule`) are removed first
   - `unknown_match` finds uncovered regions and creates `unknown` matches
   - `unknown` matches are created for regions NOT covered by good matches

2. **Weak matches are added back AFTER unknown detection:**
   - This is the critical step - weak matches (including `unknown-license-reference` and `swrule`) are added back

3. **Final refine filters them out:**
   - Something in `refine_matches()` filters out the `unknown-license-reference` and `swrule` matches
   - The `filter_license_references_with_text_match()` or `filter_contained_matches()` is removing them

### Evidence

Investigation test (single refine call, unknown_match called with ALL matches):
```
PHASE 1: unknown-license-reference matches: 53, swrule matches: 6, unknown matches: 0
AFTER refine: 4 matches: unknown-license-reference (x2), swrule, warranty-disclaimer
```

Golden test (full pipeline, unknown_match called after split_weak_matches):
```
Actual: ["unknown", "warranty-disclaimer", "unknown"]
```

### The Bug Location

The bug is in the filtering during `refine_matches()` when weak matches are added back after unknown detection.

When:
- `unknown` match (from unknown detection) overlaps with or is near
- `unknown-license-reference` match (from weak matches)

The filtering logic in `filter_license_references_with_text_match()` or `filter_contained_matches()` removes the `unknown-license-reference` match.

**Specific issue:** In `filter_contained_matches()` at lines 420-430:
```rust
if overlap > 0 {
    if current_len >= next_len && current_hilen >= next_hilen {
        if licensing_contains_match(&current, &next) {
            discarded.push(matches.remove(j));  // Removes match
            continue;
        }
    }
}
```

**OR** in `filter_license_references_with_text_match()` at lines 505-512:
```rust
if other_len >= current_len && other.hilen() >= current.hilen() {
    if licensing_contains_match(other, current) && other.qcontains(current) {
        to_discard.insert(i);
    }
}
```

### Why Python Doesn't Have This Issue

Python's `filter_contained_matches()` only uses `qcontains()` for token position containment:
```python
if current_match.qcontains(next_match):
    discarded_append(matches_pop(j))
    continue
```

Python does NOT have the expression-based containment logic that was added to Rust.

### Proposed Fix

1. **Verify** the expression-based containment is the issue by temporarily disabling it
2. **Fix** the filtering logic to NOT filter `unknown-license-reference` matches when they overlap with `unknown` matches
3. The key insight: `unknown-license-reference` is a legitimate license expression that should be kept even when `unknown` matches are present

## Success Criteria
- [x] Python implementation analyzed
- [x] Rust implementation analyzed  
- [x] Main pipeline follows Python correctly
- [x] Issue confirmed to persist
- [x] Identified pipeline difference between test and main code
- [x] Identified expression-based containment as likely cause
- [ ] Verify fix by testing without expression-based containment
- [ ] Implement proper fix
- [ ] Tests pass

## Risk Analysis
**Medium risk** - The expression-based containment was added to fix other tests. Need to understand the tradeoffs.
