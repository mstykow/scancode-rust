# PLAN-055: Expression Normalization

## Status: NOT IMPLEMENTED

## Summary

Python has expression normalization/simplification logic that Rust lacks. Complex expressions may be simplified differently.

---

## Problem Statement

**Example**: Python normalizes `lgpl-2.1 WITH exception OR cpl-1.0 WITH exception` to `lzma-sdk-2006` (based on SPDX mapping).

**Rust**: Does not have this normalization layer.

---

## Detailed Investigation Findings

### The lzma-sdk-2006 Case Study

The LZMA SDK license is a **composite license** - a single license that represents a choice between LGPL 2.1 or CPL 1.0, each with a special exception.

**Files involved:**

| File | Location | Key Data |
|------|----------|----------|
| `lzma-sdk-2006.LICENSE` | `src/licensedcode/data/licenses/` | `key: lzma-sdk-2006`, full license text |
| `lzma-sdk-2006-exception.LICENSE` | `src/licensedcode/data/licenses/` | `key: lzma-sdk-2006-exception`, exception text |
| `cpl_or_lgpl_with_lzma-sdk-2006-exception.RULE` | `src/licensedcode/data/rules/` | `license_expression: lgpl-2.1 WITH lzma-sdk-2006-exception OR cpl-1.0 WITH lzma-sdk-2006-exception` |

**How Python handles this:**

1. **License text indexing**: The `lzma-sdk-2006.LICENSE` file is indexed and creates a rule with:
   - `license_expression: lzma-sdk-2006` (the license key)
   - `is_from_license: True`
   - `is_license_text: True`
   - `relevance: 100` (always 100 for license text rules)

2. **Rule-based matching**: The `cpl_or_lgpl_with_lzma-sdk-2006-exception.RULE` matches partial text with expression:
   - `license_expression: lgpl-2.1 WITH lzma-sdk-2006-exception OR cpl-1.0 WITH lzma-sdk-2006-exception`

3. **Priority/selection**: When the FULL license text is matched against the `lzma-sdk-2006.LICENSE` rule (which has `relevance=100` and `is_from_license=True`), Python returns `lzma-sdk-2006` as the expression, NOT the decomposed complex expression.

**Key Python code paths:**

- `build_rule_from_license()` in `models.py:1136-1164` creates rules from LICENSE files
- Rule priority uses `relevance` field - license text rules always have `relevance=100`
- The `is_from_license` flag identifies rules generated from license text files

### What "Normalization" Actually Means

This is **NOT** about algebraic expression simplification. It's about:

1. **License text rule priority**: Rules generated from `.LICENSE` files (`is_from_license=True`) take precedence because:
   - They have `relevance=100`
   - They represent the full, canonical license text
   - Their `license_expression` is the license key itself (not decomposed components)

2. **Composite license documentation**: Many licenses have `notes: composite of X AND Y` or similar:
   - `lzma-sdk-2006`: composite of LGPL 2.1 or CPL 1.0 with exception
   - `brian-gladman-dual`: composite of gpl-2.0-plus AND brian-gladman-3-clause
   - `dejavu-font`: composite of a double bitstream
   - ~29 composite licenses exist in the database

3. **Expression-to-license-key mapping**: When a rule matches license text, the output is the license key, not the raw expression from the rule.

### The license-expression Library

Python uses the `license-expression` library (version 30.4.4) for:
- Parsing SPDX expressions
- Key validation
- `combine_expressions()` - combines multiple expressions with deduplication
- `licensing.contains()` - expression subsumption checks

**Key functions used in ScanCode:**
- `combine_expressions(expressions, relation='OR', licensing=licensing)` - combines expressions with deduplication
- `Licensing.license_keys(expression)` - extracts license keys from expression
- `Licensing.contains(expression1, expression2)` - checks if expr1 contains expr2

### Rust's Current State

**What Rust has (`src/license_detection/expression.rs`):**
- `parse_expression()` - parses license expression strings into AST
- `simplify_expression()` - deduplicates licenses in AND/OR expressions
- `licensing_contains()` - expression subsumption checks
- `expression_to_string()` - serializes AST back to string

**What Rust is missing:**
- Rule priority based on `relevance` and `is_from_license` flag
- Composite license handling at detection level
- License text rule preference over rule-based matches

---

## Impact Assessment

- **~30+ tests with complex expressions** - Tests where license text matches should return the license key, not decomposed expressions
- **Expression outputs may differ from Python** when:
  - A composite license is detected via partial text match vs full license text match
  - Multiple rules with different expressions match the same text

---

## Implementation Requirements

### 1. Rule Priority System

**Location**: `src/license_detection/match_refine.rs` or `src/license_detection/detection.rs`

Rules should be prioritized by:
1. `relevance` (higher is better, license text rules have 100)
2. `is_from_license` flag (True = higher priority)
3. `is_license_text` flag (True = higher priority)

**Implementation approach:**
```rust
fn compare_rule_priority(a: &LicenseMatch, b: &LicenseMatch) -> Ordering {
    // 1. Higher relevance wins
    b.rule_relevance.cmp(&a.rule_relevance)
        // 2. is_from_license wins
        .then_with(|| b.is_from_license.cmp(&a.is_from_license))
        // 3. is_license_text wins
        .then_with(|| b.is_license_text.cmp(&a.is_license_text))
        // 4. Coverage (higher wins)
        .then_with(|| b.match_coverage.partial_cmp(&a.match_coverage).unwrap_or(Ordering::Equal))
}
```

### 2. Composite License Detection

When a license is documented as composite:
- Full text match should return the composite license key
- Partial match may return decomposed expression

**Data requirements:**
- Load `notes` field from LICENSE files
- Parse "composite of X AND Y" or "composite of X OR Y" patterns
- Store composite mapping for expression normalization

### 3. License Text Rule Preference

In `filter_contained_matches()` and `filter_overlapping_matches()`:
- When matches have equal/highly overlapping spans
- Prefer rules with `is_from_license=True` and `relevance=100`
- This ensures full license text matches return the license key

### 4. Expression Key Mapping

Create mapping from complex expressions to license keys:
- `lgpl-2.1 WITH lzma-sdk-2006-exception OR cpl-1.0 WITH lzma-sdk-2006-exception` → `lzma-sdk-2006`
- This requires parsing LICENSE file notes or creating explicit mapping

---

## Current Implementation State

### Rust Already Has

**In `Rule` struct (`src/license_detection/models.rs`):**
- `relevance: u8` (line 110) - Relevance score 0-100
- `is_from_license: bool` (line 107) - True if rule created from license file
- `is_license_text: bool` (line 81) - True if full license text

**In `LicenseMatch` struct (`src/license_detection/models.rs`):**
- `rule_relevance: u8` (line 256) - Copied from matched rule
- `is_license_text: bool` (line 287) - Copied from matched rule

### Missing in LicenseMatch

- `is_from_license: bool` - **NOT present** in LicenseMatch, needs to be added
  - This is needed to prioritize license text matches in filtering functions

### Python Reference Confirmed

In `reference/scancode-toolkit/src/licensedcode/models.py:1136-1164`:
```python
def build_rule_from_license(license_obj):
    rule = Rule(
        ...
        relevance=100,           # Always 100 for license text
        is_from_license=True,    # Always True for license text
        is_license_text=True,    # Always True for license text
        ...
    )
```

---

## Key Files to Modify

| File | Changes |
|------|---------|
| `src/license_detection/models.rs` | Add `is_from_license` field to LicenseMatch struct (Rule already has it) |
| `src/license_detection/match_refine.rs` | Add rule priority comparison in filter functions |
| `src/license_detection/detection.rs` | Populate `is_from_license` when creating LicenseMatch from Rule |
| `src/license_detection/license_db.rs` | Load `notes` field and parse composite mappings (already partially done) |

---

## Testing Strategy

Following `docs/TESTING_STRATEGY.md`:

### Unit Tests

**Location**: `src/license_detection/expression_test.rs` and `src/license_detection/match_refine_test.rs`

```rust
#[test]
fn test_rule_priority_is_from_license() {
    // License text rule should beat rule-based match
    let license_rule = create_test_rule(is_from_license: true, relevance: 100);
    let partial_rule = create_test_rule(is_from_license: false, relevance: 50);
    assert_eq!(compare_rule_priority(&license_rule, &partial_rule), Ordering::Greater);
}

#[test]
fn test_composite_license_detection() {
    // lzma-sdk-2006 full text should return "lzma-sdk-2006"
    let result = detect_licenses("testdata/license-golden/datadriven/lic3/lzma-sdk-original.txt");
    assert_eq!(result.license_expressions, vec!["lzma-sdk-2006"]);
}
```

### Golden Tests

**Location**: `src/license_detection/golden_test.rs` (existing)

Focus on these test files:
- `testdata/license-golden/datadriven/lic3/lzma-sdk-original*.txt`
- Other composite license tests

### Integration Tests

**Location**: `tests/scanner_integration.rs`

Test end-to-end license detection with composite licenses:
```rust
#[test]
fn test_composite_license_full_text_match() {
    let result = scan_directory("testdata/license-golden/datadriven/lic3/");
    // Verify lzma-sdk-2006 detection returns license key, not decomposed expression
}
```

---

## Verification Checklist

- [x] Rule struct has `relevance`, `is_from_license`, `is_license_text` fields (ALREADY IMPLEMENTED)
- [ ] LicenseMatch struct has `is_from_license` field (NEEDS TO BE ADDED)
- [ ] License text rules load with `relevance=100` and `is_from_license=True`
- [ ] `filter_contained_matches()` prefers `is_from_license` rules
- [ ] `filter_overlapping_matches()` prefers higher relevance rules
- [ ] lzma-sdk golden tests pass (return `lzma-sdk-2006` not decomposed expression)
- [ ] Other composite license tests pass
- [ ] No regressions in non-composite license tests

---

## Related Plans

- PLAN-045: Expression Selection Parity for Overlapping Matches (related - also deals with expression selection)
- PLAN-029 section 2.6 (original reference)

---

## Priority: MEDIUM

Complex feature requiring significant investigation. The core issue is rule priority and license text preference, not algebraic simplification.

---

## Verification Summary (2026-02-25)

**Plan Approach: CORRECT**

1. **Mechanism identification**: ✅ The plan correctly identifies this as rule priority based on relevance, NOT algebraic expression normalization.

2. **Rust has relevance and is_from_license fields**: 
   - ✅ `Rule.relevance: u8` (line 110)
   - ✅ `Rule.is_from_license: bool` (line 107)
   - ✅ `Rule.is_license_text: bool` (line 81)
   - ⚠️ `LicenseMatch.is_from_license` is **NOT present** - only `LicenseMatch.is_license_text` exists

3. **Python uses relevance=100 for license text rules**: ✅ Confirmed at `models.py:1148` in `build_rule_from_license()`

4. **Implementation approach**: ✅ Priority comparison is correct, but needs `is_from_license` in LicenseMatch

5. **Testing strategy**: ✅ Follows TESTING_STRATEGY.md (unit tests + golden tests)

**Action Required**: Add `is_from_license: bool` field to `LicenseMatch` struct and populate it from the matched Rule.

---

## Appendix: Composite Licenses in Database

Licenses with `notes: composite of ...`:

| License Key | Composite Notes |
|-------------|-----------------|
| `lzma-sdk-2006` | composite of lgpl-2.1 WITH lzma-sdk-2006-exception OR cpl-1.0 WITH lzma-sdk-2006-exception |
| `brian-gladman-dual` | composite of gpl-2.0-plus AND brian-gladman-3-clause |
| `dejavu-font` | composite of a double bitstream |
| `doug-lea` | composite of several public domain dedications |
| `ibm-icu` | composite of x11 licenses and others |
| `jython` | complex composite of multiple licenses |
| `kerberos` | complex composite |
| `madwifi-dual` | composite replaced by expression of intel-bsd OR gpl-2.0 |
| (and ~20 more) | |

Full list can be found by: `grep -r "notes:.*composite" reference/scancode-toolkit/src/licensedcode/data/licenses/`
