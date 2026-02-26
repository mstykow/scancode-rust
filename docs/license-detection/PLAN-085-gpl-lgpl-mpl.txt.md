# PLAN-085: gpl-2.0-plus_and_lgpl-2.1-plus_and_mpl-1.1.txt Investigation

## Status: IMPLEMENTATION PLAN

## Problem Statement

**File**: `testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_lgpl-2.1-plus_and_mpl-1.1.txt`

| Expected (Python) | Actual (Rust) |
|-------------------|---------------|
| `["mpl-1.1 OR gpl-2.0-plus OR lgpl-2.1-plus", "mpl-1.1 OR gpl-2.0-plus OR lgpl-2.1-plus"]` (2) | `["mpl-1.1 OR gpl-2.0-plus OR lgpl-2.1-plus"]` (1) |

**Issue**: Missing one detection with same expression.

## Root Cause Analysis

### File Structure

The file contains a tri-license header (MPL 1.1/GPL 2.0/LGPL 2.1) with two distinct sections:

1. **Lines 2-13**: MPL 1.1 license text ("The contents of this file are subject to the Mozilla Public License Version 1.1...")
2. **Lines 14-24**: Copyright notices and contributor info
3. **Lines 25-37**: Alternative licensing text ("Alternatively, the contents of this file may be used under the terms of either the GPL or LGPL...")

### Python Behavior

Python produces **2 separate detections** with different rules:

| Detection | Lines | Rule | Matcher | Score | Coverage |
|-----------|-------|------|---------|-------|----------|
| 1 | 2-13 | `mpl-1.1_or_gpl-2.0-plus_or_lgpl-2.1-plus_1.RULE` | 2-aho | 100.0 | 100.0% |
| 2 | 25-37 | `mpl-1.1_or_gpl-2.0-plus_or_lgpl-2.1-plus_6.RULE` | 2-aho | 100.0 | 100.0% |

### Rust Behavior

Rust produces **1 detection**:

| Detection | Lines | Rule | Matcher | Score | Coverage |
|-----------|-------|------|---------|-------|----------|
| 1 | 2-37 | `mpl-1.1_or_gpl-2.0-plus_or_lgpl-2.1-plus_18.RULE` | 3-seq | 92.0 | 91.8% |

### Key Differences

1. **Matcher Type**: Python uses `2-aho` (Aho-Corasick exact match), Rust uses `3-seq` (sequence match)
2. **Rule Selection**: Python matches narrower rules (1 and 6), Rust matches broader rule (18)
3. **Rule 18 is a template**: Contains placeholders like `The Original Code is ` that match any text

### Root Cause Hypothesis

The issue is in how Rust's match filtering handles overlapping matches from different rules:

1. Rust's Aho-Corasick matcher likely finds rules 1 and 6 (same as Python)
2. Rust's sequence matcher finds rule 18 (broader, template-based)
3. During `filter_overlapping_matches` or `filter_contained_matches`:
   - Python keeps rules 1 and 6 because they are non-overlapping (separate locations)
   - Rust prefers rule 18 because it covers more content with a higher score after merging

**The critical question**: Does Python even produce rule 18 as a candidate match? If not, why?

---

## Investigation Steps

### Step 1: Trace Python's Match Generation

Run Python with debug tracing to see all candidate matches:

```bash
cd reference/scancode-toolkit
TRACE=1 TRACE_FILTER_OVERLAPPING=1 TRACE_FILTER_CONTAINED=1 TRACE_REFINE=1 \
  venv/bin/scancode -cl --json - ../../testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_lgpl-2.1-plus_and_mpl-1.1.txt 2>&1 | head -500
```

Questions to answer:
1. Does Python generate rule 18 as a match?
2. If yes, what filter removes it?
3. If no, why doesn't Python's sequence matcher produce it?

### Step 2: Trace Rust's Match Generation

Create a debug test to trace Rust's match generation:

```rust
// In src/license_detection/missing_detection_investigation_test.rs

#[test]
fn test_plan_085_trace_matches() {
    let path = "testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_lgpl-2.1-plus_and_mpl-1.1.txt";
    let content = std::fs::read_to_string(path).unwrap();

    let engine = LicenseDetectionEngine::new(None).unwrap();
    let index = engine.index();
    let query = Query::new(&content, index);

    // Get ALL matches (before refinement)
    let all_matches = query.run();
    eprintln!("Total matches before refinement: {}", all_matches.len());

    // Filter by rules of interest
    for m in &all_matches {
        if m.rule_identifier.contains("mpl-1.1_or_gpl-2.0-plus_or_lgpl-2.1-plus") {
            eprintln!(
                "Rule: {}, lines: {}-{}, matcher: {}, score: {:.1}, coverage: {:.1}%",
                m.rule_identifier, m.start_line, m.end_line, m.matcher, m.score, m.match_coverage
            );
        }
    }

    // Run refinement
    let refined = refine_matches(index, all_matches.clone(), &query);
    eprintln!("\nAfter refinement: {} matches", refined.len());

    for m in &refined {
        if m.rule_identifier.contains("mpl-1.1_or_gpl-2.0-plus_or_lgpl-2.1-plus") {
            eprintln!(
                "Rule: {}, lines: {}-{}, matcher: {}, score: {:.1}",
                m.rule_identifier, m.start_line, m.end_line, m.matcher, m.score
            );
        }
    }
}
```

### Step 3: Compare `filter_contained_matches` Behavior

The key difference may be in how containment is determined:

**Python's `filter_contained_matches`** (match.py:1075-1184):
- Sorts by: `(qspan.start, -hilen(), -len(), matcher_order)`
- Breaks inner loop when `next_match.qend > current_match.qend`
- Only uses `qcontains()` (query span containment)

**Rust's `filter_contained_matches`** (match_refine.rs:363-418):
- Sorts by: `(qstart(), -hilen, -matched_length, matcher_order)`
- Breaks when `next.end_token > current.end_token`
- Only uses `qcontains()` (same as Python)

**Key check**: Verify that Rust's sorting produces the same order as Python.

### Step 4: Compare `filter_overlapping_matches` Behavior

**Python's `filter_overlapping_matches`** (match.py:1187-1523):
- Sorts by: `(qspan.start, -hilen(), -len(), matcher_order)`
- Multiple overlap thresholds: SMALL (0.10), MEDIUM (0.40), LARGE (0.70), EXTRA_LARGE (0.90)
- Uses `licensing_contains()` for same-expression matches
- Has special handling for `surround()` relationships

**Rust's `filter_overlapping_matches`** (match_refine.rs:549-796):
- Same thresholds
- Additional logic for `candidate_resemblance` and `candidate_containment`
- May differ in how non-overlapping matches are handled

### Step 5: Check `merge_matches` Behavior

**Python's `merge_matches`** (match.py:869-1068):
- Groups matches by rule identifier FIRST
- Only merges matches from the SAME rule
- Matches from different rules are never merged together

**Rust's `merge_overlapping_matches`** (match_refine.rs:196-339):
- Same grouping by rule identifier
- Same logic for merging

**Key insight**: Rule 18 has template placeholders that may cause it to match more content, leading to a longer match that "absorbs" rules 1 and 6 during filtering.

---

## Implementation Plan

### Phase 1: Root Cause Identification

1. **Create debug test** (`test_plan_085_trace_matches`) to trace all matches
2. **Compare match generation**:
   - Does Rust produce rules 1, 6, and 18 as candidates?
   - What are the scores and positions?
3. **Trace refinement pipeline** step by step:
   - Before `merge_overlapping_matches`
   - After `merge_overlapping_matches`
   - After `filter_contained_matches`
   - After `filter_overlapping_matches`
   - After `restore_non_overlapping`

### Phase 2: Fix Implementation

Based on investigation results, one of these fixes:

#### Option A: Fix `filter_overlapping_matches` preference logic

If Python filters out rule 18 in favor of rules 1 and 6:
- Identify the exact condition that causes Python to prefer narrower matches
- Implement equivalent logic in Rust

#### Option B: Fix match scoring/comparison

If Python scores rule 18 lower than rules 1 and 6:
- Compare scoring logic between Python and Rust
- Ensure Rust's scoring produces same preferences

#### Option C: Fix non-overlapping match preservation

If the issue is in how non-overlapping matches are preserved:
- Review `restore_non_overlapping` logic
- Ensure matches at different locations are kept

### Phase 3: Test Coverage

1. **Unit test**: Create test for exact scenario with mock matches
2. **Integration test**: Add to golden test suite
3. **Regression test**: Ensure fix doesn't break other cases

---

## Specific Code Areas to Review

### 1. `filter_contained_matches` sorting order

**File**: `src/license_detection/match_refine.rs:373-379`

```rust
matches.sort_by(|a, b| {
    a.qstart()
        .cmp(&b.qstart())
        .then_with(|| b.hilen.cmp(&a.hilen))  // Higher hilen first
        .then_with(|| b.matched_length.cmp(&a.matched_length))
        .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
});
```

Verify this matches Python's: `(m.qspan.start, -m.hilen(), -m.len(), m.matcher_order)`

### 2. Containment check with non-overlapping matches

**File**: `src/license_detection/match_refine.rs:388-390`

```rust
if next.end_token > current.end_token {
    break;
}
```

This breaks the inner loop when next extends beyond current. For non-overlapping matches at different locations, this should preserve both.

### 3. `filter_overlapping_matches` same-expression handling

**File**: `src/license_detection/match_refine.rs:618-638`

```rust
let different_licenses =
    matches[i].license_expression != matches[j].license_expression;

let current_wins_on_candidate = { ... };
```

When expressions are the same (`different_licenses = false`), the filtering behavior may differ.

### 4. Rule 18 template behavior

**File**: `reference/scancode-toolkit/src/licensedcode/data/rules/mpl-1.1_or_gpl-2.0-plus_or_lgpl-2.1-plus_18.RULE`

Rule 18 contains:
```yaml
ignorable_copyrights:
    - Portions created by the Initial Developer are Copyright the Initial Developer
ignorable_holders:
    - the Initial Developer
```

This template may cause different matching behavior.

---

## Expected Outcome

After fix:
1. Rust should produce 2 detections (same as Python)
2. Detection 1: Lines 2-13, rule 1, matcher "2-aho"
3. Detection 2: Lines 25-37, rule 6, matcher "2-aho"
4. Rule 18 may or may not be produced, but should not supersede rules 1 and 6

---

## Related Files

- `src/license_detection/match_refine.rs` - Main filtering logic
- `src/license_detection/mod.rs` - Detection engine
- `reference/scancode-toolkit/src/licensedcode/match.py` - Python reference
- `reference/scancode-toolkit/src/licensedcode/data/rules/mpl-1.1_or_gpl-2.0-plus_or_lgpl-2.1-plus_*.RULE` - Rule definitions

## Related Documentation

- `docs/license_detection_comparison_report.md` - Comparison of Python vs Rust implementations
- `docs/license-detection/architecture/04-refinement.md` - Refinement architecture
