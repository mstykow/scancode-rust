# PLAN-010: x11_danse.txt

## Status: ROOT CAUSE IDENTIFIED - FIX READY

## Test File
`testdata/license-golden/datadriven/lic4/x11_danse.txt`

## Issue
Extra `unknown-license-reference` and wrong ordering.

**Expected:** `["x11 AND other-permissive"]`
**Actual:** `["unknown-license-reference AND other-permissive AND x11"]`

## Root Cause Analysis

### The Problem

**Python filters contained aho matches, Rust doesn't.**

Python's `get_exact_matches()` calls `refine_matches(merge=False)` which includes `filter_contained_matches()`. This removes contained matches like `x11_danse.RULE` which is contained within `other-permissive_339.RULE`.

Rust's `detect()` function calls `merge_overlapping_matches()` which only merges matches from the **same rule**. It does NOT filter contained matches from **different rules**.

### Key Evidence

**Python aho matches BEFORE refine:**
```
license-intro_94.RULE | qspan 4-7
license-intro_27.RULE | qspan 18-21
x11_danse.RULE | qspan 18-247  <-- contained in other-permissive
other-permissive_339.RULE | qspan 18-276  <-- contains x11_danse
unknown-license-reference_345.RULE | qspan 248-276
x11_danse2.RULE | qspan 277-317
```

**Python aho matches AFTER refine (with contained filter):**
```
license-intro_94.RULE | qspan 4-7
other-permissive_339.RULE | qspan 18-276  <-- x11_danse.RULE DISCARDED (reason=CONTAINED)
x11_danse2.RULE | qspan 277-317
```

**Rust aho matches (no contained filter):**
```
license-intro_94.RULE | qspan 4-7
other-permissive_339.RULE | qspan 18-276
x11_danse.RULE | qspan 18-247  <-- NOT FILTERED, causes problems
x11_danse2.RULE | qspan 277-317
```

### Why This Causes Problems

1. Rust keeps `x11_danse.RULE` (is_license_text=true, length=230)
2. This match triggers `is_license_text` subtraction (rule_length > 120, coverage > 98%)
3. Subtraction removes tokens 18-247 from query
4. `other-permissive_339.RULE` also triggers subtraction (tokens 18-276)
5. Query is now heavily subtracted BEFORE seq matching
6. Seq matching can't find the correct `x11_and_other-permissive_1.RULE` (the combined rule)
7. Final result is fragmented aho matches instead of combined seq match

### Python Code Reference

Python's `get_exact_matches()` in `index.py:677-697`:
```python
def get_exact_matches(self, query, matched_qspans, existing_matches, deadline):
    matches = match_aho.exact_match(...)
    # THIS IS THE KEY - refine_matches with filter_false_positive=False, merge=False
    matches, _discarded = match.refine_matches(
        matches=matches,
        query=query,
        filter_false_positive=False,
        merge=False,  # <-- no merge, but CONTAINED FILTERING still happens
    )
    return matches
```

Python's `refine_matches()` calls `filter_contained_matches()` at line 2781, even when `merge=False`.

### Fix Required

In `src/license_detection/mod.rs`, after `aho_match` and before using the matches, add a contained match filter:

```rust
// Phase 1c: Aho-Corasick matching
let aho_matches = aho_match(&self.index, &whole_run);

// Filter contained matches (Python: refine_matches with merge=False)
let filtered_aho = filter_contained_matches(&aho_matches);

for m in &filtered_aho {
    // ... rest of the logic
}
```

This matches Python's behavior in `get_exact_matches()` which calls `refine_matches(merge=False)` that still runs `filter_contained_matches()`.

### Implementation

1. Add `filter_contained_matches()` function to `match_refine.rs` (based on Python's `filter_contained_matches()`)
2. Use it in `detect()` after aho matching, before the is_license_text subtraction loop
3. This ensures contained matches are removed BEFORE they can trigger subtraction

## Investigation Files Created

- `src/license_detection/x11_danse_test.rs` - Tests comparing Rust vs Python behavior

## Tests to Fix

Once the fix is implemented, `test_x11_danse_expected_expression` should pass:

```rust
#[test]
fn test_x11_danse_expected_expression() {
    let expressions = /* detect() */;
    assert_eq!(expressions, vec!["x11 AND other-permissive"]);
}
```

## Next Steps

1. Implement `filter_contained_matches()` in `match_refine.rs`
2. Call it in `detect()` after aho matching
3. Run tests to verify fix
