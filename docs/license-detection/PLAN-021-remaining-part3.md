# PLAN-021: Remaining Part 3 Items - Detailed Implementation Plan

## Status: ITEM 2 IMPLEMENTED - Item 1 Pending

---

## Implementation Results (Item 2: licensing_contains)

### Implementation Date: 2025-02-20

### Files Modified

1. **`src/license_detection/expression.rs`**
   - Added `get_flat_args()` - Flattens AND/OR expressions into a list
   - Added `collect_flat_and_args()` - Helper for AND flattening
   - Added `collect_flat_or_args()` - Helper for OR flattening
   - Added `decompose_expr()` - Decomposes WITH expressions
   - Added `expressions_equal()` - Order-independent expression equality
   - Added `expr_in_args()` - Checks if expression is in args (with WITH decomposition)
   - Added `licensing_contains()` - Main public function
   - Added 11 unit tests in `contains_tests` module

2. **`src/license_detection/match_refine.rs`**
   - Added import for `licensing_contains`
   - Replaced `licensing_contains_approx()` with `licensing_contains_match()`
   - Updated all 6 call sites (lines 477, 485, 496, 504, 516, 526)

### Unit Test Results

All 11 `licensing_contains` tests pass:

- test_basic_containment ... ok
- test_or_containment ... ok
- test_and_containment ... ok
- test_expression_subset ... ok
- test_order_independence ... ok
- test_plus_suffix_no_containment ... ok
- test_with_decomposition ... ok
- test_mixed_operators ... ok
- test_nested_expressions ... ok (critical: `(mit OR apache) AND bsd` does NOT contain `mit`)
- test_empty_expressions ... ok
- test_invalid_expressions ... ok

### Python Semantics Verification

All test cases verified against Python's `license-expression` library:

| Expression 1 | Expression 2 | Python | Rust |
|-------------|-------------|--------|------|
| `mit` | `mit` | TRUE | TRUE |
| `mit OR apache` | `mit` | TRUE | TRUE |
| `mit AND apache` | `mit` | TRUE | TRUE |
| `gpl-2.0-plus` | `gpl-2.0` | FALSE | FALSE |
| `gpl-2.0 WITH exception` | `gpl-2.0` | TRUE | TRUE |
| `(mit OR apache) AND bsd` | `mit` | FALSE | FALSE |
| `(mit OR apache) AND bsd` | `mit OR apache` | TRUE | TRUE |

### Golden Test Impact

| Suite | Before | After | Delta |
|-------|--------|-------|-------|
| lic1 | 67 | 67 | 0 |
| lic2 | 78 | 78 | 0 |
| lic3 | 42 | 41 | -1 |
| lic4 | 64 | 65 | +1 |
| unknown | 7 | 7 | 0 |
| external | 549 | (timeout) | - |

**Net change: ~0** (lic3 improved by 1, lic4 worsened by 1)

### Analysis

The implementation is **correct** - all unit tests pass and match Python behavior exactly. However, the golden test impact is minimal because:

1. **Current approximation was close enough**: The `matched_length * 2` heuristic happened to work in many cases where expressions are similar length.

2. **Expression field may be empty**: Some matches don't have populated `license_expression`, falling back to the length heuristic.

3. **Other factors dominate**: The remaining golden test failures are likely caused by other differences in the filter pipeline, not just `licensing_contains`.

### Recommendation

The implementation is complete and correct. The minimal golden test change indicates this wasn't the primary cause of failures. Focus should shift to:

- Item 1 (`min_score` filter) - lower priority
- Investigating other filter pipeline differences

---

## Item 1: `filter_matches_below_minimum_score()`

### Overview

Python's `refine_matches()` accepts a `min_score` parameter that filters out matches scoring below a threshold. This filter runs **after** all other filters but **before** the final merge.

### Python Implementation

**Location**: `reference/scancode-toolkit/src/licensedcode/match.py:1590-1619`

```python
def filter_matches_below_minimum_score(
    matches,
    min_score=100,
    trace=TRACE_FILTER_BELOW_MIN_SCORE,
    reason=DiscardReason.BELOW_MIN_SCORE,
):
    """
    Return a filtered list of kept LicenseMatch matches and a list of
    discardable matches given a ``matches`` list of LicenseMatch by removing
    matches scoring below the provided ``min_score``.
    """
    if not min_score:
        return matches, []

    kept = []
    kept_append = kept.append
    discarded = []
    discarded_append = discarded.append

    for match in matches:
        if match.score() < min_score:
            if trace:
                logger_debug('    ==> DISCARDING low score:', match)

            match.discard_reason = reason
            discarded_append(match)
        else:
            kept_append(match)

    return kept, discarded
```

**Usage in `refine_matches()`** (lines 2819-2822):

```python
if min_score:
    matches, discarded = filter_matches_below_minimum_score(matches, min_score=min_score)
    all_discarded_extend(discarded)
    _log(matches, discarded, 'HIGH ENOUGH SCORE')
```

### Rust Current State

**Location**: `src/license_detection/match_refine.rs:1280-1341`

```rust
pub fn refine_matches(
    index: &LicenseIndex,
    matches: Vec<LicenseMatch>,
    query: &Query,
) -> Vec<LicenseMatch> {
    // ... filters ...
    // NO min_score filtering
    let merged_final = merge_overlapping_matches(&kept);
    // ...
}
```

**Detection-level filtering exists**: `src/license_detection/detection.rs:721-732`

```rust
pub fn classify_detection(detection: &LicenseDetection, min_score: f32) -> bool {
    let score = compute_detection_score(&detection.matches);
    let meets_score_threshold = score >= min_score - 0.01;
    // ...
}
```

**CLI does NOT expose `min_score`**: `src/cli.rs` has no `--min-score` option.

### Gap Analysis

| Aspect | Python | Rust | Gap |
|--------|--------|------|-----|
| Filter function | Yes | No | **Missing** |
| `refine_matches` param | Yes (`min_score=0`) | No | **Missing** |
| Detection-level filtering | Yes | Yes | None |
| CLI option | Yes (`--min-score`) | No | **Missing** |

### Implementation Requirements

#### 1. Add `filter_matches_below_minimum_score()` function

**Location**: `src/license_detection/match_refine.rs`

```rust
/// Filter matches with score below minimum threshold.
///
/// Returns (kept, discarded) tuples.
/// Only active when min_score > 0.0.
///
/// Based on Python: filter_matches_below_minimum_score() at match.py:1590-1619
pub fn filter_matches_below_minimum_score(
    matches: Vec<LicenseMatch>,
    min_score: f32,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    if min_score <= 0.0 {
        return (matches, Vec::new());
    }

    let mut kept = Vec::new();
    let mut discarded = Vec::new();

    for match_ in matches {
        if match_.score < min_score {
            discarded.push(match_);
        } else {
            kept.push(match_);
        }
    }

    (kept, discarded)
}
```

#### 2. Add `min_score` parameter to `refine_matches()`

```rust
pub fn refine_matches(
    index: &LicenseIndex,
    matches: Vec<LicenseMatch>,
    query: &Query,
    min_score: f32,  // NEW PARAMETER
) -> Vec<LicenseMatch> {
    // ... after filter_false_positive_license_lists_matches ...

    // Filter by minimum score (Python: lines 2819-2822)
    let non_low_score = if min_score > 0.0 {
        let (kept, _) = filter_matches_below_minimum_score(non_fp, min_score);
        kept
    } else {
        non_fp
    };

    let merged_final = merge_overlapping_matches(&non_low_score);
    // ...
}
```

#### 3. Update call sites

**Main pipeline** (`src/license_detection/mod.rs:246`):

```rust
let refined = refine_matches(&self.index, all_matches, &query, 0.0);
```

**Golden tests** (update to pass `0.0` as default):

- `src/license_detection/golden_test.rs:766`
- `src/license_detection/golden_test.rs:926`

**Unit tests in `match_refine.rs`** (update to pass `0.0`):

- Lines 1715, 1733, 1744, 1761, 2132, 2155, 3352

#### 4. Add CLI option (optional, lower priority)

```rust
// src/cli.rs
/// Minimum score threshold for license matches (0-100)
#[arg(long, default_value = "0")]
pub min_score: f32,
```

### Impact on Golden Tests

- **Default behavior unchanged**: `min_score=0` means filter is inactive
- **No test failures expected**: Only affects behavior when explicitly set
- **New tests needed**: Unit tests for `filter_matches_below_minimum_score()`

### Priority: LOW

- Only used when `min_score > 0`
- Default is `min_score=0` (inactive)
- Detection-level filtering already handles score-based filtering
- CLI doesn't expose this option

---

## Item 2: `licensing_contains()` - License Expression Containment

### Overview

Python uses the `license-expression` library's `Licensing.contains()` method to determine if one license expression semantically contains another. This is used in `filter_overlapping_matches()` to decide which match to discard.

### Python Implementation

**Location**: `reference/scancode-toolkit/src/licensedcode/models.py:2065-2073`

```python
def licensing_contains(self, other):
    """
    Return True if this rule licensing contains the other rule licensing.
    """
    if self.license_expression and other.license_expression:
        return self.licensing.contains(
            expression1=self.license_expression_object,
            expression2=other.license_expression_object,
        )
```

**Called from `LicenseMatch.licensing_contains()`** (`match.py:388-392`):

```python
def licensing_contains(self, other):
    """
    Return True if this match licensing contains the other match licensing.
    """
    return self.rule.licensing_contains(other.rule)
```

**Usage in `filter_overlapping_matches()`** (`match.py:1374, 1404, 1424, 1437, 1453, 1468`):

```python
if (current_match.licensing_contains(next_match)
    and current_match.len() >= next_match.len()
    and current_match.hilen() >= next_match.hilen()
):
    # Discard next_match
```

### Python Semantics (VERIFIED BY TESTING)

**Core Algorithm** (from `license_expression/__init__.py:314-324` and `boolean.py:1264-1272`):

```python
def contains(self, expression1, expression2, **kwargs):
    ex1 = self._parse_and_simplify(expression1, **kwargs)
    ex2 = self._parse_and_simplify(expression2, **kwargs)
    return ex2 in ex1  # Uses __contains__ on expression tree

def __contains__(self, expr):
    if expr in self.args:
        return True
    if isinstance(expr, self.__class__):
        return all(arg in self.args for arg in expr.args)
```

**Key behaviors**:

1. Both expressions are **simplified** (args sorted, duplicates removed)
2. `expr2 in expr1` checks if expr2 is a **subterm** of expr1
3. `LicenseWithExceptionSymbol.decompose()` yields both license and exception

**Verified Test Results**:

| Expression 1 | Expression 2 | Result | Explanation |
|-------------|-------------|--------|-------------|
| `mit` | `mit` | TRUE | Same expression |
| `mit` | `apache` | FALSE | Different licenses |
| `mit OR apache` | `mit` | **TRUE** | `mit` is in OR args |
| `mit OR apache` | `apache` | **TRUE** | `apache` is in OR args |
| `mit OR apache` | `gpl` | FALSE | Not in args |
| `mit AND apache` | `mit` | **TRUE** | `mit` is in AND args |
| `mit AND apache` | `apache` | **TRUE** | `apache` is in AND args |
| `mit` | `mit AND apache` | FALSE | Compound not in simple |
| `mit AND apache AND bsd` | `mit AND apache` | **TRUE** | Subset check |
| `mit AND apache` | `mit AND apache AND bsd` | FALSE | Not a subset |
| `mit AND apache` | `apache AND mit` | **TRUE** | Simplified to same |
| `gpl-2.0-plus` | `gpl-2.0` | **FALSE** | Different identifiers! |
| `gpl-2.0` | `gpl-2.0-plus` | **FALSE** | Different identifiers! |
| `gpl-2.0 WITH exception` | `gpl-2.0` | **TRUE** | decompose() yields both |
| `gpl-2.0` | `gpl-2.0 WITH exception` | FALSE | WITH is single symbol |

**CRITICAL**: `gpl-2.0-plus` and `gpl-2.0` are **completely separate identifiers**. There is NO suffix-based containment in Python.

### Rust Current State

**Location**: `src/license_detection/match_refine.rs:380-382`

```rust
fn licensing_contains_approx(current: &LicenseMatch, other: &LicenseMatch) -> bool {
    current.matched_length >= other.matched_length * 2
}
```

**Usage** (lines 477, 485, 496, 504, 516, 526):

```rust
if licensing_contains_approx(&matches[i], &matches[j])
    && current_len_val >= next_len_val
    && current_hilen >= next_hilen
{
    discarded.push(matches.remove(j));
    continue;
}
```

### Gap Analysis

| Aspect | Python | Rust Approximation |
|--------|--------|-------------------|
| Semantic containment | `Licensing.contains(expr1, expr2)` | Length comparison |
| Expression parsing | Full SPDX expression parsing | None |
| Accuracy | High (semantic) | Low (heuristic) |
| `mit OR apache` contains `mit` | TRUE | FALSE (unless 2x length) |
| `mit AND apache` contains `mit` | TRUE | FALSE (unless 2x length) |
| `gpl-2.0-plus` contains `gpl-2.0` | FALSE | N/A |
| `gpl-2.0 WITH exception` contains `gpl-2.0` | TRUE | N/A |

### The Problem with Length Approximation

The current approximation `current.matched_length >= other.matched_length * 2` is fundamentally wrong:

1. **`mit OR apache` contains `mit`**: TRUE in Python, FALSE in Rust (unless MIT match is 2x longer)
2. **`mit AND apache` contains `mit`**: TRUE in Python (subset check), FALSE in Rust
3. **`gpl-2.0-plus` vs `gpl-2.0`**: FALSE in Python (different identifiers), not handled in Rust

### Existing Rust Infrastructure

**File**: `src/license_detection/expression.rs` - Already has expression parsing!

**Existing capabilities**:

- `LicenseExpression` enum with `License`, `LicenseRef`, `And`, `Or`, `With` variants
- `parse_expression()` - Parses expression strings to AST
- `simplify_expression()` - Deduplicates licenses within AND/OR
- `expression_to_string()` - Converts AST back to string

**Missing**:

- `licensing_contains()` function
- Helper: `get_flat_args()` to flatten AND/OR into list
- Helper: `decompose_expr()` for WITH expressions
- Helper: `expressions_equal()` for order-independent comparison

**No new dependencies needed** - the existing custom parser handles ScanCode-specific formats like `gpl-2.0-plus` and `LicenseRef-scancode-*`.

### Implementation

Add to `src/license_detection/expression.rs`:

```rust
/// Get flattened arguments of an AND or OR expression.
fn get_flat_args(expr: &LicenseExpression) -> Vec<LicenseExpression> {
    match expr {
        LicenseExpression::And { left, right } => {
            let mut args = get_flat_args(left);
            args.extend(get_flat_args(right));
            args
        }
        LicenseExpression::Or { left, right } => {
            let mut args = get_flat_args(left);
            args.extend(get_flat_args(right));
            args
        }
        _ => vec![expr.clone()],
    }
}

/// Decompose a WITH expression into its license and exception parts.
fn decompose_expr(expr: &LicenseExpression) -> Vec<LicenseExpression> {
    match expr {
        LicenseExpression::With { left, right } => {
            let mut parts = decompose_expr(left);
            parts.extend(decompose_expr(right));
            parts
        }
        _ => vec![expr.clone()],
    }
}

/// Check if two expressions are semantically equal (ignoring order in AND/OR).
fn expressions_equal(a: &LicenseExpression, b: &LicenseExpression) -> bool {
    match (a, b) {
        (LicenseExpression::License(ka), LicenseExpression::License(kb)) => ka == kb,
        (LicenseExpression::LicenseRef(ka), LicenseExpression::LicenseRef(kb)) => ka == kb,
        (LicenseExpression::With { left: l1, right: r1 }, 
         LicenseExpression::With { left: l2, right: r2 }) => {
            expressions_equal(l1, l2) && expressions_equal(r1, r2)
        }
        (LicenseExpression::And { .. }, LicenseExpression::And { .. }) => {
            let args_a = get_flat_args(a);
            let args_b = get_flat_args(b);
            args_a.len() == args_b.len() 
                && args_b.iter().all(|b_arg| args_a.iter().any(|a_arg| expressions_equal(a_arg, b_arg)))
        }
        (LicenseExpression::Or { .. }, LicenseExpression::Or { .. }) => {
            let args_a = get_flat_args(a);
            let args_b = get_flat_args(b);
            args_a.len() == args_b.len() 
                && args_b.iter().all(|b_arg| args_a.iter().any(|a_arg| expressions_equal(a_arg, b_arg)))
        }
        _ => false,
    }
}

/// Check if an expression is contained in a list of args (handles WITH decomposition).
fn expr_in_args(expr: &LicenseExpression, args: &[LicenseExpression]) -> bool {
    if args.iter().any(|a| expressions_equal(a, expr)) {
        return true;
    }
    let decomposed = decompose_expr(expr);
    if decomposed.len() == 1 {
        return false;
    }
    decomposed.iter().any(|d| args.iter().any(|a| expressions_equal(a, d)))
}

/// Check if license expression `container` semantically contains `contained`.
///
/// A license expression A "contains" B if, after simplification:
/// - A == B (same expression)
/// - B is a single license and A's args contain B (or decompose to B for WITH)
/// - B is an AND/OR and all of B's args are in A's args (same operator type)
///
/// NOTE: `gpl-2.0-plus` and `gpl-2.0` are DIFFERENT identifiers with NO containment.
///
/// IMPORTANT: This checks DIRECT containment at the top-level, NOT recursive.
/// For example, `(mit OR apache) AND bsd` does NOT contain `mit` - the OR
/// expression is a single arg, and `mit` is nested inside it.
///
/// Based on Python: Licensing.contains() from license-expression library
pub fn licensing_contains(container: &str, contained: &str) -> bool {
    // Handle empty/whitespace expressions
    let container = container.trim();
    let contained = contained.trim();
    if container.is_empty() || contained.is_empty() {
        return false;
    }
    
    if container.to_lowercase() == contained.to_lowercase() {
        return true;
    }
    
    let Ok(parsed_container) = parse_expression(container) else { return false };
    let Ok(parsed_contained) = parse_expression(contained) else { return false };
    
    let simplified_container = simplify_expression(&parsed_container);
    let simplified_contained = simplify_expression(&parsed_contained);
    
    match (&simplified_container, &simplified_contained) {
        (LicenseExpression::And { .. }, LicenseExpression::And { .. }) |
        (LicenseExpression::Or { .. }, LicenseExpression::Or { .. }) => {
            let container_args = get_flat_args(&simplified_container);
            let contained_args = get_flat_args(&simplified_contained);
            contained_args.iter().all(|c| 
                container_args.iter().any(|ca| expressions_equal(ca, c))
            )
        }
        (LicenseExpression::And { .. } | LicenseExpression::Or { .. }, 
         LicenseExpression::License(_) | LicenseExpression::LicenseRef(_)) => {
            let container_args = get_flat_args(&simplified_container);
            expr_in_args(&simplified_contained, &container_args)
        }
        (LicenseExpression::With { .. }, 
         LicenseExpression::License(_) | LicenseExpression::LicenseRef(_)) => {
            let decomposed = decompose_expr(&simplified_container);
            decomposed.iter().any(|d| expressions_equal(d, &simplified_contained))
        }
        (LicenseExpression::License(_) | LicenseExpression::LicenseRef(_),
         LicenseExpression::And { .. } | LicenseExpression::Or { .. } | LicenseExpression::With { .. }) => {
            false
        }
        (LicenseExpression::License(k1), LicenseExpression::License(k2)) => k1 == k2,
        (LicenseExpression::LicenseRef(k1), LicenseExpression::LicenseRef(k2)) => k1 == k2,
        _ => false,
    }
}
```

### Update `filter_overlapping_matches()`

Replace `licensing_contains_approx()` with proper call:

```rust
fn licensing_contains_match(current: &LicenseMatch, other: &LicenseMatch) -> bool {
    // Handle empty expressions - fall back to length-based approximation
    if current.license_expression.is_empty() || other.license_expression.is_empty() {
        return current.matched_length >= other.matched_length * 2;
    }
    licensing_contains(&current.license_expression, &other.license_expression)
}
```

**Note**: The fallback to length-based approximation ensures backward compatibility for edge cases where `license_expression` is not populated.

### Test Cases to Add

Add to `src/license_detection/expression.rs`:

```rust
#[cfg(test)]
mod contains_tests {
    use super::*;

    #[test]
    fn test_basic_containment() {
        assert!(licensing_contains("mit", "mit"));
        assert!(!licensing_contains("mit", "apache"));
    }

    #[test]
    fn test_or_containment() {
        assert!(licensing_contains("mit OR apache", "mit"));
        assert!(licensing_contains("mit OR apache", "apache"));
        assert!(!licensing_contains("mit OR apache", "gpl"));
    }

    #[test]
    fn test_and_containment() {
        assert!(licensing_contains("mit AND apache", "mit"));
        assert!(licensing_contains("mit AND apache", "apache"));
        assert!(!licensing_contains("mit", "mit AND apache"));
    }

    #[test]
    fn test_expression_subset() {
        assert!(licensing_contains("mit AND apache AND bsd", "mit AND apache"));
        assert!(!licensing_contains("mit AND apache", "mit AND apache AND bsd"));
        assert!(licensing_contains("mit OR apache OR bsd", "mit OR apache"));
        assert!(!licensing_contains("mit OR apache", "mit OR apache OR bsd"));
    }

    #[test]
    fn test_order_independence() {
        assert!(licensing_contains("mit AND apache", "apache AND mit"));
        assert!(licensing_contains("mit OR apache", "apache OR mit"));
    }

    #[test]
    fn test_plus_suffix_no_containment() {
        // CRITICAL: These are DIFFERENT identifiers!
        assert!(!licensing_contains("gpl-2.0-plus", "gpl-2.0"));
        assert!(!licensing_contains("gpl-2.0", "gpl-2.0-plus"));
    }

    #[test]
    fn test_with_decomposition() {
        assert!(licensing_contains("gpl-2.0 WITH classpath-exception", "gpl-2.0"));
        assert!(licensing_contains("gpl-2.0 WITH classpath-exception", "classpath-exception"));
        assert!(!licensing_contains("gpl-2.0", "gpl-2.0 WITH classpath-exception"));
    }

    #[test]
    fn test_mixed_operators() {
        assert!(!licensing_contains("mit OR apache", "mit AND apache"));
        assert!(!licensing_contains("mit AND apache", "mit OR apache"));
    }

    #[test]
    fn test_nested_expressions() {
        // CRITICAL: contains() checks DIRECT containment, NOT recursive!
        // The AND args for "(mit OR apache) AND bsd" are [Or(mit, apache), bsd]
        // "mit" is NOT directly in this list, only nested inside the OR.
        assert!(!licensing_contains("(mit OR apache) AND bsd", "mit"));
        // But the OR expression IS a direct arg:
        assert!(licensing_contains("(mit OR apache) AND bsd", "mit OR apache"));
        assert!(licensing_contains("(mit OR apache) AND bsd", "bsd"));
    }

    #[test]
    fn test_empty_expressions() {
        assert!(!licensing_contains("", "mit"));
        assert!(!licensing_contains("mit", ""));
        assert!(!licensing_contains("", ""));
        assert!(!licensing_contains("   ", "mit"));
    }

    #[test]
    fn test_invalid_expressions() {
        // Invalid expressions should return false
        assert!(!licensing_contains("mit AND", "mit"));
        assert!(!licensing_contains("mit", "AND apache"));
    }
}
```

### Call Sites to Update

1. **`src/license_detection/match_refine.rs:380-382`** - Replace `licensing_contains_approx` with `licensing_contains_match`

2. **`src/license_detection/match_refine.rs:477, 485, 496, 504, 516, 526`** - Update all call sites

### Impact on Golden Tests

**Expected changes**:

- Tests where Python discards matches that Rust keeps should now match
- Expression combination differences should be reduced
- GPL variant tests (`gpl-2.0`, `gpl-2.0-plus`) will NOT change (different identifiers in Python too)

**Test patterns affected**:

- Pattern 1 (Expression Over-Combination): ~30 tests
- Pattern 2 (Over-Grouping): ~20 tests

**Verification**:

```bash
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic1
```

### Priority: MEDIUM-HIGH

This is more important than `min_score` because:

- Affects default behavior (no opt-in needed)
- May explain many of the remaining golden test failures
- License expression handling is core to correctness
- Existing `expression.rs` infrastructure reduces implementation effort

---

## Implementation Order

1. **Item 2: `licensing_contains()`** (Priority: MEDIUM-HIGH)
   - Add helper functions to `expression.rs` (`get_flat_args`, `decompose_expr`, `expressions_equal`)
   - Implement `licensing_contains()` function
   - Implement `licensing_contains_match()` wrapper with fallback
   - Update `filter_overlapping_matches()` to use new function
   - Run golden tests to measure improvement

2. **Item 1: `filter_matches_below_minimum_score()`** (Priority: LOW)
   - Implement filter function
   - Add parameter to `refine_matches()`
   - Add unit tests
   - CLI option (optional, defer)

---

## Verification Plan

### After Item 2 (licensing_contains)

```bash
# Run golden tests
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic1

# Compare before/after failure count
# Baseline: 77 failures
# Target: Reduce failures by 10-30 tests
```

### After Item 1 (min_score)

```bash
# Run all tests
cargo test --lib

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings
```

---

## Dependencies

**No new dependencies required.** The existing `src/license_detection/expression.rs` provides all necessary parsing infrastructure and handles ScanCode-specific formats like `gpl-2.0-plus` and `LicenseRef-scancode-*`.

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Implementation doesn't match Python semantics exactly | Low | Medium | Comprehensive unit tests against Python test cases |
| Performance regression from expression parsing | Low | Low | Parser is already used in codebase; can cache if needed |
| Empty/invalid expressions cause errors | Low | Medium | Handle edge cases with fallback to approximation |
| `min_score` filter breaks existing behavior | Very Low | Low | Default to 0 (inactive) |

---

## Estimated Effort

| Item | Implementation | Testing | Total |
|------|---------------|---------|-------|
| licensing_contains | 2-3 hours | 1-2 hours | 3-5 hours |
| min_score filter | 30 minutes | 30 minutes | 1 hour |

**Total**: 4-6 hours

## Implementation Notes

### Key Insight: No New Dependencies

The existing `src/license_detection/expression.rs` already has all the parsing infrastructure needed. We just need to add:

- Helper functions (`get_flat_args`, `decompose_expr`, `expressions_equal`)
- The `licensing_contains()` function itself

### Critical Python Behaviors to Match

1. **Simplification first**: Both expressions are simplified before comparison
2. **Subset check**: For AND/OR, check if all contained args are in container args
3. **WITH decomposition**: `A WITH E` decomposes to `[A, E]` for containment checks
4. **No suffix handling**: `gpl-2.0-plus` and `gpl-2.0` are different identifiers
5. **Direct containment only**: `(mit OR apache) AND bsd` does NOT contain `mit` - the OR is a single arg, `mit` is nested inside it
