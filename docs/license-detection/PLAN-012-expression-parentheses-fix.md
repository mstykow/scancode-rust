# PLAN-012: Fix Parentheses Logic in `expression_to_string`

**Status**: Implemented  
**Priority**: High  
**Component**: License Expression Rendering  
**Related**: Python parity, Golden test alignment

## 1. Problem Statement

The `expression_to_string` function in `src/license_detection/expression.rs` incorrectly adds parentheses around WITH expressions when the parent operator is OR or AND. This causes license expressions like `GPL-2.0 WITH Classpath-exception-2.0 OR MIT` to be incorrectly rendered as `(GPL-2.0 WITH Classpath-exception-2.0) OR MIT`.

### Impact

- **3 unit tests** have incorrect expected values that don't match Python behavior
- **Golden tests** may fail when comparing against Python ScanCode reference output
- License expression rendering does not match the Python `license-expression` library

### Root Cause

WITH expressions are treated as **atomic symbols** in Python (via `LicenseWithExceptionSymbol`), not as compound expressions. They should never have outer parentheses added based on parent precedence.

The current `!=` comparison logic for WITH is incorrect:

```rust
// Current WRONG logic (line 331):
if parent_prec.is_some_and(|p| p != Precedence::With) {
    format!("({})", result)  // Incorrectly adds parens for WITH inside OR/AND
}
```

---

## 2. Python Reference Analysis

### 2.1 Precedence Rules

| Operator | Precedence | Binding Strength |
|----------|------------|------------------|
| WITH     | 3 (highest) | Tightest binding |
| AND      | 2          | Medium binding   |
| OR       | 1 (lowest) | Loosest binding  |

### 2.2 Python's `isliteral` Logic

In Python's `license-expression` library:

- `Symbol.isliteral = True` - symbols are atomic
- `Function.isliteral = False` - AND/OR are compound
- `LicenseWithExceptionSymbol` extends `Symbol`, so it behaves as a **literal** (atomic)

When rendering, Python checks:

```python
if arg.isliteral:
    rendered_items_append(rendered)  # No parentheses
else:
    rendered_items_append(f"({rendered})")  # Add parentheses
```

### 2.3 Python Rendering Examples

| Input | Output | Explanation |
|-------|--------|-------------|
| `MIT OR Apache-2.0 AND GPL-2.0` | `MIT OR (Apache-2.0 AND GPL-2.0)` | AND inside OR needs parens |
| `MIT AND Apache-2.0 OR GPL-2.0` | `(MIT AND Apache-2.0) OR GPL-2.0` | AND inside OR needs parens |
| `GPL-2.0 WITH Classpath-exception-2.0 OR MIT` | `GPL-2.0 WITH Classpath-exception-2.0 OR MIT` | WITH is atomic, NO parens |
| `GPL-2.0 WITH Classpath-exception-2.0 AND MIT` | `GPL-2.0 WITH Classpath-exception-2.0 AND MIT` | WITH is atomic, NO parens |
| `MIT OR GPL-2.0 WITH Classpath-exception-2.0` | `MIT OR GPL-2.0 WITH Classpath-exception-2.0` | WITH is atomic, NO parens |
| `MIT AND GPL-2.0 WITH Classpath-exception-2.0` | `MIT AND GPL-2.0 WITH Classpath-exception-2.0` | WITH is atomic, NO parens |
| `(MIT OR Apache-2.0) WITH exception` | `(MIT OR Apache-2.0) WITH exception` | OR inside WITH needs parens |

### 2.4 Key Insight

**WITH expressions are atomic symbols** - they should NEVER have outer parentheses added based on parent precedence. This matches Python's treatment of `LicenseWithExceptionSymbol` as a literal.

---

## 3. Rust Code Analysis

### 3.1 Current Implementation

Location: `src/license_detection/expression.rs:300-338`

```rust
fn expression_to_string_internal(
    expr: &LicenseExpression,
    parent_prec: Option<Precedence>,
) -> String {
    match expr {
        // ... License and LicenseRef cases ...
        LicenseExpression::And { left, right } => {
            // ...
            if parent_prec.is_some_and(|p| p != Precedence::And) {
                format!("({})", result)  // Correct: adds parens when inside OR or WITH
            } else {
                result
            }
        }
        LicenseExpression::Or { left, right } => {
            // ...
            if parent_prec.is_some_and(|p| p != Precedence::Or) {
                format!("({})", result)  // Correct: adds parens when inside AND or WITH
            } else {
                result
            }
        }
        LicenseExpression::With { left, right } => {
            // ...
            if parent_prec.is_some_and(|p| p != Precedence::With) {  // BUG!
                format!("({})", result)  // Wrong: adds parens when inside OR or AND
            } else {
                result
            }
        }
    }
}
```

### 3.2 The Bug

For WITH inside OR:

1. `parent_prec = Some(Precedence::Or)`
2. Condition: `Or != With` = **true**
3. Result: Incorrectly adds parentheses: `(gpl-2.0 WITH classpath-exception-2.0) OR mit`

### 3.3 What's Correct

- **AND/OR branches**: The `!=` logic is correct. They need parentheses when inside each other or inside WITH.
- **WITH branch**: Should NEVER add outer parentheses. WITH is treated as an atomic symbol.

---

## 4. Proposed Changes

### 4.1 Code Change (Single Line Fix)

Remove the parentheses logic from the WITH branch:

**File**: `src/license_detection/expression.rs:327-336`

```rust
// BEFORE (incorrect):
LicenseExpression::With { left, right } => {
    let left_str = expression_to_string_internal(left, Some(Precedence::With));
    let right_str = expression_to_string_internal(right, Some(Precedence::With));
    let result = format!("{} WITH {}", left_str, right_str);
    if parent_prec.is_some_and(|p| p != Precedence::With) {
        format!("({})", result)
    } else {
        result
    }
}

// AFTER (correct):
LicenseExpression::With { left, right } => {
    let left_str = expression_to_string_internal(left, Some(Precedence::With));
    let right_str = expression_to_string_internal(right, Some(Precedence::With));
    format!("{} WITH {}", left_str, right_str)
}
```

### 4.2 Why This Works

- WITH is the highest precedence operator (tightest binding)
- WITH expressions are atomic (like symbols in Python)
- Nothing can "tear apart" a WITH expression, so no outer parentheses are ever needed
- The children of WITH (left/right) still get proper parentheses handling via recursion

---

## 5. Testing Strategy

### 5.1 Tests to Modify

The following tests have incorrect expected values:

| Test | File Line | Current (Wrong) | Expected (Python) |
|------|-----------|-----------------|-------------------|
| `test_expression_to_string_with_inside_or` | 1006-1009 | `(gpl-2.0 WITH classpath-exception-2.0) OR mit` | `gpl-2.0 WITH classpath-exception-2.0 OR mit` |
| `test_expression_to_string_with_inside_and` | 1024-1027 | `(gpl-2.0 WITH classpath-exception-2.0) AND mit` | `gpl-2.0 WITH classpath-exception-2.0 AND mit` |
| `test_expression_to_string_roundtrip_or_with` | 1070-1074 | `(gpl-2.0 WITH classpath-exception-2.0) OR mit` | `gpl-2.0 WITH classpath-exception-2.0 OR mit` |

### 5.2 Test Changes

```rust
// test_expression_to_string_with_inside_or (line 1006-1009)
// BEFORE:
assert_eq!(
    expression_to_string(&or_expr),
    "(gpl-2.0 WITH classpath-exception-2.0) OR mit"
);
// AFTER:
assert_eq!(
    expression_to_string(&or_expr),
    "gpl-2.0 WITH classpath-exception-2.0 OR mit"
);

// test_expression_to_string_with_inside_and (line 1024-1027)
// BEFORE:
assert_eq!(
    expression_to_string(&and_expr),
    "(gpl-2.0 WITH classpath-exception-2.0) AND mit"
);
// AFTER:
assert_eq!(
    expression_to_string(&and_expr),
    "gpl-2.0 WITH classpath-exception-2.0 AND mit"
);

// test_expression_to_string_roundtrip_or_with (line 1070-1074)
// BEFORE:
let input = "(gpl-2.0 WITH classpath-exception-2.0) OR mit";
let expr = parse_expression(input).unwrap();
let output = expression_to_string(&expr);
assert_eq!(output, "(gpl-2.0 WITH classpath-exception-2.0) OR mit");
// AFTER:
let input = "gpl-2.0 WITH classpath-exception-2.0 OR mit";
let expr = parse_expression(input).unwrap();
let output = expression_to_string(&expr);
assert_eq!(output, "gpl-2.0 WITH classpath-exception-2.0 OR mit");
```

### 5.3 Tests That Should Remain Unchanged

These tests verify correct behavior that should NOT change:

| Test | Expected Behavior |
|------|-------------------|
| `test_expression_to_string_and_inside_or` | `(mit OR apache-2.0) AND gpl-2.0` - OR inside AND needs parens |
| `test_expression_to_string_or_inside_and` | `(mit AND apache-2.0) OR gpl-2.0` - AND inside OR needs parens |
| `test_expression_to_string_and_inside_with` | `(mit AND apache-2.0) WITH exception` - AND inside WITH needs parens |

### 5.4 Verification Commands

```bash
# Run specific tests
cargo test test_expression_to_string_with_inside_or
cargo test test_expression_to_string_with_inside_and
cargo test test_expression_to_string_roundtrip_or_with

# Run all expression tests
cargo test --lib license_detection::expression

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings
```

---

## 6. Implementation Checklist

- [x] Modify WITH branch in `expression_to_string_internal` (remove parentheses logic)
- [x] Update expected value in `test_expression_to_string_with_inside_or`
- [x] Update expected value in `test_expression_to_string_with_inside_and`
- [x] Update test `test_expression_to_string_roundtrip_or_with`
- [x] Run `cargo test --lib` to verify all tests pass
- [x] Run `cargo clippy` to ensure no warnings
- [x] Verify output matches Python reference

---

## 7. Summary

| Item | Details |
|------|---------|
| **Root Cause** | WITH expressions incorrectly add parentheses when inside OR/AND |
| **Fix Location** | `src/license_detection/expression.rs:426-430` |
| **Fix Type** | Remove parentheses logic from WITH branch |
| **Tests Affected** | 3 tests updated with correct expected values |
| **Complexity** | Low - single code block simplification |

---

## 8. Analysis Results

### 8.1 Implementation Verification

The fix has been successfully implemented. The current code at `expression.rs:426-430`:

```rust
LicenseExpression::With { left, right } => {
    let left_str = expression_to_string_internal(left, Some(Precedence::With));
    let right_str = expression_to_string_internal(right, Some(Precedence::With));
    format!("{} WITH {}", left_str, right_str)
}
```

No parentheses logic is present - WITH expressions are rendered as atomic symbols.

### 8.2 Test Results

All 19 `expression_to_string` tests pass:

```text
test license_detection::expression::tests::test_expression_to_string_with_inside_or ... ok
test license_detection::expression::tests::test_expression_to_string_with_inside_and ... ok
test license_detection::expression::tests::test_expression_to_string_roundtrip_or_with ... ok
test license_detection::expression::tests::test_expression_to_string_or_inside_with ... ok
test license_detection::expression::tests::test_expression_to_string_and_inside_with ... ok
```

### 8.3 Edge Cases Verified

| Case | Input | Output | Status |
|------|-------|--------|--------|
| WITH inside OR | `(gpl-2.0 WITH exception) OR mit` | `gpl-2.0 WITH exception OR mit` | Correct |
| WITH inside AND | `(gpl-2.0 WITH exception) AND mit` | `gpl-2.0 WITH exception AND mit` | Correct |
| OR inside WITH | `(mit OR apache) WITH exception` | `(mit OR apache) WITH exception` | Correct |
| AND inside WITH | `(mit AND apache) WITH exception` | `(mit AND apache) WITH exception` | Correct |
| Multiple WITH in OR | `a WITH x OR b WITH y` | `a WITH x OR b WITH y` | Correct |
| WITH on both sides of AND | `a WITH x AND b WITH y` | `a WITH x AND b WITH y` | Correct |

### 8.4 Remaining Golden Test Failures

The FAILURES.md still lists tests with parentheses issues (lines 133, 158-162), but these entries are **stale** - they reflect the pre-fix state. The underlying golden tests may have other issues (grouping, unknown-license-reference, etc.) but the expression rendering fix is complete.

Key remaining golden test issues are unrelated to parentheses:

- Match grouping logic (license intro/clue detection)
- Unknown license reference filtering
- HTML demarkup preprocessing
- License deduplication

---

## 9. Conclusion

PLAN-012 has been successfully implemented. The WITH expression parentheses fix is complete and verified. All unit tests pass. The fix correctly treats WITH expressions as atomic symbols that never need outer parentheses based on parent operator precedence.
