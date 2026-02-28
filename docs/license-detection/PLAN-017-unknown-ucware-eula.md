# PLAN-017: unknown/ucware-eula.txt

## Status: ROOT CAUSE UPDATED - WEAK MATCH HANDLING

## Test File
`testdata/license-golden/datadriven/unknown/ucware-eula.txt`

## Issue
**Expected:** `["unknown-license-reference", "unknown-license-reference", "unknown", "warranty-disclaimer", "unknown", "swrule"]`
**Actual (golden test):** `["unknown", "warranty-disclaimer", "unknown"]`

## Current State Analysis (2026-02-28)

### Main Pipeline NOW Follows Python Correctly

The main detection pipeline in `src/license_detection/mod.rs:311-331` NOW follows Python's flow:

```rust
// Step 1: Initial refine WITHOUT false positive filtering
let merged_matches = refine_matches_without_false_positive_filter(&self.index, all_matches, &query);

// Step 2: Split weak from good
let (good_matches, weak_matches) = split_weak_matches(&merged_matches);

// Step 3: Unknown detection on uncovered regions
let mut all_matches = good_matches;
if unknown_licenses {
    let unknown_matches = unknown_match(&self.index, &query, &all_matches);
    let filtered_unknown = filter_invalid_contained_unknown_matches(&unknown_matches, &all_matches);
    all_matches.extend(filtered_unknown);
}
all_matches.extend(weak_matches);

// Step 4: Final refine WITH false positive filtering
let refined = refine_matches(&self.index, all_matches, &query);
```

### Remaining Issue: Weak Matches Being Filtered Out

After `refine_matches()` is called with `weak_matches` reinjected, the weak matches are being filtered out:

**From CLI output (with `unknown_licenses=false`):**
- `unknown-license-reference` at line 1 ✓
- `unknown-license-reference` at line 3 ✓
- `swrule` at line 31 ✓
- `warranty-disclaimer` at line 31 ✓

**From golden test (with `unknown_licenses=true`):**
- `unknown` at lines ??? (wrong)
- `warranty-disclaimer` ✓
- `unknown` at lines ??? (wrong)

**Missing:** `unknown-license-reference` x2, `swrule`, second `unknown`

### Root Cause: Refine Matches Filtering Weak Matches

The final `refine_matches()` call at step 4 is filtering out:
1. `unknown-license-reference` matches - these have `has_unknown() == true`
2. `swrule` match - possibly due to low coverage (10.6%)

Need to verify what filters are removing these matches in `refine_matches()`.

### Investigation Test Outdated

The investigation test `test_plan_017_rust_detection` in `unknown_ucware_test.rs` still uses the OLD pipeline approach and should be updated to match the main pipeline or removed.

## Expected Fix

### Fix 1: Investigate Why Refine Matches Removes Weak Matches

Need to trace through `refine_matches()` to find which filter is removing:
- `unknown-license-reference` matches
- `swrule` match

Possible culprits:
- `filter_below_rule_minimum_coverage()` - for low coverage seq matches
- `filter_false_positive_matches()` - if these are marked as FP
- `filter_contained_matches()` - if they overlap with unknown matches

### Fix 2: Ngram Search Scope (May Still Be Relevant)

Python's `get_matched_ngrams()` searches the FULL query tokens:
```python
qtokens = tuple(tokens)  # FULL query tokens, not region substring
for qend, _ in automaton.iter(qtokens):
    qend = qbegin + qend  # Adjusts positions with region offset
    qstart = qend - offset
    yield qstart, qend
```

Rust's `match_ngrams_in_region()` searches only the region substring:
```rust
let region_tokens = &tokens[start..end];  // Only region tokens
```

This may affect the `unknown` matches being generated.

### Fix 3: Update or Remove Investigation Test

The `test_plan_017_rust_detection` test uses an outdated pipeline approach that doesn't match the main detection pipeline. Either:
1. Update it to use `LicenseDetectionEngine::detect()` with `unknown_licenses=true`
2. Remove it entirely and rely on the golden test

## Success Criteria
- [x] Python implementation analyzed
- [x] Rust implementation analyzed  
- [x] Main pipeline now follows Python correctly
- [ ] Investigate why weak matches are filtered in final refine
- [ ] Fix weak match handling
- [ ] Tests pass

## Risk Analysis
**Medium risk** - The main pipeline now follows Python, but there's a subtle issue with how weak matches are handled in the final refine step. Need careful investigation to avoid breaking other tests.
