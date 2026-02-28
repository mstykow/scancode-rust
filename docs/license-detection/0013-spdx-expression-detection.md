# Investigation: SPDX Expression Detection Failures in Golden Tests

**Date**: 2026-02-27  
**Status**: Investigation Complete  
**Priority**: High  

## Summary

This investigation analyzes failing SPDX expression detection golden tests in the `datadriven/external/spdx/` directory. The failures reveal systematic issues in how complex SPDX license expressions (containing OR, AND, WITH operators) are detected, resolved to ScanCode license keys, and combined into detection expressions.

## Test Cases Analyzed

### 1. `complex-readme.txt` - Complex Multi-License File

**Input**: OpenJ9 README containing:
- SPDX-License-Identifier line: `EPL-2.0 OR Apache-2.0 OR GPL-2.0 WITH Classpath-exception-2.0 OR LicenseRef-GPL-2.0 WITH Assembly-exception`
- Full text of multiple licenses (EPL-2.0, Apache-2.0, Unicode, MurmurHash3, libffi, zlib, CuTest)

**Expected** (from `complex-readme.txt.yml`):
```yaml
license_expressions:
  - ((epl-2.0 OR apache-2.0) OR (gpl-2.0 WITH classpath-exception-2.0 AND gpl-2.0 WITH openjdk-exception))
    AND unicode AND public-domain AND mit AND zlib AND zlib
  - epl-2.0 OR apache-2.0 OR gpl-2.0 WITH classpath-exception-2.0 OR gpl-2.0 WITH openjdk-exception
  - epl-2.0 OR apache-2.0
  - unicode
  - unicode
  - public-domain
  - mit
  - zlib
  - zlib
```

**Key Observation**: The expected output has 9 separate license expressions - one combined expression for the SPDX line + full license texts, then individual expressions for each detected license.

### 2. `complex-short.html` - HTML with Multiple SPDX Lines

**Input**: HTML file containing two identical SPDX-License-Identifier lines (lines 23 and 50).

**Expected**:
```yaml
license_expressions:
  - epl-2.0 OR apache-2.0 OR gpl-2.0 WITH classpath-exception-2.0 OR gpl-2.0 WITH openjdk-exception
  - epl-2.0 OR apache-2.0 OR gpl-2.0 WITH classpath-exception-2.0 OR gpl-2.0 WITH openjdk-exception
  - epl-2.0 OR apache-2.0 OR gpl-2.0 WITH classpath-exception-2.0 OR gpl-2.0 WITH openjdk-exception
  - gpl-3.0 WITH autoconf-simple-exception-2.0
  - epl-2.0 OR apache-2.0
  - bsd-new
  - mit
  - ... (more licenses)
```

**Key Observation**: Same SPDX expression detected multiple times from different locations.

### 3. `complex1.c` - C Source File

**Input**: C source file with SPDX-License-Identifier in comment header.

**Expected**:
```yaml
license_expressions:
  - epl-2.0 OR apache-2.0 OR gpl-2.0 WITH classpath-exception-2.0 OR gpl-2.0 WITH openjdk-exception
  - epl-2.0 OR apache-2.0 OR gpl-2.0 WITH classpath-exception-2.0 OR gpl-2.0 WITH openjdk-exception
```

**Key Observation**: Two detections expected - one from SPDX tag, one from license text detection.

### 4. `gpl-2.0-plus_or_linux-openib_SPDX.RULE` - OR Expression

**Input**:
```
SPDX-License-Identifier: (GPL-2.0+  OR Linux-Openib)
```

**Expected**:
```yaml
license_expressions:
  - gpl-2.0-plus OR linux-openib
```

**Key Observation**: Simple OR expression with parentheses and whitespace normalization.

## Root Cause Analysis

### Issue 1: SPDX Expression Resolution Not Preserving Complex Structure

**Location**: `src/license_detection/spdx_lid.rs:316-346` (`find_matching_rule_for_expression`)

**Problem**: The current implementation in `find_matching_rule_for_expression` attempts to find a single rule for an entire SPDX expression. When the expression contains OR/AND/WITH operators, it falls back to:

1. Looking up only the first license key in the expression
2. Returning that single rule's license_expression

This loses the complex expression structure. For example:
- Input: `EPL-2.0 OR Apache-2.0 OR GPL-2.0 WITH Classpath-exception-2.0`
- Current behavior: Finds `epl-2.0` rule, returns `license_expression: "epl-2.0"`
- Expected behavior: Should return the parsed expression structure

**Python Reference Behavior** (`reference/scancode-toolkit/src/licensedcode/match_spdx_lid.py:87-97`):
```python
rule = SpdxRule(
    license_expression=expression_str,  # The FULL parsed expression
    text=text,
    length=match_len,
)
```

Python creates a `SpdxRule` with the full expression string preserved, not a single license key.

### Issue 2: Expression Not Parsed from SPDX Format

**Location**: `src/license_detection/spdx_lid.rs:248-260`

**Problem**: The code extracts the SPDX expression text but doesn't parse it properly into a ScanCode license expression. The flow is:

```rust
let (_, expression) = split_spdx_lid(spdx_text);
let spdx_expression = clean_spdx_text(&expression);
// ... then find_matching_rule_for_expression() only looks up first key
```

**Missing Step**: The SPDX license keys need to be converted to ScanCode license keys. For example:
- `GPL-2.0+` → `gpl-2.0-plus`
- `Linux-Openib` → `linux-openib`
- `Classpath-exception-2.0` → `classpath-exception-2.0`

The conversion should preserve the expression structure (OR, AND, WITH operators).

### Issue 3: LicenseRef-* Handling Missing

**Location**: `src/license_detection/spdx_lid.rs` (missing)

**Problem**: The test case `complex-readme.txt` contains:
```
LicenseRef-GPL-2.0 WITH Assembly-exception
```

This is a non-standard SPDX identifier that should map to a ScanCode license key. The Python implementation handles this via `expression_symbols` mapping and `unknown_symbol` fallback.

### Issue 4: Detection Output Format Mismatch

**Location**: `src/license_detection/golden_test.rs:147-151`

**Problem**: The golden test compares `license_expression` from each match:
```rust
let actual: Vec<&str> = detections
    .iter()
    .flat_map(|d| d.matches.iter())
    .map(|m| m.license_expression.as_str())
    .collect();
```

But the expected YAML expects **one expression per detection**, not one expression per match. The test flattens all matches' expressions, but the expected data has one expression per detection group.

**Example Mismatch**:
- Expected: `["epl-2.0 OR apache-2.0 OR gpl-2.0 WITH classpath-exception-2.0"]` (1 detection)
- Actual might be: `["epl-2.0", "apache-2.0", "gpl-2.0", "classpath-exception-2.0"]` (4 matches)

### Issue 5: WITH Exception Handling

**Location**: `src/license_detection/spdx_lid.rs` and `src/license_detection/expression.rs`

**Problem**: The `WITH` operator in SPDX expressions needs special handling:
- `GPL-2.0 WITH Classpath-exception-2.0` should remain as a single unit
- Currently, `split_license_expression()` strips AND/OR/WITH, treating WITH as a separator

**Code Evidence** (`src/license_detection/spdx_lid.rs:195-222`):
```rust
fn split_license_expression(license_expression: &str) -> Vec<String> {
    // ...
    tokens
        .into_iter()
        .filter(|t| {
            let t_lower = t.to_lowercase();
            !matches!(t_lower.as_str(), "and" | "or" | "with")  // WITH is filtered out!
        })
        .collect()
}
```

This incorrectly splits `GPL-2.0 WITH Classpath-exception-2.0` into separate keys.

### Issue 6: Plus (+) Suffix Handling

**Location**: `src/license_detection/spdx_lid.rs`

**Problem**: SPDX uses `GPL-2.0+` notation for "or later" versions, but ScanCode uses `gpl-2.0-plus`. The conversion table `DEPRECATED_SPDX_EXPRESSION_SUBS` handles some cases but not `+` suffix.

**Test Case**: `gpl-2.0-plus_or_linux-openib_SPDX.RULE` expects `gpl-2.0-plus` from `GPL-2.0+`.

## Proposed Solution Approach

### Solution 1: Implement Full SPDX Expression Parsing

**Files to modify**:
- `src/license_detection/spdx_lid.rs`

**Approach**:
1. Use the existing `expression.rs` parser to parse the SPDX expression text
2. Map each SPDX license key to its ScanCode equivalent using `spdx_mapping`
3. Preserve the expression structure (OR, AND, WITH operators)
4. Handle `LicenseRef-*` identifiers by mapping to known keys or using `unknown-spdx`

**Code location**: Modify `find_matching_rule_for_expression()` to:
```rust
fn find_matching_rule_for_expression(
    index: &LicenseIndex,
    expression: &str,
    spdx_mapping: &SpdxMapping,
) -> Option<String> {
    // Parse the expression using expression.rs parser
    let parsed = parse_expression(expression)?;
    
    // Convert each SPDX key to ScanCode key
    let converted = convert_spdx_keys_to_scancode(&parsed, index, spdx_mapping)?;
    
    // Return the full expression string
    Some(expression_to_string(&converted))
}
```

### Solution 2: Fix WITH Exception Handling

**Files to modify**:
- `src/license_detection/spdx_lid.rs`
- Potentially `src/license_detection/expression.rs`

**Approach**:
1. Remove `WITH` from the filter list in `split_license_expression()` - but this may be intentional for fallback
2. Instead, ensure the expression parser correctly handles WITH as a binary operator
3. When looking up rules, keep `license_a WITH exception_b` as a unit

### Solution 3: Add Plus (+) Suffix Conversion

**Files to modify**:
- `src/license_detection/spdx_lid.rs`

**Approach**:
Add to the conversion logic:
```rust
fn normalize_spdx_key_with_plus(key: &str) -> String {
    if key.ends_with('+') {
        let base = &key[..key.len()-1];
        // Map GPL-2.0+ -> gpl-2.0-plus, etc.
        format!("{}-plus", base.to_lowercase().replace("_", "-"))
    } else {
        normalize_spdx_key(key)
    }
}
```

### Solution 4: Create SpdxRule Equivalent

**Files to modify**:
- `src/license_detection/models.rs` (new synthetic rule type)
- `src/license_detection/spdx_lid.rs`

**Approach**:
Like Python's `SpdxRule`, create a synthetic rule that:
1. Stores the full parsed expression
2. Has `is_license_tag = true`
3. Has `relevance = 100`
4. Doesn't require a physical rule file

### Solution 5: Fix Golden Test Comparison

**Files to modify**:
- `src/license_detection/golden_test.rs`

**Approach**:
The test should compare detection-level expressions, not match-level:
```rust
let actual: Vec<&str> = detections
    .iter()
    .map(|d| d.license_expression.as_deref().unwrap_or(""))
    .collect();
```

Not flatten all matches. This matches how the Python tests work.

## Code Locations Needing Investigation

| File | Lines | Issue |
|------|-------|-------|
| `src/license_detection/spdx_lid.rs` | 316-346 | `find_matching_rule_for_expression` - returns single key instead of full expression |
| `src/license_detection/spdx_lid.rs` | 244-314 | `spdx_lid_match` - creates LicenseMatch with wrong expression |
| `src/license_detection/spdx_lid.rs` | 195-222 | `split_license_expression` - incorrectly filters WITH |
| `src/license_detection/spdx_lid.rs` | 152-193 | `DEPRECATED_SPDX_EXPRESSION_SUBS` - missing `+` suffix handling |
| `src/license_detection/golden_test.rs` | 147-151 | Test compares match-level expressions, should be detection-level |
| `src/license_detection/detection.rs` | 666-678 | `determine_license_expression` - combines with AND, but SPDX may have OR |
| `src/license_detection/expression.rs` | 548-592 | `expression_to_string` - verify WITH operator precedence |

## Comparison with Python Implementation

### Python Flow (reference/scancode-toolkit/src/licensedcode/match_spdx_lid.py)

1. **Parse SPDX expression** (lines 171-176): Uses `licensing.parse()` with full expression string
2. **Substitute deprecated keys** (lines 243-245): Maps old SPDX IDs to new expressions
3. **Convert symbols** (lines 252-267): Each SPDX symbol → ScanCode license symbol
4. **Create SpdxRule** (lines 87-97): Synthetic rule with full expression preserved
5. **Return match** (lines 109-119): Match contains the full expression

### Rust Flow (current)

1. **Split SPDX identifier** (`split_spdx_lid`): Extracts expression text
2. **Clean text** (`clean_spdx_text`): Removes markup, normalizes
3. **Find matching rule** (`find_matching_rule_for_expression`): **BUG** - only looks up first key
4. **Create match**: Uses found rule's `license_expression` - loses original structure

### Key Difference

Python preserves the expression structure throughout. Rust collapses it to a single license key.

## Testing Recommendations

1. **Unit test**: `find_matching_rule_for_expression` with complex expressions
2. **Unit test**: `split_license_expression` should NOT filter WITH
3. **Integration test**: Full detection pipeline with `complex-readme.txt`
4. **Regression test**: Ensure `GPL-2.0+` → `gpl-2.0-plus` conversion works

## Next Steps

1. Implement Solution 1 (Full SPDX Expression Parsing) as the primary fix
2. Add Solution 3 (Plus suffix conversion) as part of key normalization
3. Verify with the failing test cases
4. Consider Solution 4 (SpdxRule equivalent) for proper architecture alignment

## Appendix: Test File Locations

- `testdata/license-golden/datadriven/external/spdx/complex-readme.txt`
- `testdata/license-golden/datadriven/external/spdx/complex-short.html`
- `testdata/license-golden/datadriven/external/spdx/complex1.c`
- `testdata/license-golden/datadriven/external/spdx/gpl-2.0-plus_or_linux-openib_SPDX.RULE`
- `testdata/license-golden/datadriven/external/spdx/complex3.java`
