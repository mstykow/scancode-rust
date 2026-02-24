# PLAN-040: Expression Normalization Implementation

**Date**: 2026-02-23
**Status**: CLOSED - No Implementation Needed
**Priority**: N/A (Plan Closed)
**Related**: PLAN-029 Section 2.6, PLAN-010, PLAN-027
**Estimated Effort**: N/A (No implementation required)

---

## Closure Summary

### Why This Was Investigated

This plan was created based on analysis in PLAN-029 Section 2.6, which hypothesized that differences in license expression normalization between Python and Rust were causing ~50+ golden test failures. The initial assumption was that Python's `simplify()` method (with boolean absorption, elimination, and canonical sorting) was being applied during detection, and that Rust's simpler deduplication-only approach was causing output differences.

### Why No Implementation Is Needed

**The hypothesis was incorrect.** After thorough verification of the Python codebase:

1. **Python does NOT apply `simplify()` during detection** - The `simplify()` method is only called in summary/post-processing code (`summarycode/summarizer.py`, `score.py`, `plugin_consolidate.py`), never in the detection phase.

2. **Python's detection uses simple deduplication** - During detection, Python only uses `combine_expressions(unique=True)` which performs string-based deduplication, exactly matching Rust's current `simplify_expression()` behavior.

3. **Rust's current implementation is correct** - Rust's `simplify_expression()` is functionally equivalent to Python's `combine_expressions(unique=True)` for the detection use case.

### The Actual Root Cause

**The golden test failures are caused by match detection and grouping differences, NOT expression normalization:**

| Issue | Root Cause | Evidence |
|-------|------------|----------|
| Fewer matches detected | Match detection logic differs | `gpl-2.0_82.RULE`: Expected 3 matches, got 1 |
| Separate vs combined detections | Match grouping logic differs | `gpl_and_lgpl_and_gfdl-1.2.txt`: Expected combined expression, got separate |
| Match count mismatches | Detection aggregation differs | Tests compare individual match expressions, not combined |

**The focus should shift to investigating match detection and grouping logic**, not expression normalization.

---

## Executive Summary

**VERIFICATION RESULT**: The Python analysis in this plan is accurate, but the root cause attribution is incorrect. The ~50+ golden test failures are NOT caused by expression normalization differences. They are caused by differences in match detection and match grouping logic.

**Key Findings After Verification**:

1. Python's `simplify()` is NOT called during detection - only in post-processing
2. Rust's `simplify_expression()` is equivalent to Python's `combine_expressions(unique=True)` for detection
3. Golden tests compare individual match expressions, NOT combined detection expressions
4. Most failures are match detection/grouping issues, not expression normalization

**Recommendation**: **PLAN CLOSED** - No implementation needed. Rust's expression handling matches Python's detection-phase behavior. Focus should shift to match detection and grouping logic (separate investigation required).

---

## 0. Verification Report (2026-02-23)

> **KEY FINDING**: Expression normalization is NOT the root cause of golden test failures. This plan is closed with no implementation needed.

### 0.1 Python Analysis Verification

**Claim**: Python's `simplify()` is only used in summary/post-processing.
**Status**: **VERIFIED CORRECT**

Evidence from `reference/scancode-toolkit/src/licensedcode/detection.py`:

- Line 451: `combine_expressions([...], unique=True, licensing=licensing)`
- Line 882: `combine_expressions(expressions=..., relation='AND', unique=True, ...)`
- Line 1594: `combine_expressions(expressions=[match.rule.license_expression...], licensing=...)`
- Line 2000: `combine_expressions(expressions=..., relation='AND', unique=True, ...)`

All detection-phase calls use `combine_expressions()` with `unique=True`, which performs deduplication only.

The `.simplify()` method is only called in `summarycode/`:

- `summarizer.py:259`: `Licensing().parse(combined_declared_license_expression).simplify()`
- `score.py:191`: `Licensing().parse(combined_declared_license_expression).simplify()`
- `plugin_consolidate.py:80`: `Licensing().parse(combined_license_expression).simplify()`

### 0.2 Rust Analysis Verification

**Claim**: Rust's `simplify_expression()` is equivalent to Python's detection-phase behavior.
**Status**: **VERIFIED CORRECT**

Evidence from `src/license_detection/expression.rs`:

- `combine_expressions()` (lines 628-666) calls `simplify_expression()` when `unique=true`
- `simplify_expression()` (lines 212-233) performs deduplication within AND/OR chains
- This matches Python's `combine_expressions(unique=True)` behavior

### 0.3 Golden Test Failure Analysis

**Claim**: Expression normalization causes ~30+ golden test failures.
**Status**: **INCORRECT - Root cause is different**

The golden test at `src/license_detection/golden_test.rs:176-180` compares:

```rust
let actual: Vec<&str> = detections
    .iter()
    .flat_map(|d| d.matches.iter())
    .map(|m| m.license_expression.as_str())
    .collect();
```

This compares **individual match expressions**, NOT combined detection expressions.

Example failure analysis:

- `gpl-2.0_82.RULE`: Expected 3 `gpl-2.0` matches, got 1 match
- `gpl_and_lgpl_and_gfdl-1.2.txt`: Expected `gpl-1.0-plus AND lgpl-2.0-plus AND gfdl-1.2` (combined), got 3 separate expressions

The issue is **match grouping and detection logic**, not expression normalization.

### 0.4 License Equivalence Verification

**Claim**: Python uses `key_aliases` for license equivalence in expressions.
**Status**: **PARTIALLY CORRECT**

Evidence:

- `key_aliases` exists in `models.py:282-287` as a field on License objects
- However, `grep` shows it's only referenced in that one location
- `build_spdx_license_expression()` in `cache.py:507-524` uses SPDX key mapping, not key_aliases
- No automatic license equivalence transformation was found in expression handling

The `lzma-sdk-2006` example from PLAN-029 appears to be illustrative, not actual behavior.

### 0.5 Recommendations

1. **~~Close this plan as NOT THE ROOT CAUSE~~** - DONE. Plan is CLOSED with no implementation needed.
2. **Investigate match detection/grouping** - This is where the real differences lie (separate investigation required)
3. **Keep Rust's current expression handling** - It matches Python's detection-phase behavior exactly
4. **Consider simplification for summary/post-processing** - Only if needed for summary output parity in the future

---

## 1. Problem Description

### 1.1 Current Behavior (Rust)

Rust's `simplify_expression()` at `src/license_detection/expression.rs:212-233`:

```rust
pub fn simplify_expression(expr: &LicenseExpression) -> LicenseExpression {
    match expr {
        LicenseExpression::License(key) => LicenseExpression::License(key.clone()),
        LicenseExpression::LicenseRef(key) => LicenseExpression::LicenseRef(key.clone()),
        LicenseExpression::With { left, right } => LicenseExpression::With {
            left: Box::new(simplify_expression(left)),
            right: Box::new(simplify_expression(right)),
        },
        LicenseExpression::And { .. } => {
            let mut unique = Vec::new();
            let mut seen = HashSet::new();
            collect_unique_and(expr, &mut unique, &mut seen);
            build_expression_from_list(&unique, true)
        }
        LicenseExpression::Or { .. } => {
            let mut unique = Vec::new();
            let mut seen = HashSet::new();
            collect_unique_or(expr, &mut unique, &mut seen);
            build_expression_from_list(&unique, false)
        }
    }
}
```

**What Rust Does**:

- Deduplicates within same operator (AND or OR)
- Preserves order of first occurrence
- Treats WITH expressions as atomic
- No boolean simplification rules
- No commutativity/sorting

**What Rust Does NOT Do**:

- Absorption: `A AND (A OR B)` -> `A`
- Elimination: `(A AND B) OR (A AND NOT B)` -> `A`
- Idempotence: Already handled for simple cases
- Canonical sorting: Expression order may differ
- License equivalence: No replacement of equivalent licenses

### 1.2 Expected Behavior (Python)

Python uses the `license-expression` library which wraps the `boolean` library:

**From `boolean/boolean.py:1250-1320` (DualBase.simplify)**:

```python
def simplify(self, sort=True):
    """
    Return a new simplified expression in canonical form.

    Rules applied recursively bottom up:
    - Associativity: (A & B) & C = A & B & C
    - Annihilation: A & 0 = 0, A | 1 = 1
    - Idempotence: A & A = A, A | A = A
    - Identity: A & 1 = A, A | 0 = A
    - Complementation: A & ~A = 0, A | ~A = 1
    - Elimination: (A & B) | (A & ~B) = A
    - Absorption: A & (A | B) = A, A | (A & B) = A
    - Negative absorption: A & (~A | B) = A & B
    - Commutativity: Output is always sorted
    """
```

**From `license_expression/__init__.py:707-761` (Licensing.dedup)**:

```python
def dedup(self, expression):
    """
    Return a deduplicated LicenseExpression.
    
    The deduplication:
    - Does not sort licenses. They stay in same order.
    - Choices (OR expressions) are kept as-is and not simplified.
      This avoids dropping important choice options.
    """
    exp = self.parse(expression)
    expressions = []
    for arg in exp.args:
        if isinstance(arg, (self.AND, self.OR)):
            expressions.append(self.dedup(arg))
        else:
            expressions.append(arg)
    
    if isinstance(exp, BaseSymbol):
        deduped = exp
    elif isinstance(exp, (self.AND, self.OR)):
        relation = exp.__class__.__name__
        deduped = combine_expressions(
            expressions,
            relation=relation,
            unique=True,
            licensing=self,
        )
    return deduped
```

**Key Python Behavior Differences**:

| Feature | Python | Rust |
|---------|--------|------|
| Deduplication | Via `dedup()` + `combine_expressions()` | Only in `simplify_expression()` |
| Boolean absorption | Yes (A AND (A OR B) -> A) | No |
| Boolean elimination | Yes | No |
| Order preservation | Yes (in `dedup()`) | Yes |
| Canonical sorting | Yes (in `simplify()`) | No |
| License equivalence | Via `key_aliases` | Not implemented |

---

## 2. Python Reference Analysis

### 2.1 Key Code Locations

| Component | File | Lines | Purpose |
|-----------|------|-------|---------|
| `combine_expressions()` | `detection.py` | 21, 451, 882, 1594 | Combines match expressions |
| `Licensing.dedup()` | `license_expression/__init__.py` | 707-761 | License-aware deduplication |
| `DualBase.simplify()` | `boolean/boolean.py` | 1250-1320 | Boolean algebra simplification |
| `combine_expressions()` | `license_expression/__init__.py` | 1746-1802 | Combines expressions with optional dedup |
| License key aliases | `models.py` | 282-287 | Alternative license keys |

### 2.2 Expression Flow in Python

```
Match Detection
      |
      v
combine_expressions(expressions, relation='AND', unique=True)
      |
      +---> licensing.parse() each expression
      |
      +---> If unique: remove duplicates by string key
      |
      +---> Build combined expression
      |
      v
Licensing.dedup() [if needed]
      |
      +---> Recursively deduplicate nested AND/OR
      |
      v
Licensing.simplify() [if needed - used in summary]
      |
      +---> Full boolean simplification
      |
      +---> Canonical sorting
      |
      v
Final Expression String
```

### 2.3 When Each Function Is Used

**In `detection.py`**:

| Location | Function Called | Parameters |
|----------|----------------|------------|
| Line 451 | `combine_expressions()` | `[self.license_expression, match.license_expression]`, `unique=True` |
| Line 882 | `combine_expressions()` | Detection expressions, `relation='AND'`, `unique=True` |
| Line 1594 | `combine_expressions()` | Match rule expressions, `licensing=get_licensing()` |
| Line 2000 | `combine_expressions()` | Detection expressions, `relation='AND'`, `unique=True` |

**In `summarycode/`** (post-processing):

| Location | Function Called | Purpose |
|----------|----------------|---------|
| `summarizer.py:259` | `.simplify()` | Simplify combined declared license |
| `score.py:191` | `.simplify()` | Simplify combined declared license |
| `plugin_consolidate.py:80` | `.simplify()` | Consolidate license expression |

**Key Insight**: Python calls `simplify()` during summarization/post-processing, NOT during detection. The detection phase uses `combine_expressions()` with `unique=True`, which does deduplication but not full boolean simplification.

---

## 3. Rust Current State Analysis

### 3.1 Expression Handling in Rust

**File**: `src/license_detection/expression.rs`

| Function | Lines | Purpose |
|----------|-------|---------|
| `parse_expression()` | 195-203 | Parse expression string to AST |
| `simplify_expression()` | 212-233 | Deduplicate within AND/OR chains |
| `combine_expressions()` | 628-666 | Combine multiple expressions |
| `expression_to_string()` | 548-550 | Render AST to string |
| `licensing_contains()` | 444-506 | Check expression containment |

### 3.2 Current Simplification Implementation

```rust
// expression.rs:235-277 - AND deduplication
fn collect_unique_and(
    expr: &LicenseExpression,
    unique: &mut Vec<LicenseExpression>,
    seen: &mut HashSet<String>,
) {
    match expr {
        LicenseExpression::And { left, right } => {
            collect_unique_and(left, unique, seen);
            collect_unique_and(right, unique, seen);
        }
        LicenseExpression::Or { .. } => {
            let simplified = simplify_expression(expr);
            let key = expression_to_string(&simplified);
            if !seen.contains(&key) {
                seen.insert(key);
                unique.push(simplified);
            }
        }
        LicenseExpression::With { left, right } => {
            let simplified = LicenseExpression::With {
                left: Box::new(simplify_expression(left)),
                right: Box::new(simplify_expression(right)),
            };
            let key = expression_to_string(&simplified);
            if !seen.contains(&key) {
                seen.insert(key);
                unique.push(simplified);
            }
        }
        LicenseExpression::License(key) => {
            if !seen.contains(key) {
                seen.insert(key.clone());
                unique.push(LicenseExpression::License(key.clone()));
            }
        }
        // ... similar for LicenseRef
    }
}
```

### 3.3 What's Missing

1. **Boolean Absorption**: `A AND (A OR B)` should simplify to `A`
2. **Boolean Elimination**: `(A AND B) OR (A AND NOT B)` should simplify to `A`
3. **Complement Detection**: `A AND NOT A` should simplify to `FALSE` (though rare in licenses)
4. **Canonical Sorting**: For consistent output, expressions should be sorted
5. **License Equivalence**: Some licenses are aliases of others

---

## 4. Specific Differences Found

### 4.1 Case Study: WITH Expression Handling

**Input**: `gpl-2.0 WITH classpath-exception-2.0 AND gpl-2.0`

**Python**: Keeps both, as WITH expression is not contained by base license
**Rust**: Keeps both (correct via `licensing_contains()`)

**Status**: Correctly handled after PLAN-010 implementation

### 4.2 Case Study: Nested OR in AND

**Input**: `(mit OR apache-2.0) AND mit`

**Python with dedup()**: `(mit OR apache-2.0) AND mit` (keeps choices)
**Python with simplify()**: `mit` (absorption: A AND (A OR B) = A)

**Rust**: `(mit OR apache-2.0) AND mit` (no absorption)

**Impact**: Rust produces more verbose expressions but preserves choice semantics

### 4.3 Case Study: License Equivalence

**PLAN-029 mentions**: `lgpl-2.1 WITH exception OR cpl-1.0 WITH exception` -> `lzma-sdk-2006`

**Investigation Result**: This specific transformation was NOT found in the codebase. The `lzma-sdk-2006` reference appears to be an example of potential license equivalence, not actual implemented behavior.

**License equivalence handling in Python**:

- Via `key_aliases` field in License model (`models.py:282-287`)
- Used in `build_spdx_license_expression()` for SPDX key mapping
- NOT used for automatic expression simplification

### 4.4 Actual Differences in Golden Tests

Based on PLAN-029 analysis:

| Pattern | Count | Root Cause |
|---------|-------|------------|
| Expression differs | ~10 | Combination/simplification |
| Duplicate detection | ~8 | Deduplication edge cases |
| Match count mismatch | ~15 | Expression combination |

---

## 5. Proposed Changes

### 5.1 Phase 1: Add Boolean Absorption (Medium Priority)

**File**: `src/license_detection/expression.rs`

Add absorption rule: `A AND (A OR B)` -> `A`

```rust
/// Apply absorption law: A AND (A OR B) = A
/// And: A OR (A AND B) = A
fn apply_absorption(expr: &LicenseExpression) -> LicenseExpression {
    match expr {
        LicenseExpression::And { left, right } => {
            let left_simplified = apply_absorption(left);
            let right_simplified = apply_absorption(right);
            
            // Check if left is contained in right's OR args
            if let LicenseExpression::Or { .. } = right_simplified {
                let left_keys = left_simplified.license_keys();
                let right_args = get_flat_args(&right_simplified);
                if right_args.iter().any(|arg| {
                    arg.license_keys() == left_keys
                }) {
                    return left_simplified;
                }
            }
            // Check if right is contained in left's OR args
            if let LicenseExpression::Or { .. } = left_simplified {
                let right_keys = right_simplified.license_keys();
                let left_args = get_flat_args(&left_simplified);
                if left_args.iter().any(|arg| {
                    arg.license_keys() == right_keys
                }) {
                    return right_simplified;
                }
            }
            
            LicenseExpression::And {
                left: Box::new(left_simplified),
                right: Box::new(right_simplified),
            }
        }
        // Similar for OR absorption
        _ => expr.clone(),
    }
}
```

**Note**: Python does NOT apply absorption during detection - only during summarization. This phase may be optional for parity.

### 5.2 Phase 2: Canonical Expression Sorting (Low Priority)

**File**: `src/license_detection/expression.rs`

Sort expression arguments for consistent output:

```rust
fn sort_expression_args(expr: &mut LicenseExpression) {
    match expr {
        LicenseExpression::And { left, right } => {
            sort_expression_args(left);
            sort_expression_args(right);
            // Sort by license key string for canonical form
        }
        LicenseExpression::Or { left, right } => {
            sort_expression_args(left);
            sort_expression_args(right);
        }
        _ => {}
    }
}
```

**Note**: Python's `dedup()` explicitly does NOT sort, but `simplify()` does. This should be optional.

### 5.3 Phase 3: License Equivalence (Low Priority)

**File**: `src/license_detection/license_db.rs` or new file

Add license equivalence/alias handling:

```rust
/// License equivalence mapping
/// Maps a license key to its canonical form
pub fn get_canonical_license_key(key: &str) -> &str {
    // Example: "gpl-2.0+" -> "gpl-2.0-plus"
    // Example: "GPL-2.0-or-later" -> "gpl-2.0-plus"
    match key.to_lowercase().as_str() {
        "gpl-2.0+" | "gpl-2.0-or-later" => "gpl-2.0-plus",
        "gpl-3.0+" | "gpl-3.0-or-later" => "gpl-3.0-plus",
        _ => key,
    }
}
```

**Note**: This is already partially handled via SPDX key mapping (PLAN-032).

### 5.4 Phase 4: Expression Normalization Pipeline (Optional)

**File**: `src/license_detection/expression.rs`

Add a normalization function that combines all transformations:

```rust
/// Normalize a license expression.
/// This is equivalent to Python's simplify() for post-processing.
pub fn normalize_expression(expr: &LicenseExpression) -> LicenseExpression {
    let mut result = expr.clone();
    
    // Step 1: Deduplicate (already in simplify_expression)
    result = simplify_expression(&result);
    
    // Step 2: Apply absorption (Phase 1)
    result = apply_absorption(&result);
    
    // Step 3: Sort for canonical form (Phase 2, optional)
    // sort_expression_args(&mut result);
    
    result
}
```

---

## 6. Test Requirements

### 6.1 Unit Tests for Absorption

```rust
#[test]
fn test_absorption_and_or() {
    // A AND (A OR B) -> A
    let expr = parse_expression("mit AND (mit OR apache-2.0)").unwrap();
    let result = apply_absorption(&expr);
    assert_eq!(expression_to_string(&result), "mit");
}

#[test]
fn test_absorption_or_and() {
    // A OR (A AND B) -> A
    let expr = parse_expression("mit OR (mit AND apache-2.0)").unwrap();
    let result = apply_absorption(&expr);
    assert_eq!(expression_to_string(&result), "mit");
}

#[test]
fn test_no_absorption_different_keys() {
    // A AND (B OR C) should NOT absorb
    let expr = parse_expression("mit AND (apache-2.0 OR gpl-2.0)").unwrap();
    let result = apply_absorption(&expr);
    assert_eq!(
        expression_to_string(&result),
        "mit AND (apache-2.0 OR gpl-2.0)"
    );
}
```

### 6.2 Golden Test Verification

Run specific tests to verify expression normalization:

```bash
# Tests that may be affected by normalization
cargo test --release -q --lib license_detection::golden_tests -- --test-threads=1 2>&1 | \
    grep -E "gpl-2.0_plus|fsf-free|crapl" | head -20
```

### 6.3 Integration Tests

Compare Python and Rust output for specific expressions:

| Input Expression | Python `dedup()` | Python `simplify()` | Rust Current | Rust Target |
|-----------------|------------------|--------------------|--------------|-------------|
| `MIT AND MIT` | `mit` | `mit` | `mit` | `mit` |
| `MIT OR MIT` | `mit` | `mit` | `mit` | `mit` |
| `MIT AND (MIT OR Apache)` | `mit AND (mit OR apache)` | `mit` | `mit AND (mit OR apache)` | `mit AND (mit OR apache)` (parity) |
| `(MIT AND GPL) OR (MIT AND GPL)` | `mit AND gpl` | `mit AND gpl` | `mit AND gpl` | `mit AND gpl` |
| `GPL-2.0+ AND gpl-2.0-plus` | `gpl-2.0-plus AND gpl-2.0-plus` | `gpl-2.0-plus` | `gpl-2.0+ AND gpl-2.0-plus` | `gpl-2.0-plus` (with equivalence) |

---

## 7. Risk Assessment

### 7.1 High Risk

| Risk | Mitigation |
|------|------------|
| Breaking existing correct behavior | Comprehensive test coverage before changes |
| Over-simplification losing license choices | Follow Python's `dedup()` behavior (no choice simplification) |
| Performance impact on large expressions | Benchmark before/after, make optional |

### 7.2 Medium Risk

| Risk | Mitigation |
|------|------------|
| Canonical sorting changing output order | Make sorting optional via flag |
| License equivalence missing edge cases | Use Python's license index as reference |

### 7.3 Low Risk

| Risk | Mitigation |
|------|------------|
| Differences in edge cases | Document intentional differences |

---

## 8. Implementation Priority

### Priority Assessment (Updated After Verification)

> **STATUS: PLAN CLOSED** - No implementation required. Expression normalization was not the root cause.

Based on the verification analysis:

**Key Question**: Does Python actually apply boolean simplification during detection?

**Answer**: **No, confirmed.** Python's `detection.py` only uses `combine_expressions()` with `unique=True`, which performs deduplication. Boolean `simplify()` is only used in the summary/post-processing phase.

**However**: The golden test failures are NOT caused by expression normalization. They are caused by:

1. Different match detection results (more/fewer matches)
2. Different match grouping into detections
3. Different handling of duplicate/overlapping matches

### Recommended Approach (Updated)

1. **~~CLOSE THIS PLAN~~** - **DONE**. Expression normalization is not the root cause
2. **INVESTIGATE MATCH DETECTION** - The real differences are in:
   - How matches are discovered and scored
   - How matches are grouped into detections
   - How detection expressions are constructed from match groups
3. **IF SIMPLIFICATION IS NEEDED** - Only implement for summary/post-processing output

### Phase Order (Updated)

| Phase | Priority | Estimated Tests Fixed | Risk | Status |
|-------|----------|----------------------|------|--------|
| ~~Close this plan~~ | N/A | 0 | None | **CLOSED** |
| Investigate match grouping | Critical | ~50+ | Low | **NEEDED** |
| License equivalence | Low | ~5-10 | Medium | Optional |
| Boolean absorption | Very Low | ~0-5 | Medium | Optional |
| Canonical sorting | Very Low | ~0 | Low | Optional |

---

## 9. Action Items

### Immediate (Updated) - ALL COMPLETE

- [x] Run golden tests with verbose output to identify actual expression-related failures
- [x] Compare Python's `combine_expressions()` output with Rust's for same inputs
- [x] Verify Python does NOT apply `simplify()` during detection phase
- [x] **CONCLUSION: Expression normalization is NOT the root cause**
- [x] **PLAN CLOSED with no implementation needed**

### Next Steps (Updated) - Separate Investigation Required

- [ ] Create new plan for match detection/grouping investigation
- [ ] Investigate match detection differences between Python and Rust
- [ ] Compare match grouping logic (how matches become detections)
- [ ] Analyze why some tests expect combined expressions vs separate match expressions
- [ ] Document intentional differences in match/detection behavior

### Implementation (Lower Priority) - OPTIONAL, Not Required for Parity

- [ ] Add license equivalence mapping based on SPDX data (optional)
- [ ] Add absorption rules for summary/post-processing (optional)
- [ ] Add canonical sorting option (optional)
- [ ] Update tests (only if above implemented)

### Documentation

- [x] Document verification findings in this plan
- [x] Update plan status to CLOSED
- [ ] Create new plan for match detection/grouping investigation
- [ ] Update PLAN-029 with correct root cause

---

## 10. Conclusion (Updated After Verification)

> **PLAN STATUS: CLOSED - No Implementation Needed**
>
> Expression normalization was thoroughly investigated and confirmed NOT to be the root cause of golden test failures.

The expression normalization issue described in PLAN-029 Section 2.6 has been **thoroughly investigated**:

### Verified Findings

1. **Python does NOT perform full boolean simplification during detection** - only deduplication via `combine_expressions(unique=True)`. **VERIFIED CORRECT.**

2. **Rust's current `simplify_expression()` is equivalent to Python's `combine_expressions(unique=True)`** for the deduplication case. **VERIFIED CORRECT.**

3. **The `lzma-sdk-2006` example from PLAN-029 was illustrative**, not actual behavior. **VERIFIED CORRECT.**

4. **License equivalence handling (`key_aliases`) is NOT used for automatic expression simplification**. **VERIFIED CORRECT.**

### Root Cause Correction

1. **The golden test failures are NOT caused by expression normalization.** They are caused by differences in:
   - Match detection (different number of matches found)
   - Match grouping (how matches become detections)
   - Match expression collection (tests expect individual match expressions, not combined detection expressions)

### Final Decision

**PLAN CLOSED** without implementation. Expression normalization is working correctly for detection parity. The focus should shift to investigating match detection and grouping differences, which are the actual root cause of the golden test failures.

This plan remains valuable as documentation of Python's expression handling behavior and could be referenced if summary/post-processing output parity becomes a requirement in the future.

---

## Appendix A: Python Code References

### A.1 combine_expressions() in license_expression/**init**.py

```python
# Lines 1746-1802
def combine_expressions(
    expressions,
    relation="AND",
    unique=True,
    licensing=Licensing(),
):
    """Combine expressions with optional deduplication."""
    if not expressions:
        return

    expressions = [licensing.parse(le, simple=True) for le in expressions]

    if unique:
        # Remove duplicates, preserve order
        expressions = list({str(x): x for x in expressions}.values())

    if len(expressions) == 1:
        return expressions[0]

    relation = {"AND": licensing.AND, "OR": licensing.OR}[relation]
    return relation(*expressions)
```

### A.2 Licensing.dedup() in license_expression/**init**.py

```python
# Lines 707-761
def dedup(self, expression):
    """Deduplicate expression, preserving order and choices."""
    exp = self.parse(expression)
    expressions = []
    for arg in exp.args:
        if isinstance(arg, (self.AND, self.OR)):
            expressions.append(self.dedup(arg))
        else:
            expressions.append(arg)

    if isinstance(exp, BaseSymbol):
        deduped = exp
    elif isinstance(exp, (self.AND, self.OR)):
        relation = exp.__class__.__name__
        deduped = combine_expressions(
            expressions,
            relation=relation,
            unique=True,
            licensing=self,
        )
    return deduped
```

### A.3 Detection Usage in detection.py

```python
# Line 1594 - get_detected_license_expression()
combined_expression = combine_expressions(
    expressions=[match.rule.license_expression for match in matches_for_expression],
    licensing=get_licensing(),
)
# Note: No 'relation' parameter, defaults to AND
# Note: No explicit 'unique' parameter, defaults to True
```

---

## Appendix C: Verification Evidence (2026-02-23)

### C.1 Python Detection Code Evidence

From `reference/scancode-toolkit/src/licensedcode/detection.py`:

```python
# Line 21 - Import
from license_expression import combine_expressions

# Line 451 - Detection.append()
license_expression = combine_expressions(
    [self.license_expression, match.license_expression],
    unique=True,
    licensing=licensing,
)

# Line 882 - combine_detection_license_expressions()
license_expression_from_detections = str(combine_expressions(
    expressions=license_expressions_from_detections,
    relation='AND',
    unique=True,
    licensing=get_licensing(),
))

# Line 1594 - get_detected_license_expression()
combined_expression = combine_expressions(
    expressions=[match.rule.license_expression for match in matches_for_expression],
    licensing=get_licensing(),
)
# Note: No explicit 'unique' parameter, defaults to True

# Line 2000 - update_detected_license_expression()
detected_license_expression = combine_expressions(
    expressions=license_expressions,
    relation='AND',
    unique=True,
    licensing=get_cache().licensing)
```

### C.2 Python Summary Code Evidence

From `reference/scancode-toolkit/src/summarycode/`:

```python
# summarizer.py:258-260
declared_license_expression = str(
    Licensing().parse(combined_declared_license_expression).simplify()
)

# score.py:190-191
declared_license_expression = str(
    Licensing().parse(combined_declared_license_expression).simplify()
)

# plugin_consolidate.py:80
self.consolidated_license_expression = str(
    Licensing().parse(combined_license_expression).simplify()
)
```

### C.3 Golden Test Failure Examples

```
gpl-2.0_82.RULE:
  Expected: ["gpl-2.0", "gpl-2.0", "gpl-2.0"]
  Actual:   ["gpl-2.0"]
  Issue: Match detection/grouping - fewer matches detected

gpl_and_lgpl_and_gfdl-1.2.txt:
  Expected: ["gpl-1.0-plus AND lgpl-2.0-plus AND gfdl-1.2"]
  Actual:   ["gpl-1.0-plus", "lgpl-2.0-plus", "gfdl-1.2"]
  Issue: Match grouping - separate detections instead of combined

gpl-2.0_complex.txt:
  Expected: ["gpl-2.0", "gpl-2.0"]
  Actual:   ["gpl-2.0"]
  Issue: Match detection - fewer matches detected
```

### C.4 Rust Expression Code Evidence

From `src/license_detection/expression.rs`:

```rust
// Lines 628-666 - combine_expressions()
pub fn combine_expressions(
    expressions: &[&str],
    relation: CombineRelation,
    unique: bool,
) -> Result<String, ParseError> {
    // ...
    let final_expr = if unique {
        simplify_expression(&expr)  // Same as Python's unique=True
    } else {
        expr
    };
    // ...
}

// Lines 212-233 - simplify_expression()
pub fn simplify_expression(expr: &LicenseExpression) -> LicenseExpression {
    match expr {
        LicenseExpression::And { .. } => {
            // Deduplicates within AND chains
            let mut unique = Vec::new();
            let mut seen = HashSet::new();
            collect_unique_and(expr, &mut unique, &mut seen);
            build_expression_from_list(&unique, true)
        }
        LicenseExpression::Or { .. } => {
            // Deduplicates within OR chains
            let mut unique = Vec::new();
            let mut seen = HashSet::new();
            collect_unique_or(expr, &mut unique, &mut seen);
            build_expression_from_list(&unique, false)
        }
        // ...
    }
}
```

### C.5 Golden Test Comparison Logic

From `src/license_detection/golden_test.rs:176-180`:

```rust
let actual: Vec<&str> = detections
    .iter()
    .flat_map(|d| d.matches.iter())  // Gets individual match expressions
    .map(|m| m.license_expression.as_str())
    .collect();
```

This compares individual match expressions, NOT combined detection expressions. The test validates that the same matches are detected with the same expressions, not that the final combined detection expression is the same.

---

## Appendix D: Document History

| Date | Author | Changes |
|------|--------|---------|
| 2026-02-23 | AI Agent | Initial plan creation from PLAN-029 analysis |
| 2026-02-23 | AI Agent | Verification complete: Root cause corrected, priority lowered, added Appendix C evidence |
| 2026-02-23 | AI Agent | **PLAN CLOSED**: Added closure summary, confirmed no implementation needed, root cause is match detection/grouping (not expressions) |
