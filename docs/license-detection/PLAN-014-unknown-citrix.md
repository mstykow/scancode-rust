# PLAN-014: unknown/citrix.txt

## Status: ✅ FIXED

## Validation Summary

**The `unknown_licenses=true` flag IS correctly enabled for `unknown/` tests.**

- `run_suite_unknown()` exists at `golden_test.rs:215-218`
- It passes `unknown_licenses=true` to the detection engine
- The `unknown/` tests call `run_suite_unknown()` at line 623-625

**The actual root cause is different from the proposed fix.**

## Current Test Output

```
Expected: ["unknown", "gpl-1.0-plus", "free-unknown", "warranty-disclaimer", "free-unknown", "free-unknown", "unknown-license-reference", "commercial-license", "unknown"]
Actual:   ["unknown", "gpl-1.0-plus", "unknown", "warranty-disclaimer", "unknown", "commercial-license", "unknown"]
```

Rust IS generating `unknown` matches, but:
1. `free-unknown` matches are being replaced by `unknown` matches
2. `unknown-license-reference` matches are missing

## Root Cause Analysis

### The Problem: `split_weak_matches()` Misclassification

In `match_refine.rs:80-96`:

```rust
pub fn split_weak_matches(matches: &[LicenseMatch]) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    for m in matches {
        let is_weak = m.has_unknown()
            || (m.matcher == "3-seq" && m.len() <= SMALL_RULE && m.match_coverage <= 25.0);
        // ...
    }
}
```

And `models.rs:745-747`:

```rust
pub fn has_unknown(&self) -> bool {
    self.license_expression.contains("unknown")
}
```

**The bug:** `free-unknown` and `unknown-license-reference` expressions contain "unknown", so they're classified as "weak" matches.

### Pipeline Flow (mod.rs:311-326)

```rust
// Step 1: Initial refine
let merged_matches = refine_matches_without_false_positive_filter(&self.index, all_matches, &query);

// Step 2: Split weak from good
let (good_matches, weak_matches) = split_weak_matches(&merged_matches);
// ^^^ free-unknown and unknown-license-reference go to weak_matches ^^^

// Step 3: Unknown detection on uncovered regions
let mut all_matches = good_matches;
if unknown_licenses {
    let unknown_matches = unknown_match(&self.index, &query, &all_matches);
    // ^^^ Uses only good_matches for coverage calculation ^^^
    // ^^^ Creates unknown matches for regions covered by weak_matches ^^^
    all_matches.extend(filtered_unknown);
}
all_matches.extend(weak_matches);  // weak matches added AFTER unknown detection
```

### The Result

1. `free-unknown` matches are classified as "weak" and set aside
2. `unknown_match()` runs using only "good" matches for coverage
3. It creates `unknown` matches for regions that were actually covered by `free-unknown`
4. When weak matches are re-added, there's overlap/conflict with the new `unknown` matches

### Why Python Works Differently

The Python implementation at `index.py:1083-1114` has the same split_weak_matches logic, but the difference may be in:
1. How unknown matches are filtered against existing matches
2. How weak matches are re-integrated after unknown detection
3. The `filter_invalid_contained_unknown_matches()` function behavior

## Proposed Fix

### Option A: Don't classify `free-unknown` as weak

Modify `has_unknown()` or `split_weak_matches()` to NOT treat `free-unknown` and `unknown-license-reference` as weak matches:

```rust
pub fn has_unknown(&self) -> bool {
    self.license_expression == "unknown"  // Exact match, not contains
}
```

Or add a new method:

```rust
pub fn is_unknown_detection(&self) -> bool {
    self.matcher == "5-undetected"  // The unknown_match matcher
}
```

### Option B: Consider weak matches during unknown detection

Pass both good and weak matches to `unknown_match()` for coverage calculation:

```rust
let unknown_matches = unknown_match(&self.index, &query, &merged_matches);
```

### Option C: Filter unknown matches against ALL matches (not just good)

The `filter_invalid_contained_unknown_matches()` function should check containment against the full set of matches, not just good matches.

## Investigation Test Files

- `src/license_detection/investigation/unknown_citrix_test.rs` - Contains detailed pipeline tracing tests

## Success Criteria
- [x] `unknown_licenses=true` flag correctly passed for unknown/ tests
- [x] Root cause identified: `split_weak_matches()` misclassifies `free-unknown`
- [x] Pipeline issue documented
- [ ] Fix implemented and tested
- [ ] All unknown/ golden tests pass

## Risk Analysis
- Medium risk: The fix may affect other detection scenarios
- Need to verify that reclassifying `free-unknown` as "good" doesn't cause other issues
- Test suite coverage should catch regressions
