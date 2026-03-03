# SPDX Expression Parsing Issues - Detailed Implementation Plan

## Status: IMPLEMENTED

### Implementation Completed (2026-03-03)

**What was implemented:**

1. **Recovery tokenizer** - Added `tokenize_for_recovery()` and `classify_recovery_token()` to extract license keys and keywords from malformed expressions

2. **Recovery parsing** - Implemented `reparse_invalid_expression()` that:
   - Separates keywords (AND/OR/WITH) from license keys
   - Uses OR for bare lists (U-Boot style: `GPL-2.0+ BSD-2-Clause`)
   - Uses AND + `unknown-spdx` for expressions with keywords that didn't parse
   - Properly handles unknown SPDX identifiers

3. **Expression conversion** - Added `convert_recovered_expression_to_scancode()` that substitutes unknown keys with `unknown-spdx` instead of failing

4. **Updated main function** - Modified `find_matching_rule_for_expression()` to call recovery parsing when normal parsing fails

5. **License key validation** - Added `is_likely_license_key()` to filter out non-license text (e.g., "The author added...")

**Test results:**
- U-Boot style bare lists now produce OR expressions
- Malformed parentheses are handled gracefully
- Unknown SPDX identifiers map to `unknown-spdx`
- Text after SPDX identifiers is properly ignored

**Note:** While this fix is implemented correctly, it may not significantly reduce the golden test failure count if the tests are checking other aspects of the detection pipeline.

---

**Created:** 2026-03-03  
**Priority:** High  
**Estimated Tests Fixed:** ~50 SPDX-related failures

## Executive Summary

SPDX-License-Identifier expression parsing has four critical gaps in the Rust implementation:

1. **No recovery parsing** - Python handles malformed expressions with `_reparse_invalid_expression()`
2. **Wrong fallback logic** - Rust returns first key only, losing multi-license expressions
3. **Unknown key handling** - Rust's conversion fails entirely on unknown keys
4. **Expression structure errors** - U-Boot style bare lists should use OR, not lose licenses

## Problem Analysis

### Issue 1: Malformed Parentheses

**Example:** `(GPL-2.0-ONLY OR MIT` (missing close paren)

| Implementation | Result |
|----------------|--------|
| Python | `gpl-2.0 OR mit` |
| Rust (current) | Parse error → `gpl-2.0` (first key only) |
| Expected | `gpl-2.0 OR mit` (recovery parsing applied) |

**Root Cause:** Rust's `parse_expression()` returns `Err(ParseError::MismatchedParentheses)` with no fallback.

### Issue 2: Unknown SPDX Identifiers

**Example:** `LGPL-2.1+ The author added some notes here...`

| Implementation | Result |
|----------------|--------|
| Python | `unknown-spdx` |
| Rust (current) | Falls through to first-key fallback |
| Expected | `unknown-spdx` |

**Root Cause:** Unknown keys are not properly handled in fallback path.

### Issue 3: U-Boot Style Bare Lists

**Example:** `GPL-2.0+ BSD-2-Clause` (no operators between licenses)

| Implementation | Result |
|----------------|--------|
| Python | `gpl-2.0-plus OR bsd-simplified` |
| Rust (current) | `gpl-2.0-plus` (first key only) |
| Expected | `gpl-2.0-plus OR bsd-simplified` |

**Root Cause:** Rust lacks U-Boot style detection logic.

### Issue 4: Expression with Keywords but Malformed

**Example:** `(GPL-2.0+ and (BSD-2-Clause` (unbalanced parens with keywords)

| Implementation | Result |
|----------------|--------|
| Python | `(gpl-2.0-plus AND bsd-simplified) AND unknown-spdx` |
| Rust (current) | Parse error → first key fallback |
| Expected | AND expression with `unknown-spdx` appended as witness |

**Root Cause:** No recovery parsing that appends `unknown-spdx` for invalid expressions with keywords.

## Root Cause Analysis

### Code Location: `src/license_detection/spdx_lid/mod.rs`

The `find_matching_rule_for_expression()` function (lines 358-417) has a flawed fallback:

```rust
// Lines 395-416 - BUGGY FALLBACK
let license_keys = split_license_expression(expression);
if license_keys.is_empty() {
    return index.unknown_spdx_rid.map(...);
}

let first_key = &license_keys[0];  // WRONG: Only uses first key!
if let Some(&rid) = index.rid_by_spdx_key.get(first_key) {
    return Some(index.rules_by_rid[rid].license_expression.clone());
}
// ... more first_key fallback logic
```

**Problems:**
1. `split_license_expression()` strips operators (AND/OR/WITH), losing structure
2. Only returns first key from the list
3. No detection of U-Boot style (bare list = OR)
4. No appending of `unknown-spdx` for malformed expressions with keywords

### Python Reference: `match_spdx_lid.py:271-340`

```python
def _reparse_invalid_expression(text, licensing, expression_symbols, unknown_symbol):
    # Tokenize, extracting license keys and keywords
    results = licensing.simple_tokenizer(text)
    tokens = [r.value for r in results 
              if isinstance(r.value, (LicenseSymbol, Keyword))]
    
    # Separate keywords from license keys
    has_keywords = False
    filtered_tokens = []
    for tok in tokens:
        if isinstance(tok, Keyword):
            has_keywords = True  # Track that we saw operators
        else:
            filtered_tokens.append(tok)
    
    if not filtered_tokens:
        return unknown_symbol
    
    # KEY LOGIC: U-Boot style = OR, otherwise = AND
    joined_as = ' AND '
    if not has_keywords:
        joined_as = ' OR '  # Bare list = OR
    
    expression_text = joined_as.join(s.key for s in filtered_tokens)
    expression = _parse_expression(expression_text, ...)
    
    # Append unknown-spdx as witness for invalid expressions with keywords
    if has_keywords:
        expression = licensing.AND(expression, unknown_symbol)
    
    return expression
```

### Key Python Behavior Differences

| Scenario | Python Behavior | Rust Current Behavior |
|----------|-----------------|----------------------|
| Valid expression | Parse normally | Parse normally ✓ |
| Bare list (no keywords) | Join with OR | Return first key only ✗ |
| Has keywords but invalid | Join with AND + unknown-spdx | Return first key only ✗ |
| No valid keys | Return unknown-spdx | Return unknown-spdx ✓ |
| Unbalanced parens | Strip parens, apply above rules | Parse error ✗ |

## Implementation Steps

### Step 1: Add Recovery Token Types

**File:** `src/license_detection/spdx_lid/mod.rs`

Add types for recovery parsing:

```rust
#[derive(Debug, Clone, PartialEq)]
enum RecoveryToken {
    LicenseKey(String),
    Keyword(SpdxKeyword),
    Ignored,  // Non-license text that should be skipped
}

#[derive(Debug, Clone, PartialEq)]
enum SpdxKeyword {
    And,
    Or,
    With,
}
```

### Step 2: Implement Recovery Tokenizer

Add a tokenizer that extracts license keys and keywords:

```rust
fn tokenize_for_recovery(text: &str) -> Vec<RecoveryToken> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    
    for c in text.to_lowercase().chars() {
        match c {
            ' ' | '\t' | '(' | ')' | '\n' | '\r' => {
                if !current.is_empty() {
                    tokens.push(classify_recovery_token(&current));
                    current.clear();
                }
            }
            _ => current.push(c),
        }
    }
    
    if !current.is_empty() {
        tokens.push(classify_recovery_token(&current));
    }
    
    // Filter out Ignored tokens before returning
    tokens.into_iter().filter(|t| !matches!(t, RecoveryToken::Ignored)).collect()
}

fn classify_recovery_token(text: &str) -> RecoveryToken {
    match text {
        "and" => RecoveryToken::Keyword(SpdxKeyword::And),
        "or" => RecoveryToken::Keyword(SpdxKeyword::Or),
        "with" => RecoveryToken::Keyword(SpdxKeyword::With),
        _ => RecoveryToken::LicenseKey(text.to_string()),
    }
}
```

### Step 3: Implement Recovery Parsing Function

Add the core recovery function:

```rust
fn reparse_invalid_expression(
    text: &str,
    index: &LicenseIndex,
) -> Option<LicenseExpression> {
    let tokens = tokenize_for_recovery(text);
    
    let mut has_keywords = false;
    let mut license_keys: Vec<String> = Vec::new();
    
    for token in tokens {
        match token {
            RecoveryToken::Keyword(_) => has_keywords = true,
            RecoveryToken::LicenseKey(key) => license_keys.push(key),
        }
    }
    
    if license_keys.is_empty() {
        return Some(LicenseExpression::License("unknown-spdx".to_string()));
    }
    
    let expressions: Vec<LicenseExpression> = license_keys
        .into_iter()
        .map(|k| LicenseExpression::License(k))
        .collect();
    
    // U-Boot style: no keywords = OR
    // Otherwise: has keywords but didn't parse = AND + unknown-spdx
    let mut result = if has_keywords {
        LicenseExpression::and(expressions)
            .unwrap_or(LicenseExpression::License("unknown-spdx".to_string()))
    } else {
        LicenseExpression::or(expressions)
            .unwrap_or(LicenseExpression::License("unknown-spdx".to_string()))
    };
    
    // For invalid expressions with keywords, append unknown-spdx as witness
    if has_keywords {
        result = LicenseExpression::And {
            left: Box::new(result),
            right: Box::new(LicenseExpression::License("unknown-spdx".to_string())),
        };
    }
    
    Some(result)
}
```

### Step 4: Add Recovery-Aware Expression Conversion

**Critical:** The existing `convert_spdx_expression_to_scancode()` returns `None` if ANY key is unknown. We need a version that substitutes unknown keys:

```rust
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
                LicenseExpression::License("unknown-spdx".to_string())
            }
        }
        LicenseExpression::LicenseRef(key) => {
            // LicenseRef-* always maps to unknown-spdx unless explicitly known
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

### Step 5: Update `find_matching_rule_for_expression()`

Replace the buggy fallback with recovery parsing:

```rust
pub(crate) fn find_matching_rule_for_expression(
    index: &LicenseIndex, 
    expression: &str
) -> Option<String> {
    let lowered = expression.to_lowercase();
    
    // 1. Check deprecated SPDX expression substitutions FIRST
    if let Some(sub) = get_deprecated_substitution(&lowered) {
        let lowered = sub.to_lowercase();
        // Recursively process the substitution
        return find_matching_rule_for_expression(index, &lowered);
    }
    
    // 2. Direct lookup by SPDX key
    if let Some(&rid) = index.rid_by_spdx_key.get(&lowered) {
        return Some(index.rules_by_rid[rid].license_expression.clone());
    }
    
    // 3. Normalized lookup by rule license_expression
    for rule in &index.rules_by_rid {
        let normalized = normalize_spdx_key(&rule.license_expression);
        if normalized == lowered {
            return Some(rule.license_expression.clone());
        }
    }
    
    // 4. Try parsing as valid expression
    if let Ok(parsed) = parse_expression(expression) {
        if let Some(converted) = convert_spdx_expression_to_scancode(&parsed, index) {
            let result = expression_to_string(&converted);
            if !result.is_empty() {
                return Some(result);
            }
        }
    }
    
    // 5. Check if it's a bare license list (U-Boot style) BEFORE recovery parsing
    if is_bare_license_list(expression) {
        let license_keys = split_license_expression(expression);
        if license_keys.len() > 1 {
            let or_expression = license_keys.join(" OR ");
            if let Ok(parsed) = parse_expression(&or_expression) {
                if let Some(converted) = convert_spdx_expression_to_scancode(&parsed, index) {
                    let result = expression_to_string(&converted);
                    if !result.is_empty() {
                        return Some(result);
                    }
                }
            }
        }
    }
    
    // 6. RECOVERY PARSING for invalid/non-standard expressions
    if let Some(recovered) = reparse_invalid_expression(expression, index) {
        let converted = convert_recovered_expression_to_scancode(&recovered, index);
        let result = expression_to_string(&converted);
        if !result.is_empty() {
            return Some(result);
        }
    }
    
    // 7. Final fallback: unknown-spdx
    index.unknown_spdx_rid
        .map(|rid| index.rules_by_rid[rid].license_expression.clone())
}
```

### Step 6: Handle Text After SPDX Identifier

The current `clean_spdx_text()` function already handles stripping trailing punctuation. However, the issue with text like `LGPL-2.1+ The author added...` is that the recovery tokenizer should only extract valid-looking license identifiers.

Add validation to the recovery tokenizer:

```rust
fn classify_recovery_token(text: &str) -> RecoveryToken {
    match text.to_lowercase().as_str() {
        "and" => RecoveryToken::Keyword(SpdxKeyword::And),
        "or" => RecoveryToken::Keyword(SpdxKeyword::Or),
        "with" => RecoveryToken::Keyword(SpdxKeyword::With),
        _ => {
            // Only treat as license key if it looks like one
            // License keys: alphanumeric with -, ., +, _
            if is_likely_license_key(text) {
                RecoveryToken::LicenseKey(text.to_lowercase())
            } else {
                // Ignore non-license text (e.g., "The", "author", "added")
                RecoveryToken::Ignored
            }
        }
    }
}

fn is_likely_license_key(text: &str) -> bool {
    if text.len() < 2 {
        return false;
    }
    // License keys typically contain:
    // - Alphanumeric characters
    // - Hyphens, periods, plus signs, underscores
    // - At least one digit or known prefix
    let has_valid_chars = text.chars().all(|c| {
        c.is_alphanumeric() || c == '-' || c == '.' || c == '+' || c == '_'
    });
    let looks_like_license = text.chars().any(|c| c.is_ascii_digit())
        || text.starts_with("gpl")
        || text.starts_with("lgpl")
        || text.starts_with("bsd")
        || text.starts_with("mit")
        || text.starts_with("apache")
        || text.starts_with("mpl")
        || text.starts_with("epl")
        || text.starts_with("isc")
        || text.starts_with("unlicense")
        || text.starts_with("cddl")
        || text.starts_with("ecl")
        || text.starts_with("ogc")
        || text.starts_with("ogl")
        || text.starts_with("gfdl")
        || text.starts_with("bsl")
        || text.starts_with("postgresql")
        || text.starts_with("ntp")
        || text.starts_with("licenseref")
        || text.ends_with("+")
        || text.contains('-');
    
    has_valid_chars && looks_like_license
}
```

## Test Cases

### Unit Tests for `reparse_invalid_expression()`

| Test | Input | Expected Output |
|------|-------|-----------------|
| U-Boot bare list | `GPL-2.0+ BSD-2-Clause` | `gpl-2.0+ OR bsd-2-clause` |
| Multiple bare keys | `MIT Apache-2.0 GPL-2.0` | `mit OR apache-2.0 OR gpl-2.0` |
| Unbalanced with keywords | `(GPL-2.0 OR MIT` | `(gpl-2.0 AND mit) AND unknown-spdx` |
| No valid keys | `AND OR WITH` | `unknown-spdx` |
| Single key | `MIT` | `mit` |
| Text after identifier | `LGPL-2.1+ The author` | `lgpl-2.1+` |
| Empty input | `` | `unknown-spdx` |

**Note:** Expected outputs show license keys before conversion to ScanCode expressions. After conversion, `gpl-2.0+` becomes `gpl-2.0-plus`, `bsd-2-clause` becomes `bsd-simplified`, etc.

### Integration Tests

| Test File | Line | SPDX Expression | Expected |
|-----------|------|-----------------|----------|
| `uboot.c` | 2-7 | Multiple unknown identifiers | `unknown-spdx OR unknown-spdx OR ...` (6 total) |
| `uboot.c` | 32 | `GPL-2.0+ BSD-2-Clause` | `gpl-2.0-plus OR bsd-simplified` |
| `uboot.c` | 33 | `GPL-2.0+    BSD-2-Clause` | `gpl-2.0-plus OR bsd-simplified` |
| `uboot.c` | 34 | `GPL-2.0+    BSD-3-Clause` | `gpl-2.0-plus OR bsd-new` |
| `uboot.c` | 36 | `GPL-2.0 IBM-pibs` | `gpl-2.0 OR ibm-pibs` |
| `missing_leading_trailing_paren.txt` | 1 | `(GPL-2.0-ONLY OR MIT` | `gpl-2.0 OR mit` |
| `missing_leading_trailing_paren.txt` | 2 | `GPL-2.0-ONLY OR MIT)` | `gpl-2.0 OR mit` |
| `missing_leading_trailing_paren.txt` | 3 | `(GPL-2.0-ONLY OR (MIT)` | `(gpl-2.0 AND mit) AND unknown-spdx` |

### Golden Test Validation

After implementation:

```bash
cargo test --release -q --lib license_detection::golden_test -- spdx
```

Expected improvements:
- `uboot.c` - All 40 expressions should match
- `missing_leading_trailing_paren.txt` - All 3 expressions should match
- `misc.c` - All 16 expressions should match

## Files to Modify

| File | Changes |
|------|---------|
| `src/license_detection/spdx_lid/mod.rs` | Add `RecoveryToken`, `SpdxKeyword`, `tokenize_for_recovery()`, `classify_recovery_token()`, `is_likely_license_key()`, `reparse_invalid_expression()`, `convert_recovered_expression_to_scancode()`, update `find_matching_rule_for_expression()` |
| `src/license_detection/spdx_lid/test.rs` | Add unit tests for recovery parsing |

## Risk Assessment

**Medium Risk** - Changes are isolated to SPDX expression parsing fallback path. Normal parsing is unaffected. Recovery parsing only activates when standard parsing fails.

### Potential Issues

1. **False positives**: `is_likely_license_key()` may incorrectly classify some words as license keys
2. **Expression complexity**: Nested unbalanced parentheses may produce unexpected results
3. **Performance**: Additional parsing pass for invalid expressions

### Mitigation

1. Be conservative in `is_likely_license_key()` - prefer false negatives
2. Test with real-world SPDX expressions from kernel, U-Boot, etc.
3. Recovery parsing only runs when normal parsing fails, so minimal impact

## Verification Checklist

- [ ] `reparse_invalid_expression()` handles all test cases
- [ ] `convert_recovered_expression_to_scancode()` substitutes unknown keys correctly
- [ ] `find_matching_rule_for_expression()` calls recovery parsing at the right point
- [ ] U-Boot style bare lists produce OR expressions
- [ ] Malformed expressions with keywords append `unknown-spdx`
- [ ] Text after identifiers is ignored
- [ ] Golden tests for `uboot.c` pass
- [ ] Golden tests for `missing_leading_trailing_paren.txt` pass
- [ ] No regressions in valid SPDX expression parsing
- [ ] Full test suite passes

## Verification History

**2026-03-03 - Plan Validated**
- Root cause analysis confirmed accurate
- Code locations verified against actual source
- Python reference behavior matches plan description
- Fixed: Test case line 443 (bare list should be OR not AND)
- Fixed: Code compilation issue with `Option` handling
- Fixed: `RecoveryToken::Ignored` added to properly handle non-license text
- Added: More SPDX prefixes to `is_likely_license_key()`
- Added: LicenseRef-* handling section
- Added: Notes on redundancy with existing bare list detection

## References

- Python implementation: `reference/scancode-toolkit/src/licensedcode/match_spdx_lid.py:271-340`
- Rust implementation: `src/license_detection/spdx_lid/mod.rs:358-417`
- Test data: `testdata/license-golden/datadriven/external/spdx/`
- Related plan: `docs/license-detection/0023-phase7-spdx-parsing-plan.md`

## Additional Notes

### Why Not Fix `split_license_expression()`?

The existing `split_license_expression()` is used elsewhere (e.g., extracting keys for OR fallback in bare list detection). Changing its behavior could cause unintended side effects. Instead, we add a separate `tokenize_for_recovery()` that explicitly handles the recovery use case.

### Why Append `unknown-spdx` for Expressions with Keywords?

Python's logic: if an expression contains keywords (AND/OR/WITH) but doesn't parse correctly, it indicates a structural error. The `unknown-spdx` serves as a "witness" that the expression was invalid. This signals to downstream tools that the detection may be incomplete.

### LicenseRef Handling

`LicenseRef-*` identifiers are non-standard SPDX keys. Python maps them to `unknown-spdx` unless explicitly defined in the SPDX symbols table. Rust should do the same.

### Performance Considerations

Recovery parsing adds minimal overhead:
1. Only invoked when normal parsing fails (rare)
2. Simple tokenization (no regex, just character iteration)
3. Linear in expression length

For typical SPDX expressions (1-5 licenses), overhead is negligible.

### LicenseRef-* Handling

The existing `LicenseExpression::LicenseRef` variant handles `LicenseRef-*` identifiers. In recovery parsing:

1. Tokens starting with `licenseref-` (lowercased from `LicenseRef-`) should create `RecoveryToken::LicenseKey` 
2. `convert_recovered_expression_to_scancode()` already handles `LicenseRef` case
3. Unknown `LicenseRef-*` identifiers map to `unknown-spdx` (same as Python)

### Redundancy with Existing Bare List Detection

The current code at lines 380-393 already handles bare license lists. The recovery parsing approach should:

1. Keep the existing fast-path bare list detection for efficiency
2. Use recovery parsing only when that path fails (e.g., due to unknown keys)
3. Consider consolidating after initial implementation if duplication is problematic
