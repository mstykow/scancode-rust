# Phase 7: SPDX Expression Parsing - Implementation Plan

**Status:** Planning  
**Created:** 2026-03-01  
**Estimated Tests Fixed:** ~10  
**Complexity:** Medium  

## Executive Summary

SPDX-License-Identifier expression parsing has critical gaps in handling non-standard and complex expressions. The Rust implementation lacks the recovery parsing logic that Python uses to handle malformed or non-standard SPDX expressions (e.g., U-Boot style bare license lists).

## Problem Analysis

### Current Behavior vs Expected

| Test File | SPDX Expression | Expected Output | Actual Output | Issue |
|-----------|-----------------|-----------------|---------------|-------|
| `uboot.c:32` | `GPL-2.0+ BSD-2-Clause` | `gpl-2.0-plus OR bsd-simplified` | `gpl-2.0-plus` | U-Boot style OR not applied |
| `uboot.c:2-7` | Multiple BSD/ecos identifiers (lines 2-7) | `unknown-spdx OR unknown-spdx OR ...` (6 total) | Single unknown-spdx | Multiple unknowns not OR'd |
| `misc.c:86` | `#. # SPDX-License-Identifier: BSD-3-Clause` | `unknown-spdx OR unknown-spdx` | `unknown-spdx` | Recovery parsing missing |
| `missing_leading_trailing_paren.txt:3` | `(GPL-2.0-ONLY OR (MIT)` | `(gpl-2.0 AND mit) AND unknown-spdx` | `gpl-2.0` | Unbalanced parens not recovered |

**Note:** `uboot.c:2-7` refers to the first expected expression in `uboot.c.yml` line 2, which combines 6 SPDX lines from source lines 2-7 into one OR expression.

### Root Cause

The Rust implementation in `src/license_detection/spdx_lid.rs` has three key gaps:

1. **No recovery parsing function** - Python has `_reparse_invalid_expression()` to handle malformed expressions
2. **Wrong fallback logic** - `split_license_expression()` strips operators and returns first key only (lines 371-388 in spdx_lid.rs)
3. **Missing U-Boot style detection** - Bare license lists without keywords should default to OR

**Note:** `LicenseExpression::and()` and `LicenseExpression::or()` already exist in `expression.rs:142-178` and build left-associative chains correctly.

## Python Reference Implementation

### Key Function: `_reparse_invalid_expression()` (match_spdx_lid.py:271-340)

```python
def _reparse_invalid_expression(text, licensing, expression_symbols, unknown_symbol):
    """
    Make a best attempt at parsing eventually ignoring some of the syntax.
    Any keyword and parens will be ignored.
    """
    results = licensing.simple_tokenizer(text)
    tokens = [r.value for r in results if isinstance(r.value, (LicenseSymbol, Keyword))]
    
    has_keywords = False
    has_symbols = False
    filtered_tokens = []
    
    for tok in tokens:
        if isinstance(tok, Keyword):
            has_keywords = True
            continue
        else:
            filtered_tokens.append(tok)
            has_symbols = True
    
    if not has_symbols:
        return unknown_symbol
    
    # KEY LOGIC:
    joined_as = ' AND '
    if not has_keywords:
        # U-Boot style: bare list without keywords = OR
        joined_as = ' OR '
    
    expression_text = joined_as.join(s.key for s in filtered_tokens)
    expression = _parse_expression(expression_text, ...)
    
    if has_keywords:
        # Invalid expression with keywords = append unknown-spdx
        expression = licensing.AND(expression, unknown_symbol)
    
    return expression
```

### Key Function: `get_expression()` (match_spdx_lid.py:154-193)

```python
def get_expression(text, licensing, expression_symbols, unknown_symbol):
    """
    Note that an expression is ALWAYS returned: if parsing fails,
    returns a bare expression made of only "unknown-spdx" symbol.
    """
    _prefix, text = prepare_text(text)
    if not text:
        return
    
    expression = None
    try:
        expression = _parse_expression(text, ...)
    except Exception:
        try:
            # Recovery parsing for invalid expressions
            expression = _reparse_invalid_expression(text, ...)
        except Exception:
            pass
    
    if expression is None:
        expression = unknown_symbol
    
    return expression
```

## Rust Implementation Analysis

### Current Flow in `find_matching_rule_for_expression()` (spdx_lid.rs:349-393)

```rust
fn find_matching_rule_for_expression(index: &LicenseIndex, expression: &str) -> Option<String> {
    // 1. Direct lookup by SPDX key
    if let Some(&rid) = index.rid_by_spdx_key.get(expression) {
        return Some(index.rules_by_rid[rid].license_expression.clone());
    }
    
    // 2. Normalized lookup
    for rule in &index.rules_by_rid {
        let normalized = normalize_spdx_key(&rule.license_expression);
        if normalized == expression {
            return Some(rule.license_expression.clone());
        }
    }
    
    // 3. Parse as expression (expects valid SPDX syntax)
    if let Ok(parsed) = parse_expression(expression)
        && let Some(converted) = convert_spdx_expression_to_scancode(&parsed, index)
    {
        let result = expression_to_string(&converted);
        if !result.is_empty() {
            return Some(result);
        }
    }
    
    // 4. BUGGY FALLBACK: Strips operators, returns first key only!
    let license_keys = split_license_expression(expression);
    if license_keys.is_empty() {
        return index.unknown_spdx_rid.map(|rid| ...);
    }
    
    let first_key = &license_keys[0];  // WRONG: Loses other keys
    // ... lookup first_key ...
}
```

### Current `split_license_expression()` (spdx_lid.rs:198-225)

```rust
fn split_license_expression(license_expression: &str) -> Vec<String> {
    let normalized = license_expression.replace(['(', ')'], " ");
    // ... tokenize ...
    
    tokens.into_iter()
        .filter(|t| {
            let t_lower = t.to_lowercase();
            !matches!(t_lower.as_str(), "and" | "or" | "with")  // STRIPS OPERATORS!
        })
        .collect()
}
```

This function is used incorrectly - it's designed to extract license keys from a known-good expression, not to parse an unknown expression.

## Implementation Plan

### Step 1: Add Recovery Parsing Function

**File:** `src/license_detection/spdx_lid.rs`

Add a new function `reparse_invalid_expression()` that mirrors Python's `_reparse_invalid_expression()`:

```rust
/// Represents the result of tokenizing an SPDX expression for recovery parsing.
enum RecoveryToken {
    LicenseKey(String),
    Keyword(SpdxKeyword),
}

enum SpdxKeyword {
    And,
    Or,
    With,
}

/// Attempt to parse an invalid SPDX expression using lenient recovery.
///
/// This handles:
/// - U-Boot style bare license lists (e.g., "GPL-2.0+ BSD-2-Clause") -> OR
/// - Expressions with keywords that don't parse (e.g., unbalanced parens) -> AND + unknown-spdx
/// - Completely invalid expressions -> unknown-spdx
///
/// Based on Python: licensedcode/match_spdx_lid.py:_reparse_invalid_expression()
fn reparse_invalid_expression(
    text: &str,
    index: &LicenseIndex,
) -> Option<LicenseExpression> {
    // 1. Tokenize to extract license keys and keywords
    let tokens = tokenize_for_recovery(text);
    
    // 2. Separate keys and keywords
    let mut has_keywords = false;
    let mut license_keys: Vec<String> = Vec::new();
    
    for token in tokens {
        match token {
            RecoveryToken::Keyword(_) => has_keywords = true,
            RecoveryToken::LicenseKey(key) => license_keys.push(key),
        }
    }
    
    // 3. No keys? Return unknown-spdx
    if license_keys.is_empty() {
        return Some(LicenseExpression::License("unknown-spdx".to_string()));
    }
    
    // 4. Build expression
    let expressions: Vec<LicenseExpression> = license_keys
        .into_iter()
        .map(|k| LicenseExpression::License(k.to_lowercase()))
        .collect();
    
    // 5. U-Boot style: no keywords = OR
    // Otherwise: has keywords but didn't parse = AND + unknown-spdx witness
    let mut result = if has_keywords {
        LicenseExpression::and(expressions)?
    } else {
        LicenseExpression::or(expressions)?
    };
    
    // 6. For invalid expressions with keywords, append unknown-spdx as witness
    if has_keywords {
        result = LicenseExpression::And {
            left: Box::new(result),
            right: Box::new(LicenseExpression::License("unknown-spdx".to_string())),
        };
    }
    
    Some(result)
}

/// Tokenize text for recovery parsing, extracting license keys and keywords.
fn tokenize_for_recovery(text: &str) -> Vec<RecoveryToken> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    
    for c in text.chars() {
        match c {
            ' ' | '\t' | '(' | ')' => {
                if !current.is_empty() {
                    tokens.push(classify_token(&current));
                    current.clear();
                }
            }
            _ => current.push(c),
        }
    }
    
    if !current.is_empty() {
        tokens.push(classify_token(&current));
    }
    
    tokens
}

fn classify_token(text: &str) -> RecoveryToken {
    let upper = text.to_uppercase();
    match upper.as_str() {
        "AND" => RecoveryToken::Keyword(SpdxKeyword::And),
        "OR" => RecoveryToken::Keyword(SpdxKeyword::Or),
        "WITH" => RecoveryToken::Keyword(SpdxKeyword::With),
        _ => RecoveryToken::LicenseKey(text.to_lowercase()),
    }
}
```

### Step 2: Add Recovery-Aware Expression Conversion

**Critical Issue:** The existing `convert_spdx_expression_to_scancode()` (lines 395-453) returns `None` if ANY license key is not found in the SPDX key mapping. This breaks recovery parsing because unknown keys would cause the entire conversion to fail.

**Solution:** Create a recovery-aware conversion function that substitutes unknown keys with `unknown-spdx`:

```rust
/// Convert a recovered SPDX expression to scancode keys.
/// Unknown SPDX keys are replaced with "unknown-spdx".
fn convert_recovered_expression_to_scancode(
    expr: &LicenseExpression,
    index: &LicenseIndex,
) -> LicenseExpression {
    match expr {
        LicenseExpression::License(key) => {
            let lookup_key = key.to_lowercase();
            if let Some(&rid) = index.rid_by_spdx_key.get(&lookup_key) {
                LicenseExpression::License(
                    index.rules_by_rid[rid].license_expression.clone(),
                )
            } else {
                // Unknown key -> unknown-spdx
                LicenseExpression::License("unknown-spdx".to_string())
            }
        }
        LicenseExpression::LicenseRef(key) => {
            let lookup_key = key.to_lowercase();
            if let Some(&rid) = index.rid_by_spdx_key.get(&lookup_key) {
                LicenseExpression::License(
                    index.rules_by_rid[rid].license_expression.clone(),
                )
            } else {
                LicenseExpression::License("unknown-spdx".to_string())
            }
        }
        LicenseExpression::And { left, right } => LicenseExpression::And {
            left: Box::new(convert_recovered_expression_to_scancode(left, index)),
            right: Box::new(convert_recovered_expression_to_scancode(right, index)),
        },
        LicenseExpression::Or { left, right } => LicenseExpression::Or {
            left: Box::new(convert_recovered_expression_to_scancode(left, index)),
            right: Box::new(convert_recovered_expression_to_scancode(right, index)),
        },
        LicenseExpression::With { left, right } => LicenseExpression::With {
            left: Box::new(convert_recovered_expression_to_scancode(left, index)),
            right: Box::new(convert_recovered_expression_to_scancode(right, index)),
        },
    }
}
```

This mirrors Python's behavior in `_parse_expression()` (lines 252-267) where unknown symbols are replaced with `unknown_symbol`.

### Step 3: Update `find_matching_rule_for_expression()`

Replace the buggy fallback logic with recovery parsing:

```rust
fn find_matching_rule_for_expression(index: &LicenseIndex, expression: &str) -> Option<String> {
    let lowered = expression.to_lowercase();
    
    // 1. Direct lookup by SPDX key
    if let Some(&rid) = index.rid_by_spdx_key.get(&lowered) {
        return Some(index.rules_by_rid[rid].license_expression.clone());
    }
    
    // 2. Try parsing as valid expression
    if let Ok(parsed) = parse_expression(expression)
        && let Some(converted) = convert_spdx_expression_to_scancode(&parsed, index)
    {
        let result = expression_to_string(&converted);
        if !result.is_empty() {
            return Some(result);
        }
    }
    
    // 3. RECOVERY PARSING for invalid/non-standard expressions
    if let Some(recovered) = reparse_invalid_expression(expression, index) {
        let converted = convert_recovered_expression_to_scancode(&recovered, index);
        let result = expression_to_string(&converted);
        if !result.is_empty() {
            return Some(result);
        }
    }
    
    // 4. Final fallback: unknown-spdx
    index.unknown_spdx_rid
        .map(|rid| index.rules_by_rid[rid].license_expression.clone())
}
```

### Step 4: Handle Multiple Unknown SPDX Identifiers

For cases where multiple unknown SPDX identifiers appear (like uboot.c lines 2-7), the recovery parsing function handles this by creating OR chains. The `LicenseExpression::or()` function in `expression.rs:162-178` already correctly builds left-associative OR chains.

**Important:** The `uboot.c:2-7` case refers to multiple consecutive SPDX lines that each contain unknown identifiers. Python combines these into a single OR expression. This is NOT about combining multiple matches from the same line - it's about the recovery parsing for a single expression with multiple unknown keys.

For example, if a single SPDX line contains `BSD-2-Clause BSD-3-Clause eCos-2.0` (none parse as valid SPDX), recovery parsing should return `unknown-spdx OR unknown-spdx OR unknown-spdx`.
```

### Step 5: No Changes Needed to `spdx_lid_match()`

The `spdx_lid_match()` function in `spdx_lid.rs:247-346` processes SPDX lines and calls `find_matching_rule_for_expression()` for each line. The recovery parsing logic will be called internally by `find_matching_rule_for_expression()` when normal parsing fails. No changes are needed to the outer `spdx_lid_match()` function itself.

The key fix is in `find_matching_rule_for_expression()` (lines 349-393), which needs to call recovery parsing instead of the current buggy fallback that only returns the first key.
```

## Test Cases

### Unit Tests for `reparse_invalid_expression()`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reparse_uboot_style_bare_list() {
        // U-Boot style: bare list without keywords -> OR
        let result = reparse_invalid_expression("GPL-2.0+ BSD-2-Clause", &index);
        assert!(result.is_some());
        let expr = expression_to_string(&result.unwrap());
        assert_eq!(expr, "gpl-2.0+ OR bsd-2-clause");
    }

    #[test]
    fn test_reparse_uboot_style_multiple() {
        // Multiple bare license keys -> OR chain
        let result = reparse_invalid_expression("MIT Apache-2.0 GPL-2.0", &index);
        assert!(result.is_some());
        let expr = expression_to_string(&result.unwrap());
        assert!(expr.contains(" OR "));
        assert!(expr.contains("mit"));
        assert!(expr.contains("apache-2.0"));
        assert!(expr.contains("gpl-2.0"));
    }

    #[test]
    fn test_reparse_with_keywords_appends_unknown() {
        // Has keywords but invalid (unbalanced) -> AND + unknown-spdx
        let result = reparse_invalid_expression("(GPL-2.0 OR MIT", &index);
        assert!(result.is_some());
        let expr = expression_to_string(&result.unwrap());
        assert!(expr.contains(" AND "));
        assert!(expr.contains("unknown-spdx"));
    }

    #[test]
    fn test_reparse_no_keys_returns_unknown() {
        let result = reparse_invalid_expression("AND OR WITH", &index);
        assert!(result.is_some());
        let expr = expression_to_string(&result.unwrap());
        assert_eq!(expr, "unknown-spdx");
    }

    #[test]
    fn test_reparse_single_key() {
        let result = reparse_invalid_expression("MIT", &index);
        assert!(result.is_some());
        let expr = expression_to_string(&result.unwrap());
        assert_eq!(expr, "mit");
    }
}
```

### Integration Tests

```rust
#[test]
fn test_uboot_spdx_bare_list_or() {
    // File: uboot.c line 32
    // SPDX-License-Identifier: GPL-2.0+ BSD-2-Clause
    let text = "SPDX-License-Identifier: GPL-2.0+ BSD-2-Clause";
    let index = create_test_index_from_real_data();
    let query = Query::new(text, &index).unwrap();
    let matches = spdx_lid_match(&index, &query);
    
    assert_eq!(matches.len(), 1);
    assert!(matches[0].license_expression.contains(" OR "));
    assert!(matches[0].license_expression.contains("gpl-2.0"));
    assert!(matches[0].license_expression.contains("bsd-simplified"));
}

#[test]
fn test_missing_leading_trailing_paren() {
    // File: missing_leading_trailing_paren.txt line 3
    // SPDX-license-identifier: (GPL-2.0-ONLY OR (MIT)
    let text = "SPDX-license-identifier: (GPL-2.0-ONLY OR (MIT)";
    let index = create_test_index_from_real_data();
    let query = Query::new(text, &index).unwrap();
    let matches = spdx_lid_match(&index, &query);
    
    assert_eq!(matches.len(), 1);
    // Should contain AND (because has OR keyword) and unknown-spdx
    assert!(matches[0].license_expression.contains(" AND "));
    assert!(matches[0].license_expression.contains("unknown-spdx"));
}
```

**Note:** The `create_test_index_from_real_data()` helper should use the actual license index from `reference/scancode-toolkit/src/licensedcode/data/`.

### Golden Test Validation

After implementation, run:

```bash
cargo test --release -q --lib license_detection::golden_test -- spdx
```

Expected improvements:
- `uboot.c` - All 40 detections should match (39 SPDX lines + 1 combined line at start)
- `misc.c` - All 16 detections should match
- `missing_leading_trailing_paren.txt` - All 3 detections should match

## Files to Modify

| File | Changes |
|------|---------|
| `src/license_detection/spdx_lid.rs` | Add `reparse_invalid_expression()`, update `find_matching_rule_for_expression()`, add unit tests in `#[cfg(test)] mod tests` block (existing tests at lines 456-1215) |

**Note:** Tests are co-located in `spdx_lid.rs` rather than a separate test file.

## Dependencies

None - this phase is independent of other phases.

## Risk Assessment

**Low Risk** - The changes are isolated to SPDX expression parsing and won't affect other detection paths. The recovery parsing is only invoked when normal parsing fails, so existing correct behavior is preserved.

## Validation Checklist

- [ ] Unit tests pass for `reparse_invalid_expression()`
- [ ] Golden tests for `uboot.c` pass
- [ ] Golden tests for `misc.c` pass
- [ ] Golden tests for `missing_leading_trailing_paren.txt` pass
- [ ] No regressions in other SPDX tests
- [ ] Full test suite passes

## References

- Python implementation: `reference/scancode-toolkit/src/licensedcode/match_spdx_lid.py`
- Rust implementation: `src/license_detection/spdx_lid.rs`
- Test data: `testdata/license-golden/datadriven/external/spdx/`
- Roadmap: `docs/license-detection/0016-feature-parity-roadmap.md`

## Verification Summary

This plan was verified on 2026-03-01:

1. **Code locations verified:**
   - `src/license_detection/spdx_lid.rs` exists with tests at lines 456-1215
   - `src/license_detection/expression.rs` exists with `LicenseExpression::and()` at lines 142-158 and `or()` at lines 162-178
   - `expression_to_string()` at expression.rs:548
   - `convert_spdx_expression_to_scancode()` at spdx_lid.rs:395-453

2. **Python reference verified:**
   - `_reparse_invalid_expression()` at match_spdx_lid.py:271-340
   - `get_expression()` at match_spdx_lid.py:154-193
   - Recovery logic correctly analyzed (U-Boot style OR, keyword expressions AND + unknown-spdx)

3. **Test data verified:**
   - `testdata/license-golden/datadriven/external/spdx/uboot.c` and `uboot.c.yml` - 39 source lines, 40 expected expressions
   - `testdata/license-golden/datadriven/external/spdx/misc.c` and `misc.c.yml` - 89 source lines, 16 expected expressions
   - `testdata/license-golden/datadriven/external/spdx/missing_leading_trailing_paren.txt` and `.yml` - 3 lines, 3 expected expressions

4. **Critical issue identified and addressed:**
   - `convert_spdx_expression_to_scancode()` returns `None` for unknown keys, which would break recovery parsing
   - Solution: Added Step 2 with `convert_recovered_expression_to_scancode()` that substitutes unknown keys with `unknown-spdx`
