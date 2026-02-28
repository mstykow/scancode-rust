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

## Python Reference Output

```
Total matches: 5
0: scea-1.0 | lines 1-1 | rule=scea-1.0_4.RULE | matcher=2-aho
1: unknown-license-reference | lines 1-1 | rule=unknown-license-reference_332.RULE | matcher=2-aho
2: scea-1.0 | lines 7-7 | rule=scea-1.0_4.RULE | matcher=2-aho
3: unknown | lines 7-22 | rule=license-detection-unknown-* | matcher=6-unknown
4: unknown | lines 22-31 | rule=license-detection-unknown-* | matcher=6-unknown
```

Key observation: Python creates **TWO separate unknown matches**:
- `unknown` at lines 7-22
- `unknown` at lines 22-31

These are **adjacent** (line 22 is both end of match 3 and start of match 4).

## Rust Debug Output

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

### Option 1: Track qspan positions in LicenseMatch (Comprehensive)

Add `qspan_positions: Vec<usize>` to `LicenseMatch` to track exactly which token positions were matched. Use this for computing covered positions in unknown detection.

**Pros:**
- Accurate parity with Python
- Enables future features (highlighting, etc.)

**Cons:**
- Requires changes to all matchers
- More memory per match

### Option 2: Re-compute qspan from matched_token_positions (If available)

If `matched_token_positions` is already populated for matches, use it to compute coverage.

### Option 3: Debug Python to identify the actual gap-causing match

Run Python with tracing to identify which match's qspan creates the gap between lines 7-22 and 22-31. Then ensure Rust has equivalent behavior.

**Recommended approach:** Option 3 first to confirm hypothesis, then implement Option 1 or 2.

---

## Specific Investigation Needed

1. **Run Python with TRACE enabled** to see:
   - What matches exist before `split_weak_matches`
   - What `good_qspan` looks like
   - What `unmatched_qspan.subspans()` returns
   - Which match's qspan causes the split

2. **Compare Rust's intermediate state** to Python:
   - What matches does Rust have after initial matching?
   - What positions are marked as covered?

3. **Check for hidden matches** in Python that might create the gap:
   - Run with `TRACE=true` in Python
   - Look for matches that are filtered during `refine_matches`

---

## Debugging Commands

### Python Debug
```bash
cd reference/scancode-toolkit
python -c "
import os
os.environ['LICENSEDCODE_TRACE'] = 'true'
from licensedcode.index import LicenseIndex
from licensedcode.query import Query

idx = LicenseIndex()
text = open('../../testdata/license-golden/datadriven/unknown/scea.txt').read()
qry = Query(idx, text, location='test')
matches = idx.match(qry)
"
```

### Rust Debug
Add debug output to `unknown_match()` to print:
- Covered positions
- Unmatched regions
- Ngram match positions

---

## Estimated Effort

**Total: 4-8 hours** (depending on investigation findings)

| Task | Time |
|------|------|
| Debug Python to identify gap-causing match | 2-3 hours |
| Implement fix based on findings | 2-4 hours |
| Update tests | 1 hour |

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
