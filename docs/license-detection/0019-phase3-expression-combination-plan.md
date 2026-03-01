# Phase 3: License Expression Combination - Implementation Plan

**Status:** Investigation Complete - Ready for Implementation  
**Created:** 2026-03-01  
**Updated:** 2026-03-01  
**Parent Roadmap:** `docs/license-detection/0016-feature-parity-roadmap.md`

## Executive Summary

### Problem Statement

When multiple license matches are found in proximity, their expressions should be combined correctly using AND/OR logic. Currently, Rust incorrectly combines dual-license (OR) expressions as separate AND-combined licenses.

**Example Failure:**
- **File:** `BSL-1.0_or_MIT.txt`
- **Expected:** `["mit OR boost-1.0"]`
- **Actual:** `["mit", "boost-1.0"]` (combined as "mit AND boost-1.0")

### Root Cause (CONFIRMED)

**URL protocol mismatch prevents dual-license rule matching:**
- Rule `mit_or_boost-1.0_1.RULE` has URL `https://www.boost.org/LICENSE_1_0.txt`
- Test file `BSL-1.0_or_MIT.txt` has URL `http://www.boost.org/LICENSE_1_0.txt`
- When tokenized: `https` ≠ `http` → no exact match possible
- The rule declares `ignorable_urls` but this is NOT used during matching

### Solution

**Implement URL normalization for `ignorable_urls`:**
1. During rule indexing, normalize ignorable URLs (strip http/https protocol)
2. This allows flexible matching on URL variations
3. The dual-license rule will then match, preserving its `mit OR boost-1.0` expression

### Investigation Findings

**✓ VERIFIED: The infrastructure works correctly:**
- `LicenseMatch.license_expression` IS set from `rule.license_expression.clone()` (verified in all matchers)
- `combine_expressions()` correctly handles embedded OR expressions
- Expression parsing and serialization works correctly

**⚠️ ROOT CAUSE: Dual-license rules are not matching:**
- File `BSL-1.0_or_MIT.txt` matches individual `mit` and `boost-1.0` rules
- The dual-license rule `mit_or_boost-1.0_1.RULE` exists but is not matching
- **SPECIFIC FINDING:** URL protocol mismatch - file uses `http://` but rule has `https://`
  - Rule text: `https://www.boost.org/LICENSE_1_0.txt`
  - File text: `http://www.boost.org/LICENSE_1_0.txt`
  - When tokenized: `https` ≠ `http` → no hash match
  - The rule has `ignorable_urls: [https://www.boost.org/LICENSE_1_0.txt]` but this is not being applied during matching

### Root Cause

**UPDATED AFTER INVESTIGATION:** The matchers correctly preserve `rule.license_expression` (verified in `hash_match.rs:100`, `aho_match.rs:161`, `seq_match.rs:818`). The real issue is that **multiple different rules** match the same file, each with single-license expressions, rather than a single dual-license rule matching.

**Example:**
- File `BSL-1.0_or_MIT.txt` contains MIT-style text AND Boost reference text
- A `mit` rule matches the MIT portion → expression: "mit"
- A `boost-1.0` rule matches the Boost portion → expression: "boost-1.0"
- These are two DIFFERENT rules, each with single-license expressions
- When combined with AND: "mit AND boost-1.0" (WRONG)

**There IS a dual-license rule** (`mit_or_boost-1.0_1.RULE` with expression "mit OR boost-1.0") but it is not matching because:
1. **URL protocol mismatch (CONFIRMED):** The rule text contains `https://` URLs but the file contains `http://` URLs
   - File: `http://www.boost.org/LICENSE_1_0.txt` → tokens: `['http', 'www', 'boost', ...]`
   - Rule: `https://www.boost.org/LICENSE_1_0.txt` → tokens: `['https', 'www', 'boost', ...]`
   - The first token differs (`http` vs `https`), preventing hash-based matching
2. **Ignorable URLs not applied during matching:** The rule declares `ignorable_urls: [https://www.boost.org/LICENSE_1_0.txt]` but this field is not used to normalize/flexibilize matching - it's only used for required phrase handling
3. **Without the URL, the rest matches:** If we remove the URL portion, the remaining 186 tokens match exactly

### Impact

- **Estimated Tests Fixed:** ~20 golden test failures
- **Complexity:** Medium
- **Risk:** Medium (expression combination is core logic)

---

## Technical Analysis

### Python Implementation

**File:** `reference/scancode-toolkit/src/licensedcode/detection.py`

**Key Function:** `get_detected_license_expression()` (lines 1468-1602)

```python
def get_detected_license_expression(
    analysis,
    license_matches=None,
    license_match_mappings=None,
    post_scan=False,
):
    # ... filtering logic ...
    
    # CRITICAL: Uses match.rule.license_expression
    combined_expression = combine_expressions(
        expressions=[match.rule.license_expression for match in matches_for_expression],
        licensing=get_licensing(),
    )
    
    return detection_log, str(combined_expression)
```

**Key Insight:** Python passes `match.rule.license_expression` directly to `combine_expressions()`. The `license_expression` field on a Rule can contain OR expressions (e.g., `"mit OR boost-1.0"`).

### Rust Implementation

**File:** `src/license_detection/detection.rs`

**Key Function:** `determine_license_expression()` (lines 666-678)

```rust
pub fn determine_license_expression(matches: &[LicenseMatch]) -> Result<String, String> {
    if matches.is_empty() {
        return Err("No matches to determine expression".to_string());
    }

    let expressions: Vec<&str> = matches
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();

    // ALWAYS combines with AND
    combine_expressions(&expressions, CombineRelation::And, true)
        .map_err(|e| format!("Failed to combine expressions: {:?}", e))
}
```

**Verified Behavior (hash_match.rs:100, aho_match.rs:161, seq_match.rs:818):**
- `LicenseMatch.license_expression` IS set from `rule.license_expression.clone()`
- If a dual-license rule matches, the expression IS preserved correctly
- The `combine_expressions()` function correctly handles embedded OR expressions

### Data Flow Comparison

#### Python Data Flow (Working)

```
Rule mit_or_boost-1.0_1.RULE has license_expression = "mit OR boost-1.0"
    ↓
File text matches this rule pattern
    ↓
LicenseMatch.rule.license_expression = "mit OR boost-1.0"
    ↓
combine_expressions(["mit OR boost-1.0"], ...) = "mit OR boost-1.0"  ✓
```

#### Rust Data Flow (Current - Incorrect)

```
Rule mit_123.RULE has license_expression = "mit"
Rule boost-1.0_42.RULE has license_expression = "boost-1.0"
    ↓
File text matches BOTH rules (different patterns)
    ↓
LicenseMatch[0].license_expression = "mit"      (from mit rule)
LicenseMatch[1].license_expression = "boost-1.0" (from boost rule)
    ↓
combine_expressions(["mit", "boost-1.0"], And, ...) = "mit AND boost-1.0"  ✗
```

### The Critical Difference

**CORRECTED ANALYSIS:** The Rust matchers DO correctly preserve `rule.license_expression`. The issue is:

1. **Multiple single-license rules match** the file content, not the dual-license rule
2. The dual-license rule `mit_or_boost-1.0_1.RULE` exists but may not be matching
3. OR, the dual-license rule IS matching but is being filtered out or overridden

**Investigation showed:**
- `hash_match.rs:100`: `license_expression: rule.license_expression.clone()` ✓
- `aho_match.rs:161`: `license_expression: rule.license_expression.clone()` ✓  
- `seq_match.rs:818`: `license_expression: candidate.rule.license_expression.clone()` ✓
- `expression.rs:1469-1489`: Tests exist for embedded OR expressions ✓

---

## Investigation Required

**STATUS: PARTIALLY COMPLETED** - The following was verified:

### 1. How are LicenseMatch.license_expression values populated? ✓ VERIFIED

**Finding:** `LicenseMatch.license_expression` IS correctly set from `rule.license_expression.clone()`.

**Evidence from code:**
- `hash_match.rs:100`: `license_expression: rule.license_expression.clone()`
- `aho_match.rs:161`: `license_expression: rule.license_expression.clone()`
- `seq_match.rs:818`: `license_expression: candidate.rule.license_expression.clone()`

**Conclusion:** If a dual-license rule matches, its expression IS preserved correctly.

### 2. How does the matcher handle rules with OR expressions? ✓ VERIFIED

**Finding:** `combine_expressions()` correctly handles expressions containing OR.

**Evidence from tests (expression.rs:1469-1489):**
- `test_combine_expressions_with_existing_and()` passes
- `test_combine_expressions_with_existing_or()` passes
- Single expression with OR is preserved: `combine_expressions(&["mit OR boost-1.0"], And, true)` returns "mit OR boost-1.0"

### 3. THE REAL ISSUE: Why don't dual-license rules match? ✓ INVESTIGATION COMPLETE

**Key question:** Why does file `BSL-1.0_or_MIT.txt` match individual `mit` and `boost-1.0` rules instead of the `mit_or_boost-1.0_1.RULE`?

**ROOT CAUSE IDENTIFIED: URL protocol mismatch**

| Aspect | File | Rule |
|--------|------|------|
| URL | `http://www.boost.org/LICENSE_1_0.txt` | `https://www.boost.org/LICENSE_1_0.txt` |
| First token | `http` | `https` |
| Token match | ❌ Different | |

**Evidence:**
```python
# Python tokenization test
file_url_tokens = ['http', 'www', 'boost', 'org', 'license', '1', '0', 'txt']
rule_url_tokens = ['https', 'www', 'boost', 'org', 'license', '1', '0', 'txt']
# First token differs → no exact match possible
```

**Without the URL, everything else matches:**
- File tokens (without URL): 186 tokens
- Rule tokens (without URL): 186 tokens  
- Substring match: ✓ YES

**The rule has `ignorable_urls` but this is NOT used for matching flexibility:**
- The `ignorable_urls` field is defined in the rule YAML
- It's used for required phrase handling in Python
- But NOT used to allow URL variations during matching
- This appears to be a missing feature or a gap between rule definition and matching implementation

### Investigation Steps

**STATUS: COMPLETE** - Root cause identified as URL protocol mismatch.

1. ✓ **Verified tokenization** - Python and Rust tokenize identically
2. ✓ **Compared rule text with file text** - Found URL protocol mismatch (`http` vs `https`)
3. ✓ **Checked match coverage** - Without URL, 186/186 tokens match (100%)
4. ✓ **Verified `ignorable_urls` not used for matching** - Field exists but not applied during matching

### Implementation Strategy

**Root Cause:** The dual-license rule `mit_or_boost-1.0_1.RULE` has `https://` in its URL but the test file uses `http://`. The tokens differ (`https` vs `http`), preventing exact matching.

**Key Insight:** The `ignorable_urls` field exists in the rule but is NOT used to allow URL variations during matching. This is the gap to fix.

### Option A: Apply `ignorable_urls` During Tokenization/Matching (RECOMMENDED)

**Approach:** When a rule has `ignorable_urls`, normalize those URLs to a canonical form during tokenization, or allow flexible matching on those token positions.

**Implementation:**
1. During rule indexing, store `ignorable_urls` positions in the token sequence
2. During matching, treat ignorable URL token positions as "wildcards" that match any URL
3. OR: Normalize URLs during tokenization to strip protocol (`http`/`https` → just domain/path)

**Pros:**
- Fixes root cause using existing rule metadata
- Matches Python's intended behavior (rules define ignorable URLs for flexibility)
- Works for all rules with `ignorable_urls` (many exist)

**Cons:**
- Requires changes to tokenization or matching logic
- Need to handle URL normalization carefully

### Option B: Update Rule Text to Use `http://` (NOT RECOMMENDED)

**Approach:** Change the rule file to use `http://` instead of `https://`.

**Why NOT recommended:**
- Doesn't fix the underlying issue
- Other files might use `https://`
- Doesn't use the `ignorable_urls` metadata that exists for this purpose

### Option C: Sequence Matching with Lower Threshold (NOT RECOMMENDED)

**Approach:** Allow sequence matching to match with ~95% coverage instead of 100%.

**Why NOT recommended:**
- Could cause false positives
- Doesn't properly handle the URL variation
- The rule has `minimum_coverage: 90` but sequence matching still requires exact token sequence

---

## Specific Code Changes

### Change 1: Implement URL Normalization for `ignorable_urls`

**File:** `src/license_detection/tokenize.rs` or `src/license_detection/index/builder.rs`

**Approach:** When a rule has `ignorable_urls`, apply URL normalization during tokenization to make matching flexible.

**Option A - Normalize URLs in rule text during indexing:**
```rust
// In index/builder.rs, when building rules:
// For each ignorable_url in rule.ignorable_urls:
//   - Find URL tokens in rule text
//   - Replace with normalized form (or wildcard pattern)

fn normalize_ignorable_urls(text: &str, ignorable_urls: &[String]) -> String {
    // Strip protocol (http/https) from ignorable URLs for flexible matching
    // e.g., "https://www.boost.org/LICENSE_1_0.txt" → "www.boost.org/LICENSE_1_0.txt"
}
```

**Option B - Use URL pattern matching:**
```rust
// During matching, treat ignorable URL positions as patterns:
// - Match any URL at those token positions
// - This allows http:// or https:// to match
```

### Change 2: Add Test for URL Variation Matching

**File:** `src/license_detection/tokenize_test.rs` or `src/license_detection/index_test.rs`

```rust
#[test]
fn test_ignorable_url_normalization() {
    // Rule has https:// URL
    let rule_url = "https://www.boost.org/LICENSE_1_0.txt";
    // File has http:// URL  
    let file_url = "http://www.boost.org/LICENSE_1_0.txt";
    
    // After normalization, should match
    assert_eq!(
        normalize_url(rule_url),
        normalize_url(file_url)
    );
}
```

### Change 3: Verify the Fix with Golden Test

Run the golden test for `BSL-1.0_or_MIT.txt` to confirm the dual-license rule now matches:

```bash
cargo test --lib license_detection::golden_test -- --test-threads=1 2>&1 | grep -A2 "BSL-1.0_or_MIT"
```

---

## Test Cases from Golden Tests

### Primary Test Cases

These test files should pass after the fix:

| Test File | Expected Expression | Current Result |
|-----------|---------------------|----------------|
| `BSL-1.0_or_MIT.txt` | `mit OR boost-1.0` | `["mit", "boost-1.0"]` |
| `Ruby.t2` | `gpl-2.0 OR other-copyleft` | `["gpl-2.0", "other-copyleft"]` |
| `mit_or_commercial-option.txt` | `mit OR commercial-license` | Multiple separate detections |

### Test Validation

Run golden tests to verify fix:
```bash
cargo test --release -q --lib license_detection::golden_test
```

Count failures before and after:
```bash
cargo test --release -q --lib license_detection::golden_test 2>&1 | grep -c "mismatch"
```

---

## Testing Strategy

### Unit Tests

1. **Expression parsing tests** - Add tests for OR expressions in `expression.rs`
2. **Detection tests** - Add tests for dual-license scenarios in `detection.rs`
3. **Match creation tests** - Verify matches preserve rule expressions

### Integration Tests

1. **Golden tests** - Run full golden test suite
2. **Specific test files** - Create targeted tests for dual-license files

### Regression Tests

1. Ensure existing AND-combined expressions still work
2. Verify no regressions in single-license detection

---

## Implementation Steps

### Step 1: Implement URL Normalization for ignorable_urls (2-3 hours)

**Goal:** Allow rules with `ignorable_urls` to match files with URL variations (http vs https).

**Implementation Options:**

**Option A - Normalize during indexing (simpler):**
1. In `src/license_detection/index/builder.rs`, when processing rules with `ignorable_urls`:
   - Find the URL tokens in the rule text
   - Strip the protocol (http/https) to create a normalized form
   - Store the normalized tokens for matching

**Option B - Flexible token matching (more complex):**
1. Mark token positions that contain ignorable URLs
2. During matching, allow any URL token at those positions
3. This is more flexible but requires changes to all matchers

**Recommended:** Start with Option A for simplicity.

### Step 2: Add Unit Tests (1 hour)

1. Test URL normalization function
2. Test that rule with `ignorable_urls` matches file with different URL protocol
3. Test that dual-license expression is correctly preserved

### Step 3: Integration Testing (1-2 hours)

1. Run golden tests for `BSL-1.0_or_MIT.txt`
2. Verify output is `["mit OR boost-1.0"]` instead of `["mit", "boost-1.0"]`
3. Run full golden test suite to check for regressions

### Step 4: Documentation (30 minutes)

1. Document the URL normalization behavior
2. Add code comments explaining `ignorable_urls` handling

### Step 5: Documentation (30 minutes)

1. Document the root cause
2. Update code comments if needed

---

## Risk Assessment

### Medium Risk Areas

1. **Expression parsing** - Changes to expression handling could affect all detections
2. **Match creation** - Changes to match creation affect the entire pipeline

### Mitigation

1. **Comprehensive testing** - Run full test suite after changes
2. **Incremental changes** - Make small, verifiable changes
3. **Preserve existing behavior** - Ensure AND-combined expressions still work

### Rollback Plan

If issues are found:
1. Revert to current implementation
2. Investigate specific failure cases
3. Apply targeted fixes

---

## Dependencies

### Prerequisites

- Phase 1 (Duplicate Detection Merging) should be complete
- Phase 5 (Wrong License Selection) affects candidate selection

### Parallel Work

This phase can be worked in parallel with:
- Phase 2 (Source Map Processing)
- Phase 4 (Missing Detection)
- Phase 7 (SPDX Expression Parsing)

---

## Success Criteria

1. **Golden tests:** BSL-1.0_or_MIT.txt produces `["mit OR boost-1.0"]`
2. **Golden tests:** Ruby.t2 produces `["gpl-2.0 OR other-copyleft"]`
3. **No regressions:** Existing AND-combined expressions still work
4. **All tests pass:** `cargo test --lib` passes

---

## Appendix: Code Locations

### Rust Files

| File | Purpose | Status |
|------|---------|--------|
| `src/license_detection/detection.rs:666-678` | `determine_license_expression()` | ✓ Works correctly |
| `src/license_detection/detection.rs:840-892` | `create_detection_from_group()` | Uses determine_license_expression |
| `src/license_detection/expression.rs:628-666` | `combine_expressions()` | ✓ Works correctly, handles OR |
| `src/license_detection/models.rs:207-348` | `LicenseMatch` struct | Has license_expression field |
| `src/license_detection/models.rs:64-189` | `Rule` struct | Has license_expression field |
| `src/license_detection/hash_match.rs:99-132` | Creates LicenseMatch | ✓ Uses `rule.license_expression.clone()` |
| `src/license_detection/aho_match.rs:160-193` | Creates LicenseMatch | ✓ Uses `rule.license_expression.clone()` |
| `src/license_detection/seq_match.rs:817-850` | Creates LicenseMatch | ✓ Uses `candidate.rule.license_expression.clone()` |
| `src/license_detection/match_refine.rs:141-189` | `combine_matches()` | Combines matches for same rule |
| `src/license_detection/match_refine.rs:196-320` | `merge_overlapping_matches()` | Merges matches by rule_identifier |

### Python Reference Files

| File | Purpose |
|------|---------|
| `reference/scancode-toolkit/src/licensedcode/detection.py:1468-1602` | `get_detected_license_expression()` |
| `reference/scancode-toolkit/src/licensedcode/detection.py:1594-1597` | Key line: `match.rule.license_expression` |
| `reference/scancode-toolkit/src/licensedcode/match.py:152-244` | `LicenseMatch` class with `rule` field |
| `reference/scancode-toolkit/src/licensedcode/models.py:2262+` | `Rule` class |

### Key Rules

| Rule File | license_expression | Purpose |
|-----------|-------------------|---------|
| `reference/scancode-toolkit/src/licensedcode/data/rules/mit_or_boost-1.0_1.RULE` | `mit OR boost-1.0` | Dual-license MIT OR Boost |
| `reference/scancode-toolkit/src/licensedcode/data/rules/boost-1.0_*.RULE` | `boost-1.0` | Individual Boost rules |
| `reference/scancode-toolkit/src/licensedcode/data/rules/mit_*.RULE` | `mit` | Individual MIT rules |

### Test Data

| File | Expected |
|------|----------|
| `testdata/license-golden/datadriven/external/fossology-tests/Dual-license/BSL-1.0_or_MIT.txt` | `mit OR boost-1.0` |
| `testdata/license-golden/datadriven/external/fossology-tests/Dual-license/BSL-1.0_or_MIT.txt.yml` | `license_expressions: [mit OR boost-1.0]` |
