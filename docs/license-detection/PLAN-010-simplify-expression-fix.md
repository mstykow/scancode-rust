# PLAN-010: Fix `simplify_expression` Deduplication

**Status**: Draft  
**Priority**: Medium (~8 failing tests)  
**Estimated Effort**: Small  
**Created**: 2026-02-17

---

## 1. Problem Statement

The `simplify_expression` function in `src/license_detection/expression.rs` is intended to deduplicate license keys in expressions, but it **tracks duplicates without removing them** from the expression tree.

### Current Behavior (Incorrect)

```rust
let expr = parse_expression("crapl-0.1 AND crapl-0.1").unwrap();
let simplified = simplify_expression(&expr);
let result = expression_to_string(&simplified);
// Result: "crapl-0.1 AND crapl-0.1"  <- WRONG! Should be "crapl-0.1"
```

### Expected Behavior (Correct)

```rust
let expr = parse_expression("crapl-0.1 AND crapl-0.1").unwrap();
let simplified = simplify_expression(&expr);
let result = expression_to_string(&simplified);
// Result: "crapl-0.1"  <- Correct deduplication
```

### Impact

Approximately 8 tests fail with expressions like:
- `crapl-0.1 AND crapl-0.1` (expected: `crapl-0.1`)
- `gpl-2.0-plus AND gpl-2.0-plus` (expected: `gpl-2.0-plus`)
- `fsf-free AND fsf-free AND fsf-free` (expected: `fsf-free`)

---

## 2. Python Reference Analysis

### 2.1 Python's `dedup` Method

Location: `reference/scancode-toolkit/.venv/lib/python3.13/site-packages/license_expression/__init__.py:707-761`

```python
def dedup(self, expression):
    """
    Return a deduplicated LicenseExpression given a license ``expression``
    string or LicenseExpression object.

    The deduplication process is similar to simplification but is
    specialized for working with license expressions. Simplification is
    otherwise a generic boolean operation that is not aware of the specifics
    of license expressions.

    The deduplication:

    - Does not sort the licenses of sub-expression in an expression. They
      stay in the same order as in the original expression.

    - Choices (as in "MIT or GPL") are kept as-is and not treated as
      simplifiable. This avoids droping important choice options in complex
      expressions which is never desirable.
    """
    exp = self.parse(expression)
    expressions = []
    for arg in exp.args:
        if isinstance(arg, (self.AND, self.OR)):
            # Run this function recursively if there is another AND/OR expression
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
    else:
        raise ExpressionError(f"Unknown expression type: {expression!r}")
    return deduped
```

### 2.2 Python's `combine_expressions` Function

Location: `reference/scancode-toolkit/.venv/lib/python3.13/site-packages/license_expression/__init__.py:1746-1802`

```python
def combine_expressions(
    expressions,
    relation="AND",
    unique=True,
    licensing=Licensing(),
):
    """
    Return a combined LicenseExpression object with the `relation`, given a list
    of license ``expressions`` strings or LicenseExpression objects. If
    ``unique`` is True remove duplicates before combining expressions.
    """
    if not expressions:
        return

    # only deal with LicenseExpression objects
    expressions = [licensing.parse(le, simple=True) for le in expressions]

    if unique:
        # Remove duplicate element in the expressions list
        # and preserve original order
        expressions = list({str(x): x for x in expressions}.values())

    if len(expressions) == 1:
        return expressions[0]

    relation = {"AND": licensing.AND, "OR": licensing.OR}[relation]
    return relation(*expressions)
```

### 2.3 Key Python Behavior

1. **Deduplication happens at the list level** before building the expression tree
2. **Preserves order** - uses dict with string key but preserves insertion order
3. **Returns single expression** if only one unique license remains after dedup
4. **Recursive** - handles nested AND/OR expressions

---

## 3. Rust Code Analysis

### 3.1 Current Implementation (Buggy)

Location: `src/license_detection/expression.rs:212-247`

```rust
pub fn simplify_expression(expr: &LicenseExpression) -> LicenseExpression {
    simplify_internal(expr, &mut HashSet::new())
}

fn simplify_internal(expr: &LicenseExpression, seen: &mut HashSet<String>) -> LicenseExpression {
    match expr {
        LicenseExpression::License(key) => {
            if seen.contains(key) {
                LicenseExpression::License(key.clone())  // BUG: Returns duplicate anyway!
            } else {
                seen.insert(key.clone());
                LicenseExpression::License(key.clone())
            }
        }
        LicenseExpression::LicenseRef(key) => {
            if seen.contains(key) {
                LicenseExpression::LicenseRef(key.clone())  // BUG: Returns duplicate anyway!
            } else {
                seen.insert(key.clone());
                LicenseExpression::LicenseRef(key.clone())
            }
        }
        LicenseExpression::And { left, right } => LicenseExpression::And {
            left: Box::new(simplify_internal(left, seen)),
            right: Box::new(simplify_internal(right, seen)),
        },
        LicenseExpression::Or { left, right } => LicenseExpression::Or {
            left: Box::new(simplify_internal(left, seen)),
            right: Box::new(simplify_internal(right, seen)),
        },
        LicenseExpression::With { left, right } => LicenseExpression::With {
            left: Box::new(simplify_internal(left, seen)),
            right: Box::new(simplify_internal(right, seen)),
        },
    }
}
```

### 3.2 Bug Analysis

The function **tracks seen keys** but **never uses that information to filter duplicates**:

| Line | What it does | What it should do |
|------|--------------|-------------------|
| 219-220 | If key already seen, return it anyway | Return nothing or mark for removal |
| 234-237 | Recursively build AND with both children | Build AND only with unique children |
| 238-241 | Recursively build OR with both children | Build OR only with unique children |

### 3.3 Why Tests Pass Incorrectly

The existing test `test_simplify_expression_with_duplicates` (lines 826-832) is **misleading**:

```rust
#[test]
fn test_simplify_expression_with_duplicates() {
    let expr = parse_expression("MIT OR MIT").unwrap();
    let simplified = simplify_expression(&expr);
    let keys = simplified.license_keys();  // THIS DEDUPS INTERNALLY!
    assert_eq!(keys.len(), 1);              // Test passes for wrong reason
}
```

The `license_keys()` method (lines 122-128) has its own deduplication:

```rust
pub fn license_keys(&self) -> Vec<String> {
    let mut keys = Vec::new();
    self.collect_keys(&mut keys);
    keys.sort();
    keys.dedup();  // <-- Hides the bug!
    keys
}
```

The test should check `expression_to_string(&simplified)` instead.

---

## 4. Proposed Changes

### 4.1 Strategy: Collect-Flatten-Rebuild

Instead of tracking duplicates during traversal, we should:

1. **Collect** all unique licenses while preserving order
2. **Flatten** nested AND/OR expressions of the same operator
3. **Rebuild** expression with only unique licenses
4. **Collapse** if only one unique license remains

### 4.2 Implementation Approach

```rust
pub fn simplify_expression(expr: &LicenseExpression) -> LicenseExpression {
    simplify_internal(expr)
}

fn simplify_internal(expr: &LicenseExpression) -> LicenseExpression {
    match expr {
        // Leaf nodes: return as-is
        LicenseExpression::License(key) => LicenseExpression::License(key.clone()),
        LicenseExpression::LicenseRef(key) => LicenseExpression::LicenseRef(key.clone()),
        
        // WITH expressions: recursively simplify but don't dedupe across WITH
        // (MIT WITH Exception) AND (MIT WITH Exception) should dedupe
        LicenseExpression::With { left, right } => LicenseExpression::With {
            left: Box::new(simplify_internal(left)),
            right: Box::new(simplify_internal(right)),
        },
        
        // AND expressions: flatten, dedupe, rebuild
        LicenseExpression::And { left, right } => {
            let mut unique = Vec::new();
            collect_unique_and(expr, &mut unique);
            build_expression_from_list(&unique, CombineOperator::And)
        },
        
        // OR expressions: flatten, dedupe, rebuild
        LicenseExpression::Or { left, right } => {
            let mut unique = Vec::new();
            collect_unique_or(expr, &mut unique);
            build_expression_from_list(&unique, CombineOperator::Or)
        },
    }
}
```

### 4.3 Detailed Implementation

```rust
use std::collections::HashSet;

enum CombineOperator {
    And,
    Or,
}

/// Collect unique license expressions from an AND chain, preserving order.
fn collect_unique_and(expr: &LicenseExpression, unique: &mut Vec<LicenseExpression>) {
    match expr {
        LicenseExpression::And { left, right } => {
            collect_unique_and(left, unique);
            collect_unique_and(right, unique);
        }
        LicenseExpression::Or { .. } => {
            // OR inside AND: simplify the OR, then add if unique
            let simplified_or = simplify_internal(expr);
            add_if_unique(simplified_or, unique);
        }
        LicenseExpression::With { left, right } => {
            // WITH expressions: treat as atomic, add if unique
            add_if_unique(expr.clone(), unique);
        }
        LicenseExpression::License(key) => {
            add_if_unique(LicenseExpression::License(key.clone()), unique);
        }
        LicenseExpression::LicenseRef(key) => {
            add_if_unique(LicenseExpression::LicenseRef(key.clone()), unique);
        }
    }
}

/// Collect unique license expressions from an OR chain, preserving order.
fn collect_unique_or(expr: &LicenseExpression, unique: &mut Vec<LicenseExpression>) {
    match expr {
        LicenseExpression::Or { left, right } => {
            collect_unique_or(left, unique);
            collect_unique_or(right, unique);
        }
        LicenseExpression::And { .. } => {
            // AND inside OR: simplify the AND, then add if unique
            let simplified_and = simplify_internal(expr);
            add_if_unique(simplified_and, unique);
        }
        LicenseExpression::With { left, right } => {
            add_if_unique(expr.clone(), unique);
        }
        LicenseExpression::License(key) => {
            add_if_unique(LicenseExpression::License(key.clone()), unique);
        }
        LicenseExpression::LicenseRef(key) => {
            add_if_unique(LicenseExpression::LicenseRef(key.clone()), unique);
        }
    }
}

/// Add expression to list only if its string representation is not already present.
fn add_if_unique(expr: LicenseExpression, unique: &mut Vec<LicenseExpression>) {
    let expr_str = expression_to_string(&expr);
    let already_exists = unique.iter().any(|e| expression_to_string(e) == expr_str);
    if !already_exists {
        unique.push(expr);
    }
}

/// Build an expression from a list of unique expressions.
fn build_expression_from_list(
    unique: &[LicenseExpression],
    op: CombineOperator,
) -> LicenseExpression {
    match unique.len() {
        0 => panic!("build_expression_from_list called with empty list"),
        1 => unique[0].clone(),
        _ => {
            let mut iter = unique.iter();
            let mut result = iter.next().unwrap().clone();
            for expr in iter {
                result = match op {
                    CombineOperator::And => LicenseExpression::And {
                        left: Box::new(result),
                        right: Box::new(expr.clone()),
                    },
                    CombineOperator::Or => LicenseExpression::Or {
                        left: Box::new(result),
                        right: Box::new(expr.clone()),
                    },
                };
            }
            result
        }
    }
}
```

### 4.4 Alternative Simpler Approach

If the above is too complex, a simpler approach using the existing structure:

```rust
pub fn simplify_expression(expr: &LicenseExpression) -> LicenseExpression {
    let flattened = flatten_and_collect(expr);
    
    if flattened.len() == 1 {
        return flattened.into_iter().next().unwrap();
    }
    
    // Determine if this was originally AND or OR
    // Rebuild with unique items
    // ... (this approach needs refinement)
}
```

### 4.5 Key Implementation Considerations

1. **Preserve Order**: Python uses dict with string keys preserving insertion order. Rust should use similar approach.

2. **Handle Nested Expressions**: `(MIT AND Apache) OR (MIT AND Apache)` should become `MIT AND Apache`

3. **Don't Dedupe Across Operators**: `MIT OR MIT AND Apache` should NOT collapse to `MIT OR Apache` because AND has higher precedence than OR

4. **WITH Expressions**: `GPL-2.0 WITH Classpath-exception` should be treated as atomic for deduplication purposes

5. **Mixed Expressions**: `(MIT OR GPL) AND (MIT OR GPL)` should dedupe to `MIT OR GPL` (the entire OR expression is duplicated)

---

## 5. Testing Strategy

### 5.1 Update Existing Tests

Modify `test_simplify_expression_with_duplicates` to check the actual string output:

```rust
#[test]
fn test_simplify_expression_with_duplicates() {
    // Test AND deduplication
    let expr = parse_expression("MIT AND MIT").unwrap();
    let simplified = simplify_expression(&expr);
    assert_eq!(expression_to_string(&simplified), "mit");
    
    // Test OR deduplication
    let expr = parse_expression("MIT OR MIT").unwrap();
    let simplified = simplify_expression(&expr);
    assert_eq!(expression_to_string(&simplified), "mit");
    
    // Test multiple duplicates
    let expr = parse_expression("MIT AND MIT AND MIT").unwrap();
    let simplified = simplify_expression(&expr);
    assert_eq!(expression_to_string(&simplified), "mit");
}
```

### 5.2 New Test Cases

```rust
#[test]
fn test_simplify_and_duplicates() {
    // crapl-0.1 AND crapl-0.1 -> crapl-0.1
    let expr = parse_expression("crapl-0.1 AND crapl-0.1").unwrap();
    let simplified = simplify_expression(&expr);
    assert_eq!(expression_to_string(&simplified), "crapl-0.1");
}

#[test]
fn test_simplify_or_duplicates() {
    // mit OR mit -> mit
    let expr = parse_expression("mit OR mit").unwrap();
    let simplified = simplify_expression(&expr);
    assert_eq!(expression_to_string(&simplified), "mit");
}

#[test]
fn test_simplify_preserves_different_licenses() {
    // mit AND apache-2.0 -> mit AND apache-2.0 (no change)
    let expr = parse_expression("mit AND apache-2.0").unwrap();
    let simplified = simplify_expression(&expr);
    assert_eq!(expression_to_string(&simplified), "mit AND apache-2.0");
}

#[test]
fn test_simplify_complex_duplicates() {
    // gpl-2.0-plus AND gpl-2.0-plus AND lgpl-2.0-plus
    // -> gpl-2.0-plus AND lgpl-2.0-plus
    let expr = parse_expression("gpl-2.0-plus AND gpl-2.0-plus AND lgpl-2.0-plus").unwrap();
    let simplified = simplify_expression(&expr);
    assert_eq!(expression_to_string(&simplified), "gpl-2.0-plus AND lgpl-2.0-plus");
}

#[test]
fn test_simplify_three_duplicates() {
    // fsf-free AND fsf-free AND fsf-free -> fsf-free
    let expr = parse_expression("fsf-free AND fsf-free AND fsf-free").unwrap();
    let simplified = simplify_expression(&expr);
    assert_eq!(expression_to_string(&simplified), "fsf-free");
}

#[test]
fn test_simplify_with_expression_dedup() {
    // gpl-2.0 WITH classpath-exception-2.0 AND gpl-2.0 WITH classpath-exception-2.0
    // -> gpl-2.0 WITH classpath-exception-2.0
    let expr = parse_expression(
        "gpl-2.0 WITH classpath-exception-2.0 AND gpl-2.0 WITH classpath-exception-2.0"
    ).unwrap();
    let simplified = simplify_expression(&expr);
    assert_eq!(
        expression_to_string(&simplified),
        "gpl-2.0 WITH classpath-exception-2.0"
    );
}

#[test]
fn test_simplify_mixed_and_or() {
    // mit OR gpl-2.0 AND mit OR gpl-2.0
    // This should NOT simplify because AND has higher precedence
    // The expression is: mit OR (gpl-2.0 AND mit) OR gpl-2.0
    // Not: (mit OR gpl-2.0) AND (mit OR gpl-2.0)
    let expr = parse_expression("mit OR gpl-2.0 AND mit OR gpl-2.0").unwrap();
    let simplified = simplify_expression(&expr);
    // Result depends on parse tree structure - document expected behavior
}

#[test]
fn test_simplify_nested_duplicates() {
    // (mit AND apache-2.0) OR (mit AND apache-2.0)
    // -> mit AND apache-2.0
    let expr = parse_expression("(mit AND apache-2.0) OR (mit AND apache-2.0)").unwrap();
    let simplified = simplify_expression(&expr);
    assert_eq!(expression_to_string(&simplified), "mit AND apache-2.0");
}

#[test]
fn test_simplify_preserves_order() {
    // apache-2.0 AND mit AND apache-2.0 -> apache-2.0 AND mit
    // (first occurrence preserved, not sorted)
    let expr = parse_expression("apache-2.0 AND mit AND apache-2.0").unwrap();
    let simplified = simplify_expression(&expr);
    assert_eq!(expression_to_string(&simplified), "apache-2.0 AND mit");
}
```

### 5.3 Golden Test Verification

Run the license golden tests and verify these files now pass:

```bash
cargo test --test scanner_integration -- --test-threads=1 2>&1 | grep -E "(crapl|gpl-2.0-plus_21|gpl-2.0-plus_22|gpl-2.0_and_lgpl-2.0-plus|gpl-2.0_or_bsd-simplified|fsf-free_and_fsf-free)"
```

Expected: All tests should pass after the fix.

### 5.4 Edge Cases to Test

| Input | Expected Output | Reason |
|-------|-----------------|--------|
| `MIT AND MIT` | `mit` | Simple AND dedup |
| `MIT OR MIT` | `mit` | Simple OR dedup |
| `MIT AND MIT AND MIT` | `mit` | Multiple duplicates |
| `MIT AND Apache AND MIT` | `mit AND apache` | Middle duplicate |
| `GPL-2.0 WITH Exception AND GPL-2.0 WITH Exception` | `gpl-2.0 WITH exception` | WITH expression dedup |
| `(MIT OR GPL) AND (MIT OR GPL)` | `mit OR gpl` | Nested expression dedup |
| `MIT AND Apache` | `mit AND apache` | No dedup needed |

---

## 6. Implementation Checklist

- [ ] Implement new `simplify_expression` logic using collect-flatten-rebuild approach
- [ ] Add helper functions `collect_unique_and`, `collect_unique_or`, `add_if_unique`
- [ ] Handle WITH expressions as atomic units
- [ ] Handle nested AND/OR expressions correctly
- [ ] Preserve order of first occurrence
- [ ] Update existing test `test_simplify_expression_with_duplicates`
- [ ] Add 8+ new test cases covering edge cases
- [ ] Run `cargo test` to verify no regressions
- [ ] Run golden tests to verify 8 failing tests now pass
- [ ] Run `cargo clippy` for linting
- [ ] Update any documentation if needed

---

## 7. Related Issues

- FAILURES.md Section 5: `simplify_expression` not deduplicating (~8 tests)
- FAILURES.md lines 146, 174, 197-198, 216, 224, 278-284

---

## 8. References

- Python `license-expression` library: `reference/scancode-toolkit/.venv/lib/python3.13/site-packages/license_expression/__init__.py`
- Python `dedup` method: lines 707-761
- Python `combine_expressions` function: lines 1746-1802
- Rust `expression.rs`: `src/license_detection/expression.rs`
- AGENTS.md Section: "Porting Features from Original ScanCode"
