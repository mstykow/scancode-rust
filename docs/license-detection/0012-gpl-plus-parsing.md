# PLAN-0012: GPL-2.0+ Style License Identifier Parsing

## Problem Description

The SPDX expression parser in `src/license_detection/expression.rs` fails to parse license identifiers with a `+` suffix (like `GPL-2.0+`), causing parsing errors and incorrect license expression handling.

### Specific Code Locations

1. **Tokenizer in expression.rs:669-716**
   - The `tokenize()` function only accepts alphanumeric characters, `-`, `.`, and `_` in license identifiers (line 692-698)
   - The `+` character is NOT included in the allowed character set
   - Result: `GPL-2.0+` causes `UnexpectedToken { token: "+", position: 7 }`

2. **Ignored test in expression.rs:1406-1410**
   ```rust
   #[test]
   #[ignore]
   fn test_parse_gpl_plus_license() {
       let expr = parse_expression("GPL-2.0+").unwrap();
       assert_eq!(expr, LicenseExpression::License("gpl-2.0+".to_string()));
   }
   ```

3. **Golden test failures** for files like `gpl-2.0-plus_or_linux-openib_SPDX.RULE`:
   - Input: `SPDX-License-Identifier: (GPL-2.0+ OR Linux-Openib)`
   - Expected: `gpl-2.0-plus OR linux-openib`
   - Actual: `gpl-2.0-plus` (missing `OR linux-openib`)

## Root Cause Analysis

### Expression Parser Tokenizer Issue

The expression parser's `tokenize()` function (line 692) uses this condition:

```rust
if c.is_alphanumeric() || c == '-' || c == '.' || c == '_' {
    let start = pos;
    while pos < chars.len()
        && (chars[pos].is_alphanumeric()
            || chars[pos] == '-'
            || chars[pos] == '.'
            || chars[pos] == '_')
    {
        pos += 1;
    }
    // ...
}
```

The `+` character is not included, so when parsing `GPL-2.0+`, it:
1. Successfully parses `GPL-2.0` as a license token
2. Encounters `+` and throws `UnexpectedToken` error
3. The entire expression parse fails

### Python Reference Implementation

The Python tokenizer in `reference/scancode-toolkit/src/licensedcode/tokenize.py:72-79` handles `+`:

```python
# Split on whitespace and punctuations: keep only characters and numbers and +
# when in the middle or end of a word. Keeping the trailing + is important for
# licenses name such as GPL2+.
query_pattern = '[^_\\W]+\\+?[^_\\W]*'
```

The regex `[^_\\W]+\\+?[^_\\W]*` allows:
- One or more alphanumeric characters
- An optional `+` (important for licenses like GPL2+)
- Zero or more additional alphanumeric characters

### Current Workarounds in Codebase

The codebase has workarounds for `+` suffix handling in other places:

1. **SPDX-LID matching** (`spdx_lid.rs:317-345`): Uses `rid_by_spdx_key` hashmap to map `gpl-2.0+` to `gpl-2.0-plus` rules

2. **Index builder** (`index/builder.rs:415-420`): Stores SPDX keys with `+` in `rid_by_spdx_key`:
   ```rust
   if let Some(ref spdx_key) = rule.spdx_license_key {
       rid_by_spdx_key.insert(spdx_key.to_lowercase(), rid);
   }
   ```

3. **Test verification** (`spdx_lid.rs:872-882`): Confirms `gpl-2.0+` maps to `gpl-2.0-plus`:
   ```rust
   assert!(index.rid_by_spdx_key.contains_key("gpl-2.0+"));
   assert_eq!(rule.license_expression, "gpl-2.0-plus");
   ```

However, the expression parser itself cannot parse these identifiers.

## Proposed Solution Approach

### Option A: Add `+` to Expression Parser Tokenizer (Recommended)

Add `+` as a valid character in license identifiers within the expression parser's tokenizer:

```rust
if c.is_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '+' {
    let start = pos;
    while pos < chars.len()
        && (chars[pos].is_alphanumeric()
            || chars[pos] == '-'
            || chars[pos] == '.'
            || chars[pos] == '_'
            || chars[pos] == '+')
    {
        pos += 1;
    }
    // ...
}
```

**Pros:**
- Minimal change, single location
- Directly matches Python behavior
- Handles all `+` suffix cases (GPL-2.0+, LGPL-3.0+, etc.)
- Preserves `+` in the parsed identifier for downstream normalization

**Cons:**
- May need to handle cases where `+` appears in the middle of identifiers
- Requires normalization step to convert `GPL-2.0+` to `gpl-2.0-plus` for matching

### Option B: Pre-normalize Before Parsing

Add a pre-processing step to convert `+` suffixes to `-plus` before parsing:

```rust
pub fn parse_expression(expr: &str) -> Result<LicenseExpression, ParseError> {
    let normalized = normalize_plus_suffix(expr);  // GPL-2.0+ -> GPL-2.0-plus
    let trimmed = normalized.trim();
    // ... rest of parsing
}
```

**Pros:**
- Keeps tokenizer simpler
- Normalizes to ScanCode key format early

**Cons:**
- Requires regex or string manipulation
- May have edge cases with `+` in unexpected positions
- Changes the parsed expression string from original input

### Recommended: Option A with Post-Parse Normalization

1. Add `+` to the tokenizer (Option A)
2. Add normalization function to convert `+` suffix to `-plus` when looking up licenses
3. Preserve original parsed form for error messages and debugging

## Test Cases

### Unit Tests (from ignored test)

```rust
#[test]
fn test_parse_gpl_plus_license() {
    let expr = parse_expression("GPL-2.0+").unwrap();
    assert_eq!(expr, LicenseExpression::License("gpl-2.0+".to_string()));
}

#[test]
fn test_parse_gpl_plus_lowercase() {
    let expr = parse_expression("gpl-2.0+").unwrap();
    assert_eq!(expr, LicenseExpression::License("gpl-2.0+".to_string()));
}

#[test]
fn test_parse_gpl_plus_with_or() {
    let expr = parse_expression("GPL-2.0+ OR MIT").unwrap();
    assert!(matches!(expr, LicenseExpression::Or { .. }));
    assert_eq!(expression_to_string(&expr), "gpl-2.0+ OR mit");
}

#[test]
fn test_parse_multiple_plus_licenses() {
    let expr = parse_expression("GPL-2.0+ AND LGPL-3.0+").unwrap();
    assert!(matches!(expr, LicenseExpression::And { .. }));
}

#[test]
fn test_parse_plus_in_middle() {
    // Edge case: + in middle of identifier
    let expr = parse_expression("GPL-2.0+extra").unwrap();
    assert_eq!(expr, LicenseExpression::License("gpl-2.0+extra".to_string()));
}
```

### Golden Test Files

Files that should pass after fix:

1. `testdata/license-golden/datadriven/external/spdx/gpl-2.0-plus_or_linux-openib_SPDX.RULE`
   - Input: `SPDX-License-Identifier: (GPL-2.0+  OR Linux-Openib)`
   - Expected expression: `gpl-2.0-plus OR linux-openib`

2. `testdata/license-golden/datadriven/external/spdx/gpl-2.0-plus_or_linux-openib_SPDX2.RULE`
   - Same as above, alternate format

3. `testdata/license-golden/datadriven/external/spdx/gpl-2.0-plus_with_linux-syscall-note_or_linux-openib_SPDX.RULE`
   - Input: `SPDX-License-Identifier: GPL-2.0+ WITH Linux-syscall-note OR Linux-Openib`
   - Expected: `gpl-2.0-plus WITH linux-syscall-exception-gpl OR linux-openib`

## Implementation Steps

### Step 1: Update Tokenizer (expression.rs)

Location: `src/license_detection/expression.rs:692-698`

```rust
// Before:
if c.is_alphanumeric() || c == '-' || c == '.' || c == '_' {

// After:
if c.is_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '+' {
```

And in the while loop condition:

```rust
// Before:
while pos < chars.len()
    && (chars[pos].is_alphanumeric()
        || chars[pos] == '-'
        || chars[pos] == '.'
        || chars[pos] == '_')

// After:
while pos < chars.len()
    && (chars[pos].is_alphanumeric()
        || chars[pos] == '-'
        || chars[pos] == '.'
        || chars[pos] == '_'
        || chars[pos] == '+')
```

### Step 2: Add Normalization for SPDX Key Lookup

The SPDX-LID match code already handles `GPL-2.0+` -> `gpl-2.0-plus` conversion via the `rid_by_spdx_key` hashmap. Verify that the expression normalization doesn't break this.

If needed, add a normalization function:

```rust
/// Normalize license key, converting + suffix to -plus for ScanCode compatibility.
fn normalize_plus_suffix(key: &str) -> String {
    if key.ends_with('+') {
        format!("{}-plus", &key[..key.len()-1])
    } else {
        key.to_string()
    }
}
```

### Step 3: Enable Ignored Test

Remove `#[ignore]` from `test_parse_gpl_plus_license` and verify it passes.

### Step 4: Add Additional Tests

Add comprehensive tests for `+` suffix handling:
- Simple cases (GPL-2.0+, LGPL-3.0+)
- In expressions (GPL-2.0+ OR MIT)
- With WITH operator (GPL-2.0+ WITH exception)
- Edge cases (+ in middle, multiple +)

### Step 5: Run Golden Tests

```bash
cargo test test_extract_from_testdata -- --nocapture
```

Verify that the SPDX golden tests for `gpl-2.0-plus` files pass.

## Validation Approach

### 1. Unit Test Validation

```bash
cargo test test_parse_gpl_plus_license -- --ignored --nocapture
```

Should pass without `UnexpectedToken` error.

### 2. Golden Test Validation

```bash
cargo test test_extract_from_testdata -- --nocapture 2>&1 | grep -A5 "gpl-2.0-plus"
```

Verify expected expressions include the full OR/AND clauses.

### 3. Expression Round-Trip Test

```rust
#[test]
fn test_plus_suffix_roundtrip() {
    let input = "GPL-2.0+ OR MIT";
    let expr = parse_expression(input).unwrap();
    let output = expression_to_string(&expr);
    assert_eq!(output, "gpl-2.0+ OR mit");
}
```

### 4. SPDX-LID Match Integration

Verify that `GPL-2.0+` in SPDX-License-Identifier tags correctly matches `gpl-2.0-plus` rules.

## Edge Cases to Consider

1. **`+` at start of identifier**: `+GPL-2.0` - should this be valid?
   - Python regex `[^_\\W]+\\+?[^_\\W]*` requires alphanumeric first
   - Should reject or handle gracefully

2. **Multiple `+` in identifier**: `GPL++2.0` or `GPL-2.0++`
   - Python regex allows only one optional `+`
   - Our implementation may need similar constraint

3. **`+` in middle of identifier**: `GPL+2.0`
   - Python regex handles this
   - May not be a real license identifier but should parse

4. **Case sensitivity**: `GPL-2.0+` vs `gpl-2.0+`
   - Already handled by `to_lowercase()` in tokenizer

## Related Files

- `src/license_detection/expression.rs` - Main parser (fix location)
- `src/license_detection/tokenize.rs` - Text tokenizer (already handles `+`)
- `src/license_detection/spdx_lid.rs` - SPDX-LID matching
- `src/license_detection/index/builder.rs` - Index building with SPDX aliases
- `reference/scancode-toolkit/src/licensedcode/tokenize.py` - Python reference

## References

- Python tokenizer: `reference/scancode-toolkit/src/licensedcode/tokenize.py:72-79`
- SPDX expression parsing: `reference/scancode-toolkit/src/licensedcode/match_spdx_lid.py:226-268`
- Test file: `testdata/license-golden/datadriven/external/spdx/gpl-2.0-plus_or_linux-openib_SPDX.RULE`
