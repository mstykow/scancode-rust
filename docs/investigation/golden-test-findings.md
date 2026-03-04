# Golden Test Investigation Report

## Executive Summary

**Current Baseline:** 104 failing golden tests out of 4703 total (97.8% pass rate)

This report documents the investigation into the remaining 104 failing golden tests in the scancode-rust license detection pipeline. The failures have been categorized into five primary root causes, with attempted fixes documented alongside their outcomes.

**Key Findings:**

- Duplicate detection issues account for ~40 failures
- Containment filtering issues account for ~20 failures  
- Wrong license selection accounts for ~30 failures
- Missing duplicates account for ~15 failures
- Expression rendering issues account for ~12 failures

Several attempted fixes were reverted due to causing additional regressions, indicating the pipeline has subtle interdependencies that require careful consideration.

---

## Key Findings

### 1. Duplicate Detection Issues

**Tests Affected:** ~40 tests

**Root Cause:** Multiple occurrences of the same license being incorrectly merged or incorrectly kept separate.

**Symptoms:**

- Files with multiple identical license references show wrong count
- Example: `ar-ER.js.map` - Expected 1 "mit" match, Actual has 2
- Example: `DNSDigest.c` - Expected 3 "apache-2.0" matches, Actual has 2

**Investigation:**

The `merge_overlapping_matches()` function in `src/license_detection/match_refine/merge.rs` handles merging of matches from the same rule. The key logic compares `qspan` and `ispan` positions to determine if matches are duplicates:

```rust
if current_qspan == next_qspan && current_ispan == next_ispan {
    rule_matches.remove(j);
    continue;
}
```

**Attempted Fix:** Added HashSet equality check for qspan/ispan comparison in `merge.rs:135`

**Result:** Kept (already in codebase). This fix prevents merging of matches with identical token positions, which is correct behavior.

**Remaining Issues:**

- Some cases may require distinguishing between "same license text at different locations" vs "duplicate detection of same text"
- The `filter_license_references_with_text_match()` function may incorrectly discard legitimate separate occurrences

**Files Involved:**

- `src/license_detection/match_refine/merge.rs:130-138`
- `src/license_detection/seq_match/candidates.rs:144-174` (filter_dupes function)

---

### 2. Containment Filtering Issues

**Tests Affected:** ~20 tests

**Root Cause:** `filter_contained_matches()` removes matches from different rules when they should be kept as separate detections.

**Symptoms:**

- GPL variant matches incorrectly filtered (e.g., gpl-1.0-plus contained within gpl-2.0-plus)
- Expression-based filtering may discard matches with different `license_expression` values
- Example: `gpl-2.0-plus_and_mpl-1.0.txt` - Combined rule not detected because contained matches are filtered

**Investigation:**

The `filter_contained_matches()` function in `src/license_detection/match_refine/handle_overlaps.rs:40-96` uses `qcontains()` to determine if one match is contained within another:

```rust
if current.qcontains(&next) {
    discarded.push(matches.remove(j));
    continue;
}
```

This logic correctly filters matches that are fully contained within other matches, but the issue arises when:

1. A smaller match (e.g., GPL-1.0 header reference) is contained within a larger match (GPL-2.0 license text)
2. The smaller match has a DIFFERENT `license_expression` and should be reported separately
3. The containment check discards the smaller match, losing the detection

**Attempted Fix:** Added `license_expression` check before filtering

```rust
// Only filter if same license expression
if current.license_expression == next.license_expression && current.qcontains(&next) {
    discarded.push(matches.remove(j));
    continue;
}
```

**Result:** REVERTED - Caused 38 additional test failures

**Why It Failed:** Many legitimate cases require filtering contained matches even with different expressions. For example:
- A "MIT" reference contained within full MIT license text should be filtered
- GPL-1.0 contained within GPL-2.0-plus should sometimes be filtered (when it's a partial match)
- The expression relationship matters (e.g., WITH expressions subsuming base licenses)

**Correct Approach (Not Yet Implemented):**

The fix requires using `licensing_contains()` to check if one expression subsumes another:

```rust
if current.qcontains(&next) {
    // Only filter if the containing license expression subsumes the contained one
    if licensing_contains(&current.license_expression, &next.license_expression) {
        discarded.push(matches.remove(j));
        continue;
    }
}
```

This is partially implemented in `filter_overlapping_matches()` but not in `filter_contained_matches()`.

**Files Involved:**

- `src/license_detection/match_refine/handle_overlaps.rs:40-96`
- `src/license_detection/expression/simplify.rs:246-308` (licensing_contains function)

---

### 3. Wrong License Selection

**Tests Affected:** ~30 tests

**Root Cause:** Candidate tiebreaker prefers longer/more relevant rules instead of matching Python's rid-based ordering.

**Symptoms:**

- Similar licenses with different specificity selected incorrectly
- Example: `cc-by-nc-4.0` vs `cc-by-4.0` (NC variant should be selected when text mentions non-commercial)
- Example: `bsd-simplified` vs `bsd-new` (2-clause vs 3-clause)

**Investigation:**

The candidate ordering in `src/license_detection/seq_match/candidates.rs:93-116` uses multiple criteria:

```rust
impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score_vec_rounded
            .cmp(&other.score_vec_rounded)
            .then_with(|| self.score_vec_full.cmp(&other.score_vec_full))
            .then_with(|| self.rule.tokens.len().cmp(&other.rule.tokens.len()))
            .then_with(|| self.rule.relevance.cmp(&other.rule.relevance))
    }
}
```

The tiebreaker prefers:
1. Longer rule text (more tokens)
2. Higher relevance

**Attempted Fix:** Match Python's rid-based ordering

```rust
.then_with(|| b.rule.identifier.cmp(&a.rule.identifier))
```

**Result:** REVERTED - Caused 3 additional test failures

**Why It Failed:** Python's rid assignment is based on rule file loading order, which is filesystem-dependent. Replicating this exactly would make the codebase fragile and platform-dependent. The current approach is more deterministic but may select different candidates in edge cases.

**Alternative Approaches:**

1. **Semantic tiebreaking:** Prefer licenses with more specific constraints (e.g., NC variants over base licenses)
2. **Rule metadata:** Use rule properties like `is_license_text`, `is_license_reference` to inform selection
3. **Expression complexity:** Prefer expressions that are more specific (contain more constraints)

**Files Involved:**

- `src/license_detection/seq_match/candidates.rs:93-116`
- `src/license_detection/rules/loader.rs` (rid assignment)

---

### 4. Missing Duplicates

**Tests Affected:** ~15 tests

**Root Cause:** License references and license text being incorrectly grouped together, causing separate detections to be merged.

**Symptoms:**

- Files with license reference header + full license text show wrong expression
- Example: `MIT.t10` - "The MIT License" header at line 1 should be separate from license text at lines 5-20
- Expression shows only "mit" instead of separate detections

**Investigation:**

The `filter_license_references_with_text_match()` function in `src/license_detection/match_refine/merge.rs:275-318` filters license references when a text match exists:

```rust
if current_is_ref
    && other_is_text
    && current.matched_length < other.matched_length
    && other.qcontains(current)
{
    to_discard.insert(i);
}
```

This correctly filters references that are CONTAINED within text matches. However, the issue arises when:

1. A reference is at a DIFFERENT location than the text
2. The reference should create a separate detection
3. But the grouping logic (`group_matches_by_region`) groups them together due to line proximity

**Python Behavior Analysis:**

The Python implementation may create separate detections for:
- `is_license_reference` matches at different locations
- `is_license_text` matches as primary detections
- References NOT contained within text regions are kept

**Investigation Needed:**

1. How does Python handle `license_reference` flag in detection creation?
2. Does Python use token gaps (not just line gaps) for grouping?
3. Are references always separate detections or only when not contained?

**Files Involved:**

- `src/license_detection/match_refine/merge.rs:275-318`
- `src/license_detection/detection/grouping.rs:21-64`
- `src/license_detection/models/rule.rs` (is_license_reference, is_license_text flags)

---

### 5. Expression Rendering Issues

**Tests Affected:** ~12 tests

**Root Cause:** Parenthesization doesn't preserve structural grouping of nested AND/OR expressions.

**Symptoms:**

- Combined expressions show incorrect parentheses placement
- Example: `(mit OR apache-2.0) AND gpl-2.0` vs expected `mit OR apache-2.0 AND gpl-2.0`
- Some expressions missing parentheses, others have unnecessary parentheses

**Investigation:**

The `expression_to_string()` function in `src/license_detection/expression/simplify.rs:358-395` uses precedence-based parenthesization:

```rust
LicenseExpression::And { left, right } => {
    let left_str = expression_to_string_internal(left, Some(Precedence::And));
    let right_str = expression_to_string_internal(right, Some(Precedence::And));
    let result = format!("{} AND {}", left_str, right_str);
    if parent_prec.is_some_and(|p| p != Precedence::And) {
        format!("({})", result)
    } else {
        result
    }
}
```

The precedence hierarchy is: `OR < AND < WITH`

**Issues Identified:**

1. Python's license-expression library may have different parenthesization rules
2. Some edge cases with nested expressions produce different output
3. The `simplify_expression()` function may restructure expressions in ways that change output

**Test Cases:**

- `mit OR apache-2.0 AND gpl-2.0` - Python: `mit OR (apache-2.0 AND gpl-2.0)`, Rust: `mit OR (apache-2.0 AND gpl-2.0)` ✓
- `bsd-new AND mit AND gpl-3.0-plus WITH autoconf-simple-exception` - May differ in parentheses placement

**Files Involved:**

- `src/license_detection/expression/simplify.rs:358-395`
- `src/license_detection/expression/simplify.rs:14-35` (simplify_expression)
- `src/license_detection/expression/parse.rs` (expression parsing)

---

## Recommended Next Steps

### Priority 1: Containment Filtering Fix

**Action:** Implement expression-aware containment filtering

**Location:** `src/license_detection/match_refine/handle_overlaps.rs:filter_contained_matches()`

**Approach:**
```rust
if current.qcontains(&next) {
    // Use licensing_contains to check semantic containment
    if licensing_contains(&current.license_expression, &next.license_expression) {
        discarded.push(matches.remove(j));
        continue;
    }
    // Otherwise, keep both matches (different licenses)
}
```

**Expected Impact:** Fix ~20 tests without causing regressions

**Risk:** Medium - Need to verify `licensing_contains()` handles all edge cases correctly

---

### Priority 2: Investigate License Reference Handling

**Action:** Study Python's behavior for `is_license_reference` matches

**Steps:**
1. Run Python ScanCode on test files with license references
2. Analyze how references are grouped vs text matches
3. Check if token gaps are used in addition to line gaps
4. Document the exact rules for reference detection creation

**Test Files:**
- `testdata/license-golden/datadriven/lic2/ar-ER.js.map`
- Files with `MIT.t10` pattern
- Files with license headers followed by license text

**Expected Impact:** Fix ~15 tests

---

### Priority 3: Token-Based Grouping

**Action:** Investigate if Python uses token gaps for match grouping

**Current Implementation:** `src/license_detection/detection/grouping.rs:should_group_together()`

```rust
pub(super) fn should_group_together(
    prev: &LicenseMatch,
    cur: &LicenseMatch,
    threshold: usize,
) -> bool {
    let line_gap = cur.start_line.saturating_sub(prev.end_line);
    line_gap <= threshold
}
```

**Investigation:**
1. Check Python's `group_matches()` implementation
2. Determine if token gap threshold exists alongside line gap
3. If so, implement token gap checking

**Expected Impact:** Fix subset of containment/grouping issues

---

### Priority 4: Expression Rendering Review

**Action:** Compare Rust and Python expression rendering character-by-character

**Steps:**
1. Extract failing test expressions from Python output
2. Compare with Rust output
3. Identify specific parenthesization differences
4. Adjust `expression_to_string()` if needed

**Note:** May be acceptable differences if semantically equivalent

**Expected Impact:** Fix ~12 tests (or document as acceptable differences)

---

### Priority 5: Rule Loading Order Verification

**Action:** Verify rid assignment matches Python

**Investigation:**
1. Compare rule file loading order in Python vs Rust
2. Check if Python's rid assignment is deterministic
3. Document any intentional differences

**Expected Impact:** Fix subset of wrong license selection issues

---

## Appendix: Test Case Categories

### A. Duplicate Detection Failures

| Test File | Expected | Actual | Root Cause |
|-----------|----------|--------|------------|
| `ar-ER.js.map` | 1 mit | 2 mit | Multiple MIT refs not deduplicated |
| `DNSDigest.c` | 3 apache-2.0 | 2 apache-2.0 | Apache refs incorrectly merged |
| `sa11xx_base.c` | 2 mpl-1.1 OR gpl-2.0 | 1 mpl-1.1 OR gpl-2.0 | OR expressions merged |

### B. Containment Filtering Failures

| Test File | Expected | Actual | Root Cause |
|-----------|----------|--------|------------|
| `gpl-2.0-plus_and_mpl-1.0.txt` | mpl-1.0 OR gpl-2.0-plus | Separate detections | Contained matches filtered incorrectly |
| `gpl-2.0_9.txt` | gpl-2.0-plus | gpl-1.0-plus, gpl-2.0-plus | GPL variants not merged |

### C. Wrong License Selection Failures

| Test File | Expected | Actual | Root Cause |
|-----------|----------|--------|------------|
| `cc-by-nc-4.0` variants | cc-by-nc-4.0 | cc-by-4.0 | Base license selected over NC variant |
| `bsd` variants | bsd-simplified | bsd-new | Wrong BSD variant selected |

### D. Expression Rendering Failures

| Test File | Expected Expression | Actual Expression | Difference |
|-----------|---------------------|-------------------|------------|
| Mixed AND/OR | `(a OR b) AND c` | `a OR b AND c` | Missing parentheses |
| WITH expressions | `a WITH b AND c` | `(a WITH b) AND c` | Extra parentheses |

---

## Conclusion

The 104 failing golden tests represent edge cases in the license detection pipeline that require careful investigation. The primary challenges are:

1. **Containment filtering** - Requires semantic understanding of license expression relationships
2. **Duplicate detection** - Balancing deduplication with preserving legitimate separate detections
3. **Candidate selection** - Matching Python's behavior without replicating platform-specific ordering
4. **Expression rendering** - Ensuring consistent parenthesization across implementations

The recommended approach is to address issues in priority order, with thorough testing after each fix to ensure no regressions. The `licensing_contains()` function provides a solid foundation for semantic-aware filtering and should be leveraged for containment decisions.
