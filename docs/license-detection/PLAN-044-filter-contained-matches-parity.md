# PLAN-044: filter_contained_matches Parity

**Status**: ⚠️ IMPLEMENTATION CAUSES REGRESSION - NEEDS INVESTIGATION
**Impact**: Implementation causes -6 tests regression (3780 → 3774 passed)

## Implementation Attempt (2026-02-24)

Two changes were attempted:

1. Remove `licensing_contains_match()` from containment checks
2. Add `spans_equal()` for non-contiguous span comparison

Result:

- **Baseline**: 3780 passed, 583 failed
- **After PLAN-044**: 3774 passed, 589 failed (regression of 6 tests)

### Root Cause of Regression

**Change 1: Removing `licensing_contains_match()`**

- Python uses expression-based containment in `filter_overlapping_matches`, not `filter_contained_matches`
- Removing it from Rust causes different deduplication behavior
- Matches that should be deduplicated based on expression subsumption are now kept

**Change 2: Adding `spans_equal()`**

- Correct for non-contiguous spans but changes deduplication behavior
- Matches with same bounds but different actual positions are no longer considered equal
- This causes more matches to be kept when they should be deduplicated

### Investigation Needed

1. The `licensing_contains_match()` may be a **beneficial extension** over Python
   - Handles cases like `gpl-2.0 WITH exception` containing `gpl-2.0`
   - Should be kept but moved to correct location?

2. The `spans_equal()` fix is correct but affects golden tests
   - Need to analyze which tests fail and why
   - May indicate Python also has issues with non-contiguous spans

### Recommendation

1. **Keep `licensing_contains_match()`** - it's a beneficial extension
2. **Investigate `spans_equal()` failures** - determine if tests need updating or fix is wrong

---

## Summary

Investigation of the `filter_contained_matches` function comparing Python reference implementation with Rust implementation, identifying differences and creating a plan for 100% parity.

## Investigation Date

2026-02-24

## Python Reference Implementation

**Location**: `reference/scancode-toolkit/src/licensedcode/match.py` lines 1075-1184

**Signature**:

```python
def filter_contained_matches(
    matches,
    trace=TRACE_FILTER_CONTAINED,
    reason=DiscardReason.CONTAINED,
):
    """
    Return a filtered list of kept LicenseMatch matches and a list of
    discardable matches given a `matches` list of LicenseMatch by removing
    matches that are contained in larger matches.

    For instance a match entirely contained in another bigger match is removed.
    When more than one matched position matches the same license(s), only one
    match of this set is kept.
    """
```

## Rust Implementation

**Location**: `src/license_detection/match_refine.rs` lines 326-380

**Signature**:

```rust
fn filter_contained_matches(matches: &[LicenseMatch]) -> (Vec<LicenseMatch>, Vec<LicenseMatch>)
```

---

## Detailed Comparison

### 1. Sorting Order

**Python (lines 1097-1100)**:

```python
# sort on start, longer high, longer match, matcher type
sorter = lambda m: (m.qspan.start, -m.hilen(), -m.len(), m.matcher_order)
matches = sorted(matches, key=sorter)
```

**Rust (lines 334-340)**:

```rust
matches.sort_by(|a, b| {
    a.start_token
        .cmp(&b.start_token)
        .then_with(|| b.hilen.cmp(&a.hilen))
        .then_with(|| b.matched_length.cmp(&a.matched_length))
        .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
});
```

**Finding**: **IDENTICAL** - Both sort by:

1. `start_token` ascending (earlier matches first)
2. `hilen` descending (more high-value tokens preferred)
3. `matched_length` descending (longer matches preferred)
4. `matcher_order` ascending (better matchers preferred)

---

### 2. Early Exit Condition (No Overlap Possible)

**Python (lines 1126-1134)**:

```python
# BREAK/shortcircuit rather than continue since continuing looking
# next matches will yield no new findings. e.g. stop when no overlap
# is possible. Based on sorting order if no overlap is possible,
# then no future overlap will be possible with the current match.
# Note that touching and overlapping matches have a zero distance.
if next_match.qend > current_match.qend:
    if trace:
        logger_debug(
            '    ---> ###filter_contained_matches: matches have a distance: '
            'NO OVERLAP POSSIBLE -->',
            'qdist:', current_match.qdistance_to(next_match))

    j += 1
    break
```

**Rust (lines 349-351)**:

```rust
if next.end_token > current.end_token {
    break;
}
```

**Finding**: **DIFFERENT**

| Aspect | Python | Rust |
|--------|--------|------|
| Condition | `next_match.qend > current_match.qend` | `next.end_token > current.end_token` |
| Break behavior | `j += 1; break` | `break` |

**Problem**: Python increments `j` before breaking, but both then increment `i` (Python: `i += 1`, Rust: `i += 1`). This means:

- Python: `j` is now pointing to the next match after the non-overlapping one
- Rust: `j` stays at the same position (no increment before break)

However, since both reset `j = i + 1` at the start of the inner loop, this difference is **functionally equivalent** for the outer loop iteration.

**Verdict**: Functionally equivalent due to loop reset behavior.

---

### 3. Equal Spans Check

**Python (lines 1136-1156)**:

```python
# equals matched spans
if current_match.qspan == next_match.qspan:
    if current_match.coverage() >= next_match.coverage():
        if trace:
            logger_debug(
                '    ---> ###filter_contained_matches: '
                'next EQUALS current, '
                'removed next with lower or equal coverage', matches[j])

        discarded_append(matches_pop(j))
        continue
    else:
        if trace:
            logger_debug(
                '    ---> ###filter_contained_matches: '
                'next EQUALS current, '
                'removed current with lower coverage', matches[i])
        discarded_append(matches_pop(i))
        i -= 1
        break
```

**Rust (lines 353-362)**:

```rust
if current.start_token == next.start_token && current.end_token == next.end_token {
    if current.match_coverage >= next.match_coverage {
        discarded.push(matches.remove(j));
        continue;
    } else {
        discarded.push(matches.remove(i));
        i = i.saturating_sub(1);
        break;
    }
}
```

**Finding**: **DIFFERENT**

| Aspect | Python | Rust |
|--------|--------|------|
| Comparison | `current_match.qspan == next_match.qspan` (Span equality) | `start_token == next.start_token && current.end_token == next.end_token` |
| Handles non-contiguous | Yes (qspan can be discontinuous) | No (assumes contiguous spans) |

**Problem**: Python's `qspan` is a `Span` object that can represent **discontinuous** token positions (e.g., `Span(1, 5) | Span(10, 15)`). The Rust comparison only checks the bounds, not the actual positions.

**Example of difference**:

- Match A: tokens [1, 2, 10, 11] (start=1, end=12)
- Match B: tokens [1, 2, 3, 4] (start=1, end=5)
- Python: `qspan == qspan` → False (different token sets)
- Rust: `start == start && end > end` → Uses containment check instead

This is a **latent bug** in Rust that may cause incorrect filtering for non-contiguous matches.

---

### 4. Containment Check

**Python (lines 1158-1176)**:

```python
# remove contained matched spans
if current_match.qcontains(next_match):
    # ... discard next
    discarded_append(matches_pop(j))
    continue

# remove contained matches the other way
if next_match.qcontains(current_match):
    # ... discard current
    discarded_append(matches_pop(i))
    i -= 1
    break
```

**Rust (lines 364-372)**:

```rust
if current.qcontains(&next) || licensing_contains_match(&current, &next) {
    discarded.push(matches.remove(j));
    continue;
}
if next.qcontains(&current) || licensing_contains_match(&next, &current) {
    discarded.push(matches.remove(i));
    i = i.saturating_sub(1);
    break;
}
```

**Finding**: **DIFFERENT - Rust has EXTRA logic**

Rust adds `licensing_contains_match()` which checks if one license expression "contains" another (e.g., `gpl-2.0 WITH exception` contains `gpl-2.0`).

**This is NOT in Python's `filter_contained_matches`**.

Looking at Python code, expression-based containment is handled in **`filter_overlapping_matches`**, not `filter_contained_matches`.

**Python's `qcontains` (match.py line 444-448)**:

```python
def qcontains(self, other):
    """
    Return True if qspan contains other.qspan.
    """
    return other.qspan in self.qspan
```

This is **purely token-position based**, not expression-based.

**Rust's `qcontains` (models.rs lines 491-506)**:

```rust
pub fn qcontains(&self, other: &LicenseMatch) -> bool {
    if let (Some(self_positions), Some(other_positions)) =
        (&self.qspan_positions, &other.qspan_positions)
    {
        return other_positions.iter().all(|p| self_positions.contains(p));
    }

    if self.start_token == 0
        && self.end_token == 0
        && other.start_token == 0
        && other.end_token == 0
    {
        return self.start_line <= other.start_line && self.end_line >= other.end_line;
    }
    self.start_token <= other.start_token && self.end_token >= other.end_token
}
```

This correctly handles discontinuous spans via `qspan_positions`, but the fallback to bounds-only check is the same as the equal-spans issue above.

---

### 5. Expression-Based Containment

**Python**: Does NOT exist in `filter_contained_matches`. Expression subsumption is in `filter_overlapping_matches` (lines 1300-1451).

**Rust**: Added via `licensing_contains_match()` which calls `licensing_contains()` from `expression.rs`.

**This is an intentional Rust extension** documented in the function comment:

```rust
/// A match A is contained in match B if:
/// - A's qspan (token positions) is contained in B's qspan, OR
/// - B's license expression subsumes A's expression (e.g., "gpl-2.0 WITH exception" subsumes "gpl-2.0")
```

**Verdict**: This is a **feature difference**, not a parity issue. However, it may cause different behavior from Python in certain edge cases.

---

## Test Cases Analysis

From `reference/scancode-toolkit/tests/licensedcode/test_match.py`:

### Test: `test_filter_contained_matches_only_filter_contained_matches_with_same_licensings` (lines 591-601)

```python
overlap = LicenseMatch(rule=r1, qspan=Span(0, 5), ispan=Span(0, 5))
same_span1 = LicenseMatch(rule=r1, qspan=Span(1, 6), ispan=Span(1, 6))
same_span2 = LicenseMatch(rule=r2, qspan=Span(1, 6), ispan=Span(1, 6))

matches, discarded = filter_contained_matches([overlap, same_span1, same_span2])
assert matches == [overlap, same_span1]
assert discarded
```

**Analysis**:

- `overlap`: qspan(0, 5) = tokens 0,1,2,3,4
- `same_span1`: qspan(1, 6) = tokens 1,2,3,4,5
- `same_span2`: qspan(1, 6) = tokens 1,2,3,4,5 (same as same_span1)

After sorting by start, hilen desc, len desc:

1. `overlap` (start=0)
2. `same_span1` (start=1) or `same_span2` (start=1)

`same_span1.qspan == same_span2.qspan` → True (both Span(1, 6))
→ One gets discarded (same spans, same license expression)

Result: `[overlap, same_span1]` (or `same_span2` depending on stable sort)

### Test: `test_filter_contained_matches_does_filter_across_rules` (lines 693-707)

```python
m1 = LicenseMatch(rule=r1, qspan=Span(0, 5), ispan=Span(0, 5))
contained1 = LicenseMatch(rule=r2, qspan=Span(1, 2), ispan=Span(1, 2))
contained2 = LicenseMatch(rule=r3, qspan=Span(3, 4), ispan=Span(3, 4))
m5 = LicenseMatch(rule=r5, qspan=Span(1, 6), ispan=Span(1, 6))

result, _discarded = filter_contained_matches([m1, contained1, contained2, m5])
assert result == [m1, m5]
```

**Analysis**:

- `m1`: tokens 0-4
- `contained1`: tokens 1-1 (inside m1)
- `contained2`: tokens 3-3 (inside m1)
- `m5`: tokens 1-5 (overlaps with m1)

After sorting:

1. `m1` (start=0)
2. `contained1` (start=1)
3. `contained2` (start=3)
4. `m5` (start=1)

Process:

- `m1` vs `contained1`: `m1.qcontains(contained1)` → True, discard `contained1`
- `m1` vs `contained2`: `m1.qcontains(contained2)` → True, discard `contained2`
- `m1` vs `m5`: Neither contains the other (m1 has 0-4, m5 has 1-5, both overlap)

Result: `[m1, m5]`

### Test: `test_filter_contained_matches_matches_does_filter_matches_with_contained_spans_if_licenses_are_different` (lines 944-956)

```python
r1 = create_rule_from_text_and_expression(license_expression='apache-2.0')
m1 = LicenseMatch(rule=r1, qspan=Span(0, 2), ispan=Span(0, 2))

r2 = create_rule_from_text_and_expression(license_expression='apache-2.0')
m2 = LicenseMatch(rule=r2, qspan=Span(1, 6), ispan=Span(1, 6))

r3 = create_rule_from_text_and_expression(license_expression='apache-1.1')
m3 = LicenseMatch(rule=r3, qspan=Span(0, 2), ispan=Span(0, 2))

matches, discarded = filter_contained_matches([m1, m2, m3])
assert matches == [m1, m2]
assert discarded
```

**Analysis**:

- `m1`: tokens 0-1, license=apache-2.0
- `m2`: tokens 1-5, license=apache-2.0
- `m3`: tokens 0-1, license=apache-1.1

After sorting:

1. `m1` (start=0)
2. `m3` (start=0) - same start, but different hilen/len
3. `m2` (start=1)

Process:

- `m1` vs `m3`: `m1.qspan == m3.qspan` → True (both Span(0, 2))
  - `m1.coverage() >= m3.coverage()` → whichever has higher coverage wins
  - Let's say they're equal → `m3` discarded (or `m1` if m3 has higher coverage)
- Result keeps `m1` and `m2`

**This test confirms that same spans with different licenses are deduplicated** based on coverage.

---

## Key Differences Summary

| # | Aspect | Python | Rust | Impact |
|---|--------|--------|------|--------|
| 1 | Non-contiguous span handling | `Span` objects with proper set semantics | Bounds-only check when `qspan_positions` is None | **Medium** - affects merged matches |
| 2 | Expression-based containment | Not in `filter_contained_matches` | Added via `licensing_contains_match()` | **Low** - extension, not parity issue |
| 3 | Equal span comparison | Uses `Span.__eq__` (set equality) | Uses bounds comparison | **Medium** - same as #1 |

---

## Plan for 100% Parity

### Phase 1: Fix Non-Contiguous Span Handling

1. **Update `qcontains` to always use `qspan_positions`** when available:

   ```rust
   pub fn qcontains(&self, other: &LicenseMatch) -> bool {
       // Always prefer explicit position sets
       if let (Some(self_pos), Some(other_pos)) = 
           (&self.qspan_positions, &other.qspan_positions) {
           return other_pos.iter().all(|p| self_pos.contains(p));
       }
       
       // Fall back to bounds (only valid for contiguous spans)
       // ... existing logic
   }
   ```

2. **Ensure merged matches populate `qspan_positions`**:
   - Verify `combine_matches()` sets `qspan_positions` and `ispan_positions`
   - Current code at lines 116-144 appears correct

### Phase 2: Fix Equal Span Check

1. **Replace bounds check with actual span comparison**:

   ```rust
   // Instead of:
   if current.start_token == next.start_token && current.end_token == next.end_token {
   
   // Use:
   if current.qspan() == next.qspan() {
   ```

2. **Implement `qspan()` to return actual positions**:
   - Current implementation at models.rs:543-549 is correct
   - Returns `qspan_positions` if available, otherwise generates range

### Phase 3: Consider Removing Expression-Based Containment

**Decision Required**: Keep or remove `licensing_contains_match()`?

Arguments for keeping:

- Handles cases like `gpl-2.0 WITH exception` containing `gpl-2.0`
- May be an intentional improvement over Python

Arguments for removing:

- Not in Python reference
- May cause different output in golden tests
- Expression subsumption is Python's `filter_overlapping_matches` responsibility

**Recommendation**: Keep as optional feature behind a flag, default to Python behavior.

### Phase 4: Add Comprehensive Tests

Add tests to `match_refine.rs` matching Python test cases:

1. `test_filter_contained_matches_same_spans_different_licenses`
2. `test_filter_contained_matches_across_rules`
3. `test_filter_contained_matches_nested_contained`
4. `test_filter_contained_matches_non_overlapping_preserved`
5. `test_filter_contained_matches_discontinuous_spans`

---

## Expected Impact on Golden Tests

### Likely Affected Files

Golden tests with **merged matches** (non-contiguous `qspan_positions`) may produce different results:

1. Files with multiple license references that get merged
2. Files with partial license text matches that span gaps
3. Files with `WITH` expressions and their base licenses

### Expected Changes

1. **Fewer duplicate detections**: Non-contiguous span comparison may eliminate false positives
2. **Better containment detection**: Matches with gaps are properly compared
3. **Potential regression**: Expression-based containment removal may bring back some duplicates

---

## Verified Findings Against Python Reference

### Question 1: Is the non-contiguous span handling correctly understood?

**VERIFIED: Yes, but with important nuances.**

Python's `Span` class (spans.py) uses `intbitset` internally to store positions:

```python
# spans.py:50-114
class Span(Set):
    def __init__(self, *args):
        if len_args == 2:
            # args0 and args1 describe a start and end closed range
            self._set = intbitset(range(args[0], args[1] + 1))
        else:
            # some sequence or iterable of ints
            self._set = intbitset(list(args[0]))
```

**Key Python Span behaviors verified:**

1. **Equality is set-based** (spans.py:134-135):

   ```python
   def __eq__(self, other):
       return isinstance(other, Span) and self._set == other._set
   ```

   Two spans are equal if and only if they contain the exact same set of integers.

2. **Containment is set-based** (spans.py:177-210):

   ```python
   def __contains__(self, other):
       if isinstance(other, Span):
           return self._set.issuperset(other._set)
   ```

   A span contains another if and only if ALL positions of other are in self.

3. **Non-contiguous example**:

   ```python
   Span([1, 2, 10, 11])  # Creates span with positions {1, 2, 10, 11}
   # start=1, end=11, len=4, magnitude=11 (end-start+1)
   ```

**Rust's current implementation correctly handles this when `qspan_positions` is populated:**

```rust
// models.rs:491-496
if let (Some(self_positions), Some(other_positions)) =
    (&self.qspan_positions, &other.qspan_positions)
{
    return other_positions.iter().all(|p| self_positions.contains(p));
}
```

**BUG CONFIRMED**: The fallback to bounds-only check (lines 498-505) is incorrect for non-contiguous spans:

```rust
// BUGGY: This checks bounds, not actual positions
self.start_token <= other.start_token && self.end_token >= other.end_token
```

### Question 2: What exactly does Python's Span do with discontinuous positions?

**VERIFIED: Full set semantics.**

Python's Span is a proper mathematical set of integers, implemented with `intbitset` for efficiency. Key operations:

| Operation | Python Implementation | Complexity |
|-----------|----------------------|------------|
| `len(span)` | `len(self._set)` | O(1) |
| `span.start` | `self._set[0]` | O(1) |
| `span.end` | `self._set[-1]` | O(1) |
| `a == b` | `self._set == other._set` | O(min(len(a), len(b))) |
| `a in b` | `self._set.issuperset(other._set)` | O(len(other)) |
| `a & b` | `self._set.intersection(...)` | O(min(len(a), len(b))) |
| `a \| b` | `self._set.union(...)` | O(len(a) + len(b)) |

**Example of discontinuous span behavior:**

```python
>>> s = Span([1, 2, 10, 11])
>>> s.start
1
>>> s.end
11
>>> len(s)
4
>>> s.magnitude()  # end - start + 1
11
>>> Span([1, 2, 10, 11]) == Span([1, 2, 10, 11])
True
>>> Span([1, 2, 10, 11]) == Span(1, 11)  # Span(1,11) = {1,2,3,4,5,6,7,8,9,10,11}
False
>>> Span([1, 2]) in Span([1, 2, 10, 11])
True
>>> Span([1, 3]) in Span([1, 2, 10, 11])  # 3 is not in the set
False
```

### Question 3: Will changing equal span comparison break anything?

**VERIFIED: No, it will FIX a bug.**

Current Rust code at match_refine.rs:353:

```rust
if current.start_token == next.start_token && current.end_token == next.end_token {
```

This only checks bounds, not actual positions. For non-contiguous spans with same bounds but different internal positions, this will incorrectly declare them equal.

**Example of bug:**

| Match | qspan_positions | start_token | end_token |
|-------|-----------------|-------------|-----------|
| A | {1, 2, 10, 11} | 1 | 11 |
| B | {1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11} | 1 | 11 |

Current Rust: `A.start == B.start && A.end == B.end` → True (WRONG!)
Python: `A.qspan == B.qspan` → False (CORRECT)

**Fix Required:**

```rust
// Replace bounds check with set equality
fn qspan_eq(a: &LicenseMatch, b: &LicenseMatch) -> bool {
    match (&a.qspan_positions, &b.qspan_positions) {
        (Some(a_pos), Some(b_pos)) => a_pos == b_pos,
        (None, None) => a.start_token == b.start_token && a.end_token == b.end_token,
        // One has positions, one doesn't - need to compare properly
        (Some(a_pos), None) => {
            let b_range: HashSet<_> = (b.start_token..b.end_token).collect();
            a_pos == &b_range
        }
        (None, Some(b_pos)) => {
            let a_range: HashSet<_> = (a.start_token..a.end_token).collect();
            b_pos == &a_range
        }
    }
}
```

### Question 4: Is the expression-based containment in Rust an extension or divergence?

**VERIFIED: This is a DIVERGENCE from Python.**

Python's `filter_contained_matches` (match.py:1075-1184) uses **only** position-based containment:

```python
# match.py:1158-1176 - filter_contained_matches
if current_match.qcontains(next_match):  # Position-based only!
    discarded_append(matches_pop(j))
    continue

if next_match.qcontains(current_match):  # Position-based only!
    discarded_append(matches_pop(i))
    i -= 1
    break
```

Python's expression-based containment is in `filter_overlapping_matches` (match.py:1374, 1404, 1424, 1437):

```python
# match.py:1374-1385 - filter_overlapping_matches, MEDIUM overlap case
if (current_match.licensing_contains(next_match)
    and current_match.len() >= next_match.len()
    and current_match.hilen() >= next_match.hilen()
):
    discarded_append(matches_pop(j))
    continue
```

**Rust's current implementation in `filter_contained_matches` (match_refine.rs:364-372):**

```rust
if current.qcontains(&next) || licensing_contains_match(&current, &next) {
    discarded.push(matches.remove(j));
    continue;
}
```

**Impact Analysis:**

This divergence may cause Rust to discard matches that Python would keep:

| Scenario | Python | Rust (current) | Impact |
|----------|--------|----------------|--------|
| Same position, expression A contains B | Keep both | Discard B | **Potential difference** |
| Different position, expression A contains B | Keep both | Keep both | No difference |
| Same position, same expression | Discard one | Discard one | No difference |

**Recommendation:**

Remove `licensing_contains_match` from `filter_contained_matches` to match Python behavior exactly. The expression-based logic already exists in `filter_overlapping_matches` (Rust lines 606, 614, etc.) which is correct.

---

## Detailed Edge Case Analysis

### Edge Case 1: Merged Matches with Gaps

When matches are merged (e.g., license text appears in two places in a file), the resulting `qspan` becomes non-contiguous.

**Python example from test (test_match.py:858-881):**

```python
m1 = LicenseMatch(
    rule=r1,
    qspan=Span(50, 90) | Span(92, 142) | Span(151, 182) | Span(199, 200),
    ispan=Span(5, 21) | Span(23, 46) | Span(48, 77) | Span(79, 93) |
          Span(95, 100) | Span(108, 128) | Span(130, 142),
)
```

This match has:

- `start=50`, `end=200`
- Actual positions: {50..90, 92..142, 151..182, 199, 200}
- Length: 4 + 51 + 32 + 2 = 89 tokens
- Magnitude: 200 - 50 + 1 = 151

**Current Rust behavior:**

- If `qspan_positions` is populated: Correct
- If `qspan_positions` is None: Uses bounds (50, 200) which is WRONG

### Edge Case 2: Equal Bounds, Different Positions

Two matches with same start/end but different actual token sets.

```python
# Python
m1 = LicenseMatch(qspan=Span(0, 10))       # tokens 0-10
m2 = LicenseMatch(qspan=Span([0, 2, 10]))  # tokens 0, 2, 10

m1.qspan == m2.qspan  # False - different sets!
```

```rust
// Current Rust (WRONG)
m1.start_token == m2.start_token && m1.end_token == m2.end_token  // True!
```

### Edge Case 3: Touching but Non-Overlapping Matches

Matches that touch at boundaries should not be considered as having overlap.

```python
m1 = LicenseMatch(qspan=Span(0, 5))   # tokens 0,1,2,3,4
m2 = LicenseMatch(qspan=Span(5, 10))  # tokens 5,6,7,8,9

m1.qspan.overlap(m2.qspan)  # 0 - no overlap
m1.qspan.distance_to(m2.qspan)  # 1 - touching
```

Current Rust `qoverlap` (models.rs:508-521) handles this correctly for contiguous spans.

### Edge Case 4: License Expression Containment

```python
# gpl-2.0 WITH classpath-exception-2.0 "contains" gpl-2.0
# Python only uses this in filter_overlapping_matches, not filter_contained_matches
```

Rust incorrectly applies this in `filter_contained_matches`.

---

## Additional Changes Needed

### Change 1: Implement Proper Span Equality in `filter_contained_matches`

**Current (match_refine.rs:353):**

```rust
if current.start_token == next.start_token && current.end_token == next.end_token {
```

**Proposed:**

```rust
if spans_equal(&current, &next) {
```

**Helper function:**

```rust
fn spans_equal(a: &LicenseMatch, b: &LicenseMatch) -> bool {
    match (&a.qspan_positions, &b.qspan_positions) {
        (Some(a_pos), Some(b_pos)) => a_pos == b_pos,
        (None, None) => {
            // Both contiguous - compare bounds
            a.start_token == b.start_token && a.end_token == b.end_token
        }
        // Mixed case: one has explicit positions, one uses bounds
        // Need to check if bounds would produce same set
        (Some(a_pos), None) => {
            if a.start_token == b.start_token && a.end_token == b.end_token {
                // Check if a_pos is exactly the range b.start_token..b.end_token
                let expected_len = b.end_token.saturating_sub(b.start_token);
                a_pos.len() == expected_len && a_pos.iter().eq(b.start_token..b.end_token)
            } else {
                false
            }
        }
        (None, Some(b_pos)) => spans_equal(b, a),
    }
}
```

### Change 2: Remove Expression-Based Containment from `filter_contained_matches`

**Current (match_refine.rs:364-372):**

```rust
if current.qcontains(&next) || licensing_contains_match(&current, &next) {
    discarded.push(matches.remove(j));
    continue;
}
if next.qcontains(&current) || licensing_contains_match(&next, &current) {
    discarded.push(matches.remove(i));
    i = i.saturating_sub(1);
    break;
}
```

**Proposed (match Python):**

```rust
// Remove licensing_contains_match - expression subsumption is for filter_overlapping_matches
if current.qcontains(&next) {
    discarded.push(matches.remove(j));
    continue;
}
if next.qcontains(&current) {
    discarded.push(matches.remove(i));
    i = i.saturating_sub(1);
    break;
}
```

### Change 3: Ensure `qspan_positions` is Always Populated for Merged Matches

Verify that `combine_matches` (or equivalent merge logic) properly populates `qspan_positions` when creating merged matches.

**Check in:**

- `src/license_detection/match_refine.rs` - merge functions
- `src/license_detection/models.rs` - `combine()` equivalent

---

## Test Cases to Add

### Test 1: `test_filter_contained_matches_discontinuous_equal_spans`

```rust
#[test]
fn test_filter_contained_matches_discontinuous_equal_spans() {
    // Two matches with same bounds but different actual positions
    // Should NOT be considered equal spans
    
    let mut m1 = create_test_match("mit", 0, 10);
    m1.qspan_positions = Some(HashSet::from([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]));
    
    let mut m2 = create_test_match("mit", 0, 10);
    m2.qspan_positions = Some(HashSet::from([0, 2, 4, 6, 8]));  // Same bounds, different positions
    
    let (kept, discarded) = filter_contained_matches(&[m1.clone(), m2.clone()]);
    
    // Python: m1.qspan != m2.qspan, so neither is discarded as "equal"
    // m1.qcontains(m2) is True (m1 has all of m2's positions)
    // So m2 should be discarded as contained
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].qspan_positions, m1.qspan_positions);
}
```

### Test 2: `test_filter_contained_matches_no_expression_containment`

```rust
#[test]
fn test_filter_contained_matches_no_expression_containment() {
    // Verify that expression containment is NOT used in filter_contained_matches
    // (It's only for filter_overlapping_matches)
    
    let m1 = create_test_match("gpl-2.0", 0, 10);
    let m2 = create_test_match("gpl-2.0 WITH classpath-exception-2.0", 0, 10);
    
    // Same positions, different expressions
    // Python: Neither contains the other (same positions, different licenses)
    // Coverage decides which to keep
    
    let (kept, discarded) = filter_contained_matches(&[m1, m2]);
    
    // Should discard one based on coverage, not expression containment
    assert_eq!(kept.len(), 1);
    assert_eq!(discarded.len(), 1);
}
```

### Test 3: `test_filter_contained_matches_merged_match_containment`

```rust
#[test]
fn test_filter_contained_matches_merged_match_containment() {
    // Merged match (non-contiguous) should properly contain/overlap with others
    
    let mut merged = create_test_match("mit", 0, 20);
    merged.qspan_positions = Some(HashSet::from([0, 1, 2, 10, 11, 12]));  // Gap in middle
    
    let inner = create_test_match("mit", 0, 3);  // tokens 0,1,2 - inside merged
    
    let (kept, discarded) = filter_contained_matches(&[merged.clone(), inner]);
    
    // inner.qspan = {0,1,2} is subset of merged.qspan = {0,1,2,10,11,12}
    // So inner should be discarded
    assert_eq!(kept.len(), 1);
    assert!(discarded.iter().any(|m| m.start_token == 0 && m.end_token == 3));
}
```

### Test 4: `test_filter_contained_matches_touching_not_overlapping`

```rust
#[test]
fn test_filter_contained_matches_touching_not_overlapping() {
    // Matches that touch at boundaries should not be considered overlapping
    
    let m1 = create_test_match("mit", 0, 5);   // tokens 0,1,2,3,4
    let m2 = create_test_match("apache-2.0", 5, 10);  // tokens 5,6,7,8,9
    
    let (kept, discarded) = filter_contained_matches(&[m1.clone(), m2.clone()]);
    
    // Neither contains the other, both should be kept
    assert_eq!(kept.len(), 2);
    assert!(discarded.is_empty());
}
```

### Test 5: Port Python test `test_merge_does_not_merge_overlapping_matches_in_sequence_with_assymetric_overlap`

This test (test_match.py:848-881) specifically tests non-contiguous spans and verifies they are NOT merged when overlap is asymmetric.

---

## Summary of Required Changes

| # | Change | File | Lines | Priority |
|---|--------|------|-------|----------|
| 1 | Fix equal span check to use set equality | match_refine.rs | 353 | **HIGH** |
| 2 | Remove `licensing_contains_match` from `filter_contained_matches` | match_refine.rs | 364-372 | **HIGH** |
| 3 | Ensure `qspan_positions` populated for merged matches | match_refine.rs | merge logic | **MEDIUM** |
| 4 | Add `spans_equal` helper function | match_refine.rs | new | **HIGH** |
| 5 | Add comprehensive tests | match_refine_test.rs | new | **HIGH** |

---

## References

- Python source: `reference/scancode-toolkit/src/licensedcode/match.py:1075-1184`
- Python tests: `reference/scancode-toolkit/tests/licensedcode/test_match.py:565-1014`
- Python spans: `reference/scancode-toolkit/src/licensedcode/spans.py`
- Rust source: `src/license_detection/match_refine.rs:326-380`
- Rust models: `src/license_detection/models.rs:491-506`
- Rust expression: `src/license_detection/expression.rs:444-506`
