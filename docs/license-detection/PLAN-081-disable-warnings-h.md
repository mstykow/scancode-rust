# PLAN-081: Aho Matches Override by Seq Matches - Implementation Plan

## Status: READY FOR IMPLEMENTATION

## Problem Summary

**File**: `testdata/license-golden/datadriven/lic4/disable_warnings.h`

| Expected (Python) | Actual (Rust) |
|-------------------|---------------|
| `["bsd-new OR gpl-2.0", "gpl-2.0", "bsd-new", "bsd-new"]` | `["gpl-2.0 OR bsd-new", "bsd-new"]` |

**Root Cause**: Rust runs both aho AND seq matching, then containment filtering removes correct aho matches when a longer seq match contains them. Python stops after aho matching if no matchable regions remain.

---

## Python Reference Behavior

**File**: `reference/scancode-toolkit/src/licensedcode/index.py` lines 1010-1067

```python
matchers = [
    Matcher(function=get_spdx_id_matches, include_low=True, name='spdx_lid', continue_matching=True),
    Matcher(function=self.get_exact_matches, include_low=False, name='aho', continue_matching=False),
]

if approximate:
    matchers += [Matcher(function=approx, include_low=False, name='seq', continue_matching=False), ]

already_matched_qspans = []
for matcher in matchers:
    matched = matcher.function(...)
    matched = match.merge_matches(matched)
    matches.extend(matched)
    
    # Track 100% coverage matches
    already_matched_qspans.extend(
        mtch.qspan for mtch in matched if mtch.coverage() == 100)
    
    # KEY LOGIC: Stop if no more matchable regions
    if not matcher.continue_matching:
        if not whole_query_run.is_matchable(
            include_low=matcher.include_low,
            qspans=already_matched_qspans,
        ):
            break  # STOP - don't run seq matching
```

**Key Points**:
1. `aho` has `continue_matching=False` - may stop after aho
2. `is_matchable(include_low=False)` checks if uncovered regions remain
3. If aho matches cover all matchable regions, seq is never run

---

## Implementation Plan

### Step 1: Add `is_matchable` Method to Query

**File**: `src/license_detection/query.rs`

Add a method to check if the query has remaining matchable regions:

```rust
/// Check if this query run has matchable regions not covered by matched_qspans.
///
/// When `include_low` is false, low-quality matches are excluded from coverage calculation.
/// This matches Python's QueryRun.is_matchable() behavior.
pub fn is_matchable(&self, include_low: bool, matched_qspans: &[PositionSpan]) -> bool {
    // Get the span of already matched positions
    let matched: std::collections::HashSet<usize> = matched_qspans
        .iter()
        .flat_map(|span| span.start..span.end)
        .collect();
    
    // Count matchable tokens not in matched spans
    let unmatched_matchables: usize = self.matchables
        .iter()
        .filter(|(pos, is_low)| {
            !matched.contains(pos) && (*is_low || include_low)
        })
        .count();
    
    unmatched_matchables > 0
}
```

### Step 2: Modify Detection Pipeline in `detect()`

**File**: `src/license_detection/mod.rs` lines 185-278

**Current Code** (simplified):
```rust
// Phase 1c: Aho-Corasick matching
{
    let whole_run = query.whole_query_run();
    let aho_matches = aho_match(&self.index, &whole_run);
    let merged_aho = merge_overlapping_matches(&aho_matches);
    // ... track matched_qspans ...
    all_matches.extend(merged_aho);
}

// Phases 2-4: Sequence matching (ALWAYS runs)
let mut seq_all_matches = Vec::new();
// ... seq matching code ...
all_matches.extend(merged_seq);
```

**New Code** (after fix):
```rust
// Phase 1c: Aho-Corasick matching
{
    let whole_run = query.whole_query_run();
    let aho_matches = aho_match(&self.index, &whole_run);
    let merged_aho = merge_overlapping_matches(&aho_matches);
    
    for m in &merged_aho {
        if m.match_coverage >= 99.99 && m.end_token > m.start_token {
            matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
        }
        if m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0 {
            let span = query::PositionSpan::new(m.start_token, m.end_token.saturating_sub(1));
            query.subtract(&span);
        }
    }
    all_matches.extend(merged_aho);
}

// CHECK: Should we continue with seq matching?
// Python: if not whole_query_run.is_matchable(include_low=False, qspans=already_matched_qspans)
//         then break - skip seq matching
let whole_run = query.whole_query_run();
let should_skip_seq = !whole_run.is_matchable(false, &matched_qspans);

if !should_skip_seq {
    // Phases 2-4: Sequence matching
    let mut seq_all_matches = Vec::new();
    // ... existing seq matching code ...
    let merged_seq = merge_overlapping_matches(&seq_all_matches);
    all_matches.extend(merged_seq);
}
```

### Step 3: Track 100% Coverage Matches

The Python code tracks 100% coverage matches for the `is_matchable` check:

```python
already_matched_qspans.extend(
    mtch.qspan for mtch in matched if mtch.coverage() == 100)
```

In Rust, this means we should track matches where `match_coverage >= 99.99` (accounting for floating point). The current code already does this:

```rust
if m.match_coverage >= 99.99 && m.end_token > m.start_token {
    matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
}
```

### Step 4: Update Query's `matchables` Structure

**File**: `src/license_detection/query.rs`

Ensure `QueryRun` tracks which tokens are "low" quality (for `include_low` parameter):

```rust
pub struct QueryRun {
    pub start: usize,
    pub end: usize,
    /// Matchable tokens as (position, is_low) pairs
    /// is_low indicates if the token is a low-quality match candidate
    pub matchables: Vec<(usize, bool)>,
}
```

---

## Testing Strategy

### Unit Test: `test_aho_priority_over_seq`

Create a test in `src/license_detection/mod.rs`:

```rust
#[test]
fn test_aho_priority_over_seq_disable_warnings() {
    let Some(engine) = create_engine_from_reference() else {
        eprintln!("Skipping test: reference directory not found");
        return;
    };

    let test_file = std::path::PathBuf::from("testdata/license-golden/datadriven/lic4/disable_warnings.h");
    let text = match std::fs::read_to_string(&test_file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Skipping test: cannot read test file: {}", e);
            return;
        }
    };

    let detections = engine.detect(&text, false).expect("Detection failed");
    
    // Expected: bsd-new OR gpl-2.0 (lines 2-5)
    // Expected: gpl-2.0 (lines 9-22)
    // Expected: bsd-new (lines 27-27)
    // Expected: bsd-new (lines 32-56)
    
    let all_expressions: Vec<_> = detections
        .iter()
        .flat_map(|d| d.matches.iter().map(|m| m.license_expression.as_str()))
        .collect();
    
    // Should NOT have "gpl-2.0 OR bsd-new" (wrong order from seq match)
    assert!(
        !all_expressions.iter().any(|e| *e == "gpl-2.0 OR bsd-new"),
        "Should not have 'gpl-2.0 OR bsd-new' from seq matcher, got: {:?}",
        all_expressions
    );
    
    // Should have "bsd-new OR gpl-2.0" (correct from aho match)
    assert!(
        all_expressions.iter().any(|e| *e == "bsd-new OR gpl-2.0"),
        "Should have 'bsd-new OR gpl-2.0' from aho matcher, got: {:?}",
        all_expressions
    );
    
    // Should have gpl-2.0
    assert!(
        all_expressions.iter().any(|e| *e == "gpl-2.0"),
        "Should have 'gpl-2.0' from aho matcher, got: {:?}",
        all_expressions
    );
}
```

### Golden Test Verification

Run the existing golden test to verify:

```bash
cargo test test_license_detection_golden -- --nocapture
```

The `disable_warnings.h` test should now pass with:
- `detected_license_expression: '((bsd-new OR gpl-2.0) AND gpl-2.0) AND bsd-new'`

### Regression Test

Run all license detection tests to ensure no regressions:

```bash
cargo test license_detection -- --nocapture
```

---

## Implementation Checklist

- [ ] Add `is_matchable()` method to `QueryRun` in `src/license_detection/query.rs`
- [ ] Modify `detect()` in `src/license_detection/mod.rs` to check `is_matchable` after aho matching
- [ ] Skip seq matching phases (2-4) if no matchable regions remain
- [ ] Add unit test `test_aho_priority_over_seq_disable_warnings`
- [ ] Run golden tests to verify fix
- [ ] Run full test suite for regression check

---

## Edge Cases to Consider

1. **Partial aho coverage**: If aho matches cover some but not all regions, seq should still run on uncovered regions.

2. **Mixed matcher results**: SPDX-LID and aho matches may coexist. The `continue_matching=True` for SPDX means we always continue after SPDX, but aho may stop further matching.

3. **Query run subtraction**: Long license text matches subtract their span from the query. This affects `is_matchable` calculation.

4. **Multiple query runs**: The detection pipeline may have multiple query runs. Each should independently check matchability.

---

## Files to Modify

1. **`src/license_detection/query.rs`** - Add `is_matchable()` method
2. **`src/license_detection/mod.rs`** - Add early-exit logic after aho matching
3. **`src/license_detection/mod.rs`** - Add unit test

---

## Risk Assessment

**Low Risk**: The change adds an early-exit condition that matches Python behavior. If the condition is not met, the existing seq matching logic runs unchanged. This is a safe, targeted fix.

**Potential Issues**: 
- Some edge cases where seq matching was providing value might lose those matches
- However, these would be incorrect matches per Python's behavior, so this is actually correct
