# PLAN-087: ijg.txt Investigation

## Status: IMPLEMENTATION PLAN READY

## Problem Statement

**File**: `testdata/license-golden/datadriven/lic4/ijg.txt`

| Expected (Python) | Actual (Rust) |
|-------------------|---------------|
| `["ijg"]` (1) | `["ijg", "warranty-disclaimer", "ijg", "free-unknown", "free-unknown"]` (5) |

**Issue**: Many extra detections - `warranty-disclaimer`, extra `ijg`, and `free-unknown` entries.

## Root Cause Analysis

### Python Reference Behavior

Python correctly returns a single `ijg` match:
- **Match**: `ijg` (lines 12-96, score 99.56%, matcher `3-seq`)
- **Rule**: `ijg.LICENSE` (full license text)
- **Coverage**: 99.56%

The ijg.LICENSE is a large rule (112 lines) that contains the complete IJG license text including:
- Warranty disclaimer language ("NO WARRANTY")
- "freely distributable" language
- Multiple embedded references

### Why Python Filters These Correctly

Python's `filter_contained_matches()` uses `qspan` containment:
```python
def qcontains(self, other):
    return other.qspan in self.qspan
```

Where `qspan` is a `Span` object backed by `intbitset`. The `in` operator checks if `other.qspan._set` is a subset of `self.qspan._set`.

**Key insight**: Python's seq matches create a `Span(range(qpos, qspan_end))` for each match block:
```python
# match_seq.py:116-17
qspan = Span(range(qpos, qspan_end))
ispan = Span(range(ipos, ipos + mlen))
```

When multiple blocks are merged (via `combine()`), the qspan becomes a union:
```python
# match.py:678
qspan=Span(self.qspan | other.qspan),
```

### Rust Implementation Gap

**Critical finding**: Rust's `seq_match.rs` creates matches with `qspan_positions: None`:

```rust
// seq_match.rs:787, 933
qspan_positions: None,
ispan_positions: None,
```

This means seq matches rely on `start_token..end_token` range for qspan calculations.

### The Containment Check Flow

When `filter_contained_matches()` runs:

1. **Matches are sorted by**: `qstart(), -hilen, -matched_length, matcher_order`
2. **For each pair, `qcontains()` checks containment**:
   - If both have `qspan_positions: Some(...)`: Set-based containment
   - If both have `qspan_positions: None`: Range-based containment (`start_token <= other.start_token && end_token >= other.end_token`)

### Possible Issues

1. **Merged seq matches may not have correct token bounds**: After `combine_matches()`, the `start_token`/`end_token` may not correctly represent the union of all matched positions

2. **The ijg.LICENSE match may be split into multiple blocks**: If seq matching creates multiple blocks for ijg.LICENSE, and they get merged, but the merged match doesn't properly cover all contained matches

3. **Token position alignment**: The absolute vs relative token positions may be misaligned between matches

## Investigation Steps

### Step 1: Verify qspan_bounds for ijg match

Add debug output to see the actual token bounds for the ijg.LICENSE match vs contained matches:

```rust
// In filter_contained_matches or a debug test
for m in &matches {
    eprintln!(
        "  {} (rid={}, coverage={:.1}%, start_token={}, end_token={}, qspan_positions={:?})",
        m.license_expression,
        m.rid,
        m.match_coverage,
        m.start_token,
        m.end_token,
        m.qspan_positions.as_ref().map(|p| p.len())
    );
}
```

### Step 2: Check merge behavior for seq matches

If ijg.LICENSE has multiple matching blocks, they should be merged by `merge_overlapping_matches()`. Verify that:
- The merged match has correct `start_token` and `end_token`
- The merged match has `qspan_positions: Some(...)` after merge (via `combine_matches()`)

### Step 3: Trace containment check

Add tracing to `qcontains()` to see why it returns `false`:

```rust
pub fn qcontains(&self, other: &LicenseMatch) -> bool {
    // Debug output
    if self.license_expression.contains("ijg") || other.license_expression.contains("ijg") {
        eprintln!(
            "qcontains: self={} ({}-{}) other={} ({}-{})",
            self.license_expression, self.start_token, self.end_token,
            other.license_expression, other.start_token, other.end_token
        );
    }
    // ... existing logic
}
```

## Implementation Plan

### Phase 1: Add Debug Test

Create a focused test to investigate the issue:

**File**: `src/license_detection/ijg_investigation_test.rs`

```rust
#[cfg(test)]
mod tests {
    use crate::license_detection::{LicenseDetectionEngine, detect_licenses_from_string};

    #[test]
    fn test_ijg_extra_detections_debug() {
        let content = include_str!("../testdata/license-golden/datadriven/lic4/ijg.txt");
        let engine = LicenseDetectionEngine::from_reference().expect("Failed to create engine");
        
        let detections = engine.detect(content, false).expect("Detection failed");
        
        // Debug: Print all matches before filtering
        // ...
        
        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].license_expression, "ijg");
    }
}
```

### Phase 2: Identify Root Cause

Based on debug output, determine which of these is the issue:

**Hypothesis A**: Token bounds mismatch
- The ijg match has wrong `start_token`/`end_token`
- Fix: Ensure `combine_matches()` correctly unions token ranges

**Hypothesis B**: Match ordering issue
- Smaller matches are processed before the large ijg match
- Fix: Verify sorting in `filter_contained_matches()` puts larger matches first

**Hypothesis C**: qspan_positions not populated after merge
- Merged matches should have `qspan_positions: Some(...)` but don't
- Fix: Verify `combine_matches()` always sets `qspan_positions`

### Phase 3: Implement Fix

Based on root cause, implement the appropriate fix:

**If Hypothesis A (bounds mismatch)**:
- Check `combine_matches()` in `match_refine.rs:155-190`
- Ensure `start_token` and `end_token` correctly reflect the union

**If Hypothesis B (ordering)**:
- Check sorting in `filter_contained_matches()` at line 373-379
- Ensure larger matches (by hilen/matched_length) sort before smaller ones

**If Hypothesis C (qspan_positions)**:
- Check that `combine_matches()` always sets `qspan_positions`
- Current code at line 180: `merged.qspan_positions = Some(qspan_vec);`

### Phase 4: Verification Tests

Add tests to prevent regression:

1. **Golden test**: Verify ijg.txt produces `["ijg"]` only
2. **Unit test**: Test `qcontains()` with realistic token ranges
3. **Integration test**: Test that large composite licenses filter contained matches

## Files to Investigate

| File | Purpose |
|------|---------|
| `src/license_detection/seq_match.rs:720-950` | How seq matches are created |
| `src/license_detection/match_refine.rs:155-190` | `combine_matches()` implementation |
| `src/license_detection/match_refine.rs:343-419` | `filter_contained_matches()` implementation |
| `src/license_detection/models.rs:532-558` | `qcontains()` implementation |
| `src/license_detection/models.rs:407-413` | `qstart()` implementation |

## Expected Outcome

After fix:
- `ijg.txt` returns single `ijg` detection
- Other composite licenses (like BSD with warranty text) also filter contained matches correctly
- Golden test suite passes

## Related Issues

- PLAN-062: Extra detection investigation (GFDL case)
- BUG-009: qcontains uses range containment instead of set containment
- CDDL investigation: Similar issues with `qspan_positions` handling
