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
- When tokenized: `https` ≠ `http` → no exact match possible in aho automaton
- The rule declares `ignorable_urls` but this is NOT used during matching

### Solution

**Implement URL normalization for `ignorable_urls`:**
1. During rule indexing, normalize ignorable URLs (strip http/https protocol)
2. This allows flexible matching on URL variations
3. The dual-license rule will then match via aho, preserving its `mit OR boost-1.0` expression

---

## Python Analysis

### Python Matcher Flow (index.py:1009-1065)

```python
matchers = [
    Matcher(function=get_spdx_id_matches, include_low=True, name='spdx_lid', continue_matching=True),
    Matcher(function=self.get_exact_matches, include_low=False, name='aho', continue_matching=False),
]

if approximate:
    matchers += [Matcher(function=approx, include_low=False, name='seq', continue_matching=False), ]

already_matched_qspans = []
for matcher in matchers:
    matched = matcher.function(qry, matched_qspans=already_matched_qspans, ...)
    matched = match.merge_matches(matched)
    matches.extend(matched)
    
    # KEY: Only add 100% coverage matches to subtracted spans
    already_matched_qspans.extend(
        mtch.qspan for mtch in matched if mtch.coverage() == 100)
    
    # KEY: Stop matching if no matchable regions remain
    if not matcher.continue_matching:
        if not whole_query_run.is_matchable(
            include_low=matcher.include_low,
            qspans=already_matched_qspans,
        ):
            break
```

### Python Overlap Filtering (match.py:1187-1500)

The key insight is Python's `licensing_contains()` check during overlap filtering:

```python
# When medium_next overlap (40-70% overlap with next match):
if medium_next:
    if (current_match.licensing_contains(next_match)
        and current_match.len() >= next_match.len()
        and current_match.hilen() >= next_match.hilen()
    ):
        # next is discarded - current's licensing CONTAINS next's licensing
        discarded_append(matches_pop(j))
        continue
```

**Critical Behavior:**
- `licensing_contains("mit OR boost-1.0", "mit")` → **TRUE** (OR expression contains individual licenses)
- `licensing_contains("mit", "mit OR boost-1.0")` → **FALSE** (individual doesn't contain OR)

This means if the dual-license rule `mit OR boost-1.0` matches, it will WIN during overlap filtering against individual `mit` or `boost-1.0` matches.

### Python's `licensing_contains()` Implementation (models.py:2065-2075)

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

This uses the `license-expression` library's `contains()` method which correctly handles OR expressions.

### Why Python Works

1. **Aho matching finds the dual-license rule** (if URL matches or is close enough)
2. **OR expression is preserved** in `LicenseMatch.rule.license_expression`
3. **Overlap filtering preserves the OR match** because `licensing_contains(OR, individual)` is TRUE

---

## Rust Analysis

### Current Rust Flow (mod.rs:230-420)

```rust
// Phase 1c: Aho-Corasick matching
let aho_matches = aho_match(&self.index, &whole_run);
let refined_aho = match_refine::refine_aho_matches(&self.index, aho_matches, &query);

for m in &refined_aho {
    if m.match_coverage >= 99.99 && m.end_token > m.start_token {
        matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
    }
}
all_matches.extend(refined_aho);

// Later: Check if we should skip sequence matching
let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);
```

### Rust's `licensing_contains()` (expression.rs:444-505)

```rust
pub fn licensing_contains(container: &str, contained: &str) -> bool {
    // ... handles OR expressions correctly ...
    
    match (&simplified_container, &simplified_contained) {
        // OR contains individual license
        (LicenseExpression::Or { .. }, LicenseExpression::License(_)) => {
            let container_args = get_flat_args(&simplified_container);
            expr_in_args(&simplified_contained, &container_args)
        }
        // ...
    }
}
```

**Verified:** Rust's `licensing_contains()` correctly returns TRUE for:
- `licensing_contains("mit OR boost-1.0", "mit")` → TRUE
- `licensing_contains("mit OR boost-1.0", "boost-1.0")` → TRUE

### The Gap: URL Protocol Mismatch

**Test file (BSL-1.0_or_MIT.txt):**
```
// (See accompanying file LICENSE_1_0.txt or copy at
// http://www.boost.org/LICENSE_1_0.txt)
```

**Rule (mit_or_boost-1.0_1.RULE):**
```yaml
ignorable_urls:
    - https://www.boost.org/LICENSE_1_0.txt
---
...
  (See accompanying file LICENSE_1_0.txt or copy at
  https://www.boost.org/LICENSE_1_0.txt)
```

**Token mismatch:**
- File tokens: `['http', 'www', 'boost', 'org', 'license', '1', '0', 'txt']`
- Rule tokens: `['https', 'www', 'boost', 'org', 'license', '1', '0', 'txt']`
- First token differs → aho automaton cannot match

---

## The Fix

### Root Cause

The dual-license rule `mit_or_boost-1.0_1.RULE` cannot match via aho because:
1. The rule text contains `https://www.boost.org/LICENSE_1_0.txt`
2. The file text contains `http://www.boost.org/LICENSE_1_0.txt`
3. The token sequences differ at the first token (`https` vs `http`)
4. Aho-Corasick requires exact token sequence matches

Without the dual-license rule matching:
- Individual `mit` rule matches the MIT portion
- Individual `boost-1.0` rule matches the Boost portion
- These are combined with AND: "mit AND boost-1.0" (WRONG)

### Solution: URL Normalization for `ignorable_urls`

The rule already declares `ignorable_urls: [https://www.boost.org/LICENSE_1_0.txt]`. This metadata should be used during tokenization to normalize URLs.

**Implementation:**

1. **During rule indexing** (index/builder.rs), when processing rules with `ignorable_urls`:
   - Find each ignorable URL in the rule text
   - Replace with normalized form (strip `http://` or `https://` prefix)
   - OR: Use a URL-agnostic token pattern

2. **During file tokenization** (tokenize.rs):
   - Apply same URL normalization to file content
   - This allows `http://` and `https://` URLs to match

### Specific Code Changes

**File: `src/license_detection/index/builder.rs`**

Add function to normalize ignorable URLs:

```rust
/// Normalize URLs in text for flexible matching.
/// Strips http:// or https:// prefix from URLs matching ignorable_urls.
fn normalize_ignorable_urls(text: &str, ignorable_urls: &[String]) -> String {
    let mut result = text.to_string();
    for url in ignorable_urls {
        // Strip protocol from the ignorable URL pattern
        let normalized_pattern = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))
            .unwrap_or(url);
        
        // Replace both http and https variants in the text
        if let Some(http_pos) = result.find(&format!("http://{}", normalized_pattern)) {
            result.replace_range(http_pos..http_pos+7, "");
        }
        if let Some(https_pos) = result.find(&format!("https://{}", normalized_pattern)) {
            result.replace_range(https_pos..https_pos+8, "");
        }
    }
    result
}
```

**Apply during rule building:**

```rust
// In build_rule() or similar
let normalized_text = if let Some(ref ignorable_urls) = rule.ignorable_urls {
    normalize_ignorable_urls(&rule.text, ignorable_urls)
} else {
    rule.text.clone()
};
// Use normalized_text for tokenization
```

**Alternative: Token-level normalization**

Instead of modifying text, normalize at the token level:

```rust
// During tokenization, when a URL token is encountered:
// Check if it matches an ignorable URL pattern
// If so, emit a normalized token (without protocol)
```

---

## Test Cases

### Primary Test Case

**File:** `testdata/license-golden/datadriven/external/fossology-tests/Dual-license/BSL-1.0_or_MIT.txt`

**Expected:** `license_expressions: [mit OR boost-1.0]`

**Current:** `license_expressions: [mit, boost-1.0]` (incorrectly combined as AND)

### Other Dual-License Test Cases

| Test File | Expected Expression |
|-----------|---------------------|
| `BSL-1.0_or_MIT.txt` | `mit OR boost-1.0` |
| `Ruby.t2` | `gpl-2.0 OR other-copyleft` |
| `mit_or_commercial-option.txt` | `mit OR commercial-license` |

---

## Verification

### Unit Test

Add test for URL normalization:

```rust
#[test]
fn test_ignorable_url_normalization() {
    let ignorable_urls = vec!["https://www.boost.org/LICENSE_1_0.txt".to_string()];
    
    let text_http = "See http://www.boost.org/LICENSE_1_0.txt";
    let text_https = "See https://www.boost.org/LICENSE_1_0.txt";
    
    let norm_http = normalize_ignorable_urls(text_http, &ignorable_urls);
    let norm_https = normalize_ignorable_urls(text_https, &ignorable_urls);
    
    assert_eq!(norm_http, norm_https);
}
```

### Integration Test

Run golden tests after fix:

```bash
cargo test --release -q --lib license_detection::golden_test
```

---

## Implementation Steps

### Step 1: Implement URL Normalization (2-3 hours)

1. Add `normalize_ignorable_urls()` function
2. Apply during rule indexing
3. Apply during file tokenization (or use consistent normalization)

### Step 2: Unit Tests (1 hour)

1. Test URL normalization function
2. Test that normalized tokens match

### Step 3: Integration Testing (1-2 hours)

1. Run golden tests for `BSL-1.0_or_MIT.txt`
2. Verify `["mit OR boost-1.0"]` output
3. Check for regressions

### Step 4: Documentation (30 minutes)

1. Document URL normalization behavior
2. Add code comments

---

## Success Criteria

1. `BSL-1.0_or_MIT.txt` produces `["mit OR boost-1.0"]`
2. No regressions in other golden tests
3. `cargo test --lib` passes

---

## Appendix: Code Locations

### Key Rust Files

| File | Purpose |
|------|---------|
| `src/license_detection/index/builder.rs` | Rule indexing - add URL normalization |
| `src/license_detection/tokenize.rs` | Tokenization - apply URL normalization |
| `src/license_detection/models.rs:57` | `Rule.ignorable_urls` field |
| `src/license_detection/expression.rs:444` | `licensing_contains()` - correctly handles OR |
| `src/license_detection/match_refine.rs:607` | `filter_overlapping_matches()` - uses licensing_contains |

### Key Python Reference

| File | Purpose |
|------|---------|
| `reference/scancode-toolkit/src/licensedcode/index.py:1009-1065` | Matcher flow |
| `reference/scancode-toolkit/src/licensedcode/match.py:1187-1500` | Overlap filtering with licensing_contains |
| `reference/scancode-toolkit/src/licensedcode/models.py:2065` | `licensing_contains()` method |

### Key Rule

| File | Expression |
|------|------------|
| `reference/scancode-toolkit/src/licensedcode/data/rules/mit_or_boost-1.0_1.RULE` | `mit OR boost-1.0` |
