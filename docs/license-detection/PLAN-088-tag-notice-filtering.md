# PLAN-088: Tag Matches Filtered When Contained by Notice Matches

## Status: INVESTIGATION COMPLETE - FIX REQUIRED

## Problem Statement

**File**: `testdata/license-golden/datadriven/lic4/kde_licenses_test.txt`

| Expected (Python) | Actual (Rust) |
|-------------------|---------------|
| 15 matches | 14 matches |

**Critical difference at lines 92-109**:

```
92:  LGPL 2.1
93:  
94:  LGPL-2.1
95:  
96:  Copyright <year>  <name of author> <e-mail>
...  (LGPL notice text)
109: License along with this library.  If not, see <http://www.gnu.org/licenses/>.
```

### Expected Behavior (Python)

Python produces **3 separate matches**:
1. `lgpl-2.1` at line 92 - tag match ("LGPL 2.1")
2. `lgpl-2.1` at line 94 - tag match ("LGPL-2.1")
3. `lgpl-2.1-plus` at lines 96-109 - notice match

### Actual Behavior (Rust)

Rust produces **1 match**:
- `lgpl-2.1-plus` at lines 92-109 - covers everything

The tag matches for `lgpl-2.1` are being filtered as "contained" by the larger `lgpl-2.1-plus` match.

## Root Cause Analysis

### Key Rules Involved

1. **`lgpl-2.1-plus_114.RULE`** - Notice rule
   - Expression: `lgpl-2.1-plus`
   - `is_license_notice: yes`
   - `minimum_coverage: 50`
   - Text includes BOTH the tags ("LGPL 2.1", "LGPL-2.1") AND the notice text

2. **Tag rules for `lgpl-2.1`** (e.g., `lgpl-2.1_115.RULE`, etc.)
   - Expression: `lgpl-2.1`
   - `is_license_tag: yes`
   - Match short identifiers like "License: LGPL 2.1"

### Why This Happens

The `lgpl-2.1-plus_114.RULE` has `minimum_coverage: 50`, meaning it only needs to match 50% of its rule text. The rule text includes the tag lines at the beginning, so:

1. **Sequence matching** finds a match for `lgpl-2.1-plus` starting at line 92
2. The match spans lines 92-109 (including both tags AND notice)
3. **Aho matching** also finds separate tag matches at lines 92 and 94
4. **`filter_contained_matches()`** discards the tag matches because they are spatially contained within the larger `lgpl-2.1-plus` match

### Python's Different Behavior

Python also has the same filtering logic, but Python produces 3 matches. The key insight from test `test_filter_contained_matches_matches_does_filter_matches_with_contained_spans_if_licenses_are_different`:

```python
def test_filter_contained_matches_matches_does_filter_matches_with_contained_spans_if_licenses_are_different(self):
    r1 = create_rule_from_text_and_expression(license_expression='apache-2.0')
    m1 = LicenseMatch(rule=r1, qspan=Span(0, 2), ispan=Span(0, 2))

    r2 = create_rule_from_text_and_expression(license_expression='apache-2.0')
    m2 = LicenseMatch(rule=r2, qspan=Span(1, 6), ispan=Span(1, 6))

    r3 = create_rule_from_text_and_expression(license_expression='apache-1.1')  # DIFFERENT LICENSE
    m3 = LicenseMatch(rule=r3, qspan=Span(0, 2), ispan=Span(0, 2))

    matches, discarded = filter_contained_matches([m1, m2, m3])
    assert matches == [m1, m2]  # m3 is kept because license differs from m1
```

**Key insight**: When a contained match has a **different license expression** than the containing match, Python keeps both.

In our case:
- Tag matches: `lgpl-2.1` (different expression)
- Notice match: `lgpl-2.1-plus` (different expression)
- Therefore, both should be kept!

### The Bug in Rust

The Rust `filter_contained_matches()` implementation at `src/license_detection/match_refine.rs:362-419`:

```rust
if current.qcontains(&next) {
    discarded.push(matches.remove(j));
    continue;
}
```

This unconditionally discards contained matches without checking if the license expressions differ.

## Implementation Plan

### Phase 1: Verify the Hypothesis

**Task 1.1**: Add debug output to confirm Rust is generating tag matches

Create a test to verify that tag matches ARE being generated initially but are being filtered:

```rust
// In src/license_detection/missing_detection_investigation_test.rs or new file
#[test]
fn test_lgpl_tag_matches_generated_before_filtering() {
    // Run detection with detailed logging
    // Verify that lgpl-2.1 tag matches appear BEFORE filter_contained_matches
    // Verify that they are DISCARDED by filter_contained_matches
}
```

**Files to modify**:
- `src/license_detection/match_refine.rs` - Add temporary debug logging

**Task 1.2**: Compare Python and Rust filtering side-by-side

Run both implementations with trace logging enabled and compare the filtering decisions.

### Phase 2: Implement the Fix

**Task 2.1**: Modify `filter_contained_matches()` to preserve matches with different expressions

Location: `src/license_detection/match_refine.rs:362-419`

The fix: When a match is contained by another, check if their license expressions differ. If they differ, KEEP both matches.

```rust
// Current code (line 403-405):
if current.qcontains(&next) {
    discarded.push(matches.remove(j));
    continue;
}

// Fixed code:
if current.qcontains(&next) {
    // Preserve matches with different license expressions
    if !current.license_expression.eq_ignore_ascii_case(&next.license_expression) {
        j += 1;
        continue;  // Keep both matches
    }
    discarded.push(matches.remove(j));
    continue;
}
```

Apply the same logic to the reverse case (lines 407-411):

```rust
if next.qcontains(&current) {
    // Preserve matches with different license expressions
    if !current.license_expression.eq_ignore_ascii_case(&next.license_expression) {
        j += 1;
        continue;  // Keep both matches
    }
    discarded.push(matches.remove(i));
    i = i.saturating_sub(1);
    break;
}
```

**Task 2.2**: Add unit tests for the new behavior

Add tests to `src/license_detection/match_refine.rs` in the `#[cfg(test)]` module:

```rust
#[test]
fn test_filter_contained_preserves_different_license_expressions() {
    // Contained match with different expression should be kept
    let mut m1 = create_test_match_with_tokens("rule1", 0, 10, 10);
    m1.license_expression = "lgpl-2.1-plus".to_string();
    
    let mut m2 = create_test_match_with_tokens("rule2", 2, 4, 2);
    m2.license_expression = "lgpl-2.1".to_string();  // Different expression
    
    let (kept, discarded) = filter_contained_matches(&[m1.clone(), m2.clone()]);
    
    assert_eq!(kept.len(), 2, "Both matches should be kept with different expressions");
    assert!(discarded.is_empty());
}

#[test]
fn test_filter_contained_filters_same_license_expressions() {
    // Contained match with same expression should be filtered
    let m1 = create_test_match_with_tokens("rule1", 0, 10, 10);
    let m2 = create_test_match_with_tokens("rule2", 2, 4, 2);
    // Both have default "mit" expression
    
    let (kept, discarded) = filter_contained_matches(&[m1, m2]);
    
    assert_eq!(kept.len(), 1, "Only larger match should be kept with same expression");
    assert_eq!(discarded.len(), 1);
}
```

### Phase 3: Verify the Fix

**Task 3.1**: Run the golden test

```bash
cargo test license_detection_golden --lib -- --test-threads=1 2>&1 | grep -A5 "kde_licenses"
```

**Task 3.2**: Run all license detection tests

```bash
cargo test --lib license_detection
```

**Task 3.3**: Run the full test suite

```bash
cargo test
```

### Phase 4: Consider Edge Cases

**Task 4.1**: Check for expression subsumption

Python may also consider whether one expression "subsumes" another (e.g., `lgpl-2.1-plus` vs `lgpl-2.1`). The `lgpl-2.1-plus` expression technically includes `lgpl-2.1` as a possibility.

**Investigation needed**: Check Python's `is_equivalent` or `subsumes` logic:

```bash
grep -r "subsumes\|is_equivalent" reference/scancode-toolkit/src/licensedcode/
```

If subsumption is considered, the fix may need to be more nuanced:

```rust
// Possible enhancement: consider expression relationships
fn expressions_are_related(expr1: &str, expr2: &str) -> bool {
    // lgpl-2.1-plus subsumes lgpl-2.1
    // apache-2.0 is unrelated to mit
    // etc.
}
```

**Task 4.2**: Check for tag vs notice distinction

Should `is_license_tag` matches ALWAYS be preserved even when contained? This would be a simpler fix but may have unintended consequences.

From Python code at `match.py:1957-1960`:
```python
# Matches to license tag are special and can be scattered on a few
# extra lines.
if rule.is_license_tag:
    matched_len += 2
```

This shows Python gives special treatment to tag matches in some contexts.

**Task 4.3**: Test with other similar cases

Search for other test files that might have similar tag+notice patterns:

```bash
grep -r "LGPL.*2.1" testdata/license-golden/datadriven/ | head -20
```

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

**Low Risk**: The fix is localized to `filter_contained_matches()` and adds a simple condition check. The behavior matches Python's documented test cases.

**Potential Impact**: Other golden tests may see additional matches now that contained matches with different expressions are preserved. This is expected and correct behavior.

## Timeline

- Phase 1 (Verification): 1 hour
- Phase 2 (Implementation): 2 hours
- Phase 3 (Testing): 1 hour
- Phase 4 (Edge Cases): 2 hours
- **Total**: ~6 hours

## References

- Python test: `test_filter_contained_matches_matches_does_filter_matches_with_contained_spans_if_licenses_are_different`
- Python code: `reference/scancode-toolkit/src/licensedcode/match.py:1075-1184`
- Rust code: `src/license_detection/match_refine.rs:362-419`
- Related: PLAN-088-kde-licenses-test.md (initial analysis)
