# PLAN-016: unknown/scea.txt

## Status: INVESTIGATION COMPLETE - AWAITING FIX

## Test File
`testdata/license-golden/datadriven/unknown/scea.txt`

## Issue
**Expected:** `["scea-1.0", "unknown-license-reference", "scea-1.0", "unknown", "unknown"]`
**Actual:** `["scea-1.0", "unknown-license-reference", "scea-1.0", "unknown"]`

## Differences
- **Missing one `unknown` match at the end**
- Python has 5 matches, Rust has 4 matches

## Python Reference Output (Confirmed 2026-02-28)

Run with: `scancode --license --unknown-licenses <file>`

```
Detection 1: scea-1.0
  Match: scea-1.0 | lines 1-1 | rule=scea-1.0_4.RULE | matcher=2-aho

Detection 2: scea-1.0 AND unknown
  Match: scea-1.0 | lines 7-7 | rule=scea-1.0_4.RULE | matcher=2-aho
  Match: unknown | lines 7-22 | matcher=6-unknown
  Match: unknown | lines 22-31 | matcher=6-unknown
```

Key observation: Python creates **TWO separate unknown matches**:
- `unknown` at lines 7-22
- `unknown` at lines 22-31

These are **adjacent** (line 22 is both end of match 3 and start of match 4).

## Rust Debug Output (Confirmed 2026-02-28)

```
=== RUST DETECTIONS ===
Number of detections: 3

Detection 1:
  license_expression: Some("scea-1.0")
  Number of matches: 1
    Match 1:
      license_expression: scea-1.0
      matcher: 2-aho
      lines: 1-1

Detection 2:
  license_expression: Some("unknown-license-reference")
  Number of matches: 1
    Match 1:
      license_expression: unknown-license-reference
      matcher: 2-aho
      lines: 1-1

Detection 3:
  license_expression: Some("scea-1.0 AND unknown")
  Number of matches: 2
    Match 1:
      license_expression: scea-1.0
      matcher: 2-aho
      lines: 7-7
    Match 2:
      license_expression: unknown
      matcher: 5-undetected
      lines: 7-31
```

Rust creates **ONE unknown match** spanning lines 7-31.

---

## Refined Root Cause Analysis

### Key Discovery

The splitting into multiple unknown matches happens in Python's `index.py:1095`:

```python
unmatched_qspan = original_qspan.difference(good_qspan)
for unspan in unmatched_qspan.subspans():
    unquery_run = query.QueryRun(query=qry, start=unspan.start, end=unspan.end)
    unknown_match = match_unknown.match_unknowns(...)
```

**Critical:** `subspans()` splits based on gaps in the unmatched positions. These gaps are caused by known matches' `qspan` (matched token positions only, not the full region).

### Why Two Unknown Matches in Python?

For Python to create two unknown matches at lines 7-22 and 22-31, there must be a gap in `unmatched_qspan` caused by a known match's qspan covering some positions between them.

**Hypothesis:** There's likely a match (possibly filtered during `refine_matches`) whose `qspan` creates this gap. This could be:
1. An aho match for a rule that matches part of line 22
2. A seq match that was found but later filtered out
3. Some match that exists in Python's intermediate state but not in final output

### Rust vs Python Difference

| Aspect | Python | Rust |
|--------|--------|------|
| Coverage computation | Uses `qspan` (matched positions only) | Uses `start_token..end_token` (full region) |
| Splitting mechanism | `subspans()` on sparse position set | Finds contiguous uncovered regions |
| Gap detection | Automatic via Span's difference/subspans | Manual region finding |

### The Critical Bug

Rust's `compute_covered_positions()` uses `start_token..end_token` which is the **qregion** (full span from start to end), NOT the **qspan** (matched positions only).

In Python:
```python
good_qspans = (mtch.qspan for mtch in good_matches)  # qspan, not qregion!
good_qspan = Span().union(*good_qspans)
unmatched_qspan = original_qspan.difference(good_qspan)
```

This means Python correctly excludes ONLY the matched positions, while Rust excludes the entire region from start_token to end_token.

---

## Proposed Fix

The `LicenseMatch` struct already has `qspan_positions: Option<Vec<usize>>` field (line 323 in models.rs).

### Implementation Steps

1. **Populate `qspan_positions` for aho matches**:
   - In `aho_match.rs`, set `qspan_positions` to the exact token positions matched
   - Currently it's set to `None` for all aho matches

2. **Use `qspan_positions` in `compute_covered_positions()`**:
   - In `unknown_match.rs:161-174`, check if `qspan_positions` is Some
   - If Some, use those positions instead of `start_token..end_token`
   - This matches Python's behavior of using only matched positions

### Code Change in `unknown_match.rs`

```rust
fn compute_covered_positions(
    _query: &Query,
    known_matches: &[LicenseMatch],
) -> std::collections::HashSet<usize> {
    let mut covered = std::collections::HashSet::new();

    for m in known_matches {
        if let Some(positions) = &m.qspan_positions {
            // Use exact matched positions (Python's qspan behavior)
            for pos in positions {
                covered.insert(*pos);
            }
        } else {
            // Fallback to full range for matches without qspan_positions
            for pos in m.start_token..m.end_token {
                covered.insert(pos);
            }
        }
    }

    covered
}
```

---

## Investigation Notes

**Root Cause Confirmed (2026-02-28):**

The Python output shows the gap is created by a known match at line 22 (the `license-intro_59.RULE` which matches "Contributions"). This creates a gap in the unknown regions that causes Python to split into two matches.

However, the deeper issue is that Python's unknown detection uses `qspan` (exact matched token positions) while Rust uses `start_token..end_token` (full range). The fix is to populate and use `qspan_positions`.

---

## Estimated Effort

**Total: 2-4 hours**

| Task | Time |
|------|------|
| Populate qspan_positions in aho_match.rs | 1-2 hours |
| Update compute_covered_positions() in unknown_match.rs | 30 min |
| Add tests and verify | 1 hour |

---

## Risk Analysis
- **Medium risk**: Fix depends on understanding the exact Python behavior
- **Functional impact**: Users will see more accurate unknown match boundaries
- **Priority**: Medium - affects feature parity with Python

## Related Files
- `src/license_detection/unknown_match.rs` - Unknown match detection
- `src/license_detection/match_refine.rs` - `split_weak_matches()`
- `reference/scancode-toolkit/src/licensedcode/index.py` - Python reference
- `reference/scancode-toolkit/src/licensedcode/spans.py` - Python Span implementation
