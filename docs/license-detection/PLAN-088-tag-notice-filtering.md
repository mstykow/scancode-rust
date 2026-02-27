# PLAN-088: Tag Matches Filtered When Contained by Notice Matches

## Status: ROOT CAUSE IS MATCH GENERATION, NOT FILTERING

**Previous fix was WRONG**: The proposed fix (preserve different expressions in containment filtering) would break Python parity.

**Validation findings**:
- Python's `filter_contained_matches` does NOT check license expression
- Python's test `test_filter_contained_matches...if_licenses_are_different` shows matches ARE filtered when they have different licenses
- The real issue is that Rust and Python match DIFFERENT rules

**Root cause**:
- Python matches: `lgpl-2.1` (tag) + `lgpl-2.1` (tag) + `lgpl-2.1-plus` (notice) = 3 matches
- Rust matches: `lgpl-2.1-plus_114.RULE` (combined tag+notice) = 1 match

**Next investigation needed**:
- Why does Rust's sequence matcher prefer the combined rule?
- Check candidate scoring for tag rules vs combined rules
- May need to adjust seq matching to prefer narrower rules

## Problem Statement

**File**: `testdata/license-golden/datadriven/lic4/kde_licenses_test.txt`

| Expected (Python) | Actual (Rust) |
|-------------------|---------------|
| 15 matches | 14 matches |

**Critical difference at lines 88-109**:

```
Line 88: (empty)
Line 89: You should have received a copy of the GNU General Public License
Line 90: along with this program.  If not, see <http://www.gnu.org/licenses/>.
Line 91: (empty)
Line 92: LGPL 2.1
Line 93: (empty)
Line 94: LGPL-2.1
Line 95: (empty)
Line 96-109: LGPL notice text
```

### Expected Behavior (Python)

Python produces **3 separate matches** for this region:
1. `lgpl-2.1` at lines 90-92, rule=lgpl-2.1_72.RULE (matches URL in line 90)
2. `lgpl-2.1` at line 94, rule=lgpl-2.1_85.RULE (matches "lgpl 2.1")
3. `lgpl-2.1-plus` at lines 98-109, rule=lgpl-2.1-plus_36.RULE (matches notice text)

### Actual Behavior (Rust)

Rust produces **1 match** for this region:
- `lgpl-2.1-plus` at lines 92-109, rule=lgpl-2.1-plus_114.RULE (matches tags + notice combined)

## Root Cause Analysis

### Key Discovery: Different Rules Being Matched

The root cause is **NOT** filter_contained_matches behavior. The root cause is that **Rust and Python are matching different rules**:

| System | Rule Used | Text Coverage |
|--------|-----------|---------------|
| Python | `lgpl-2.1-plus_36.RULE` | Notice text only (lines 98-109) |
| Rust | `lgpl-2.1-plus_114.RULE` | Tags + Notice text (lines 92-109) |

### Rule File Comparison

**lgpl-2.1-plus_36.RULE** (Python uses this):
```yaml
license_expression: lgpl-2.1-plus
is_license_notice: yes
---
This library is free software; you can redistribute it and/or modify
it under the terms of the {{GNU Lesser General Public License as published
by the Free Software Foundation; either version 2.1 of the License, or
(at your option) any later version}}.
...
```

**lgpl-2.1-plus_114.RULE** (Rust uses this):
```yaml
license_expression: lgpl-2.1-plus
is_license_notice: yes
minimum_coverage: 50
---
LGPL 2.1

LGPL-2.1

This library is free software; you can redistribute it and/or
modify it under the terms of the {{GNU Lesser General Public
...
```

### Why Python Doesn't Match lgpl-2.1-plus_114.RULE

The `lgpl-2.1-plus_114.RULE` has `minimum_coverage: 50`, meaning it only needs 50% match coverage. However:

1. Python's sequence matcher may score this rule lower than the separate tag matches + notice match
2. Python may prefer the combination of:
   - Tag match (`lgpl-2.1_85.RULE` matches "lgpl 2.1")
   - URL reference match (`lgpl-2.1_72.RULE` matches URL)
   - Notice match (`lgpl-2.1-plus_36.RULE` matches notice)
3. After `filter_contained_matches()`, Python keeps these separate matches because they have different license expressions

### Why Rust Prefers lgpl-2.1-plus_114.RULE

Rust's sequence matcher is finding `lgpl-2.1-plus_114.RULE` as a better match because:
1. It matches both the tags AND the notice in one rule
2. The `minimum_coverage: 50` allows partial matching
3. Rust's scoring may favor this comprehensive rule over separate smaller matches

### Python's filter_contained_matches Test Analysis

From `test_filter_contained_matches_matches_does_filter_matches_with_contained_spans_if_licenses_are_different`:
```python
# m1: apache-2.0, Span(0, 2)
# m2: apache-2.0, Span(1, 6)  
# m3: apache-1.1, Span(0, 2) - DIFFERENT LICENSE
assert matches == [m1, m2]  # m3 IS kept because different license
```

**Key insight**: The test name is misleading. Python DOES keep matches with different license expressions even when spatially contained!

However, in our case, Python doesn't even generate the lgpl-2.1-plus_114.RULE match, so containment filtering isn't the issue.

## Implementation Plan

### Phase 1: Verify Rust Match Generation

**Task 1.1**: Create test to see what matches Rust generates before filtering

```rust
#[test]
fn test_kde_licenses_lgpl_region_matches() {
    // Run detection pipeline step-by-step
    // Print ALL matches before filter_contained_matches
    // Check if lgpl-2.1 tag matches are generated
    // Check if lgpl-2.1-plus_114.RULE match is generated
}
```

**Task 1.2**: Determine if the issue is:
- Rust not generating the tag matches at all, OR
- Rust generating lgpl-2.1-plus_114.RULE which swallows everything, OR
- filter_contained_matches filtering the tag matches

### Phase 2: Two Possible Fixes

#### Option A: Fix in filter_contained_matches (if tag matches are being filtered)

If Rust IS generating tag matches but they're being filtered, modify `filter_contained_matches()` at lines 403-411:

```rust
// Current code:
if current.qcontains(&next) {
    discarded.push(matches.remove(j));
    continue;
}

// Fixed code - preserve matches with different license expressions:
if current.qcontains(&next) {
    if current.license_expression != next.license_expression {
        j += 1;
        continue;  // Keep both matches
    }
    discarded.push(matches.remove(j));
    continue;
}
```

**Note**: This fix matches Python's behavior as shown in the test case.

#### Option B: Fix in sequence matching (if wrong rule is being matched)

If Rust is preferring `lgpl-2.1-plus_114.RULE` over separate tag + notice matches:

1. **Investigate scoring**: Check if Rust's seq matcher scores should prefer separate matches
2. **Rule prioritization**: Consider if `minimum_coverage: 50` rules should be deprioritized
3. **Match combination**: Consider if multiple smaller matches should be preferred over one comprehensive match

### Phase 3: Verification

**Task 3.1**: Run golden test
```bash
cargo test kde_licenses --lib -- --nocapture
```

**Task 3.2**: Run all license detection tests
```bash
cargo test --lib license_detection
```

### Phase 4: Edge Cases

**Task 4.1**: Check expression subsumption

The expressions `lgpl-2.1` and `lgpl-2.1-plus` are related:
- `lgpl-2.1-plus` includes `lgpl-2.1` as a possibility (version 2.1 or later)
- Should matches with subsumed expressions still be preserved?

Python test shows it preserves `apache-2.0` and `apache-1.1` (different licenses), so it should also preserve `lgpl-2.1` and `lgpl-2.1-plus` (different expressions).

## Files to Modify

| File | Changes |
|------|---------|
| `src/license_detection/match_refine.rs` | Modify `filter_contained_matches()` to preserve matches with different expressions |
| `src/license_detection/match_refine.rs` | Add unit tests for new behavior |

## Success Criteria

1. `kde_licenses_test.txt` produces 15 matches matching Python output
2. All existing tests pass
3. New unit tests verify the different-expression preservation behavior
4. No regressions in other golden tests

## Risk Assessment

**Low Risk**: The fix in `filter_contained_matches()` is a simple condition check. Python's test explicitly shows this behavior is expected.

**Potential Impact**: Other golden tests may see additional matches now that contained matches with different expressions are preserved. This is expected and correct behavior.

## Timeline

- Phase 1 (Verification): 1 hour
- Phase 2 (Implementation): 2 hours  
- Phase 3 (Testing): 1 hour
- Phase 4 (Edge Cases): 1 hour
- **Total**: ~5 hours

## References

- Python test: `test_filter_contained_matches_matches_does_filter_matches_with_contained_spans_if_licenses_are_different`
- Python code: `reference/scancode-toolkit/src/licensedcode/match.py:1075-1184`
- Rust code: `src/license_detection/match_refine.rs:362-419`
- Related: PLAN-088-kde-licenses-test.md (initial analysis)

## Appendix: Python Reference Output

```
lgpl-2.1: lines=90-92, rule=lgpl-2.1_72.RULE
lgpl-2.1: lines=94-94, rule=lgpl-2.1_85.RULE
lgpl-2.1-plus: lines=98-109, rule=lgpl-2.1-plus_36.RULE
```

Note: Python matches `lgpl-2.1_72.RULE` which matches a URL in the file (line 90 has `http://www.gnu.org/licenses/lgpl-2.1`), not the text "LGPL 2.1" on line 92.
