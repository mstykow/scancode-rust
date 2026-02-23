# PLAN-034: Add Copyright Word Check to `is_false_positive()`

**Status:** Not Started  
**Priority:** P2 (Medium - Feature Parity)  
**Estimated Effort:** Low (1-2 hours)  
**Affected Tests:** Potentially affects false positive classification across license detection  
**Last Updated:** 2026-02-23  

---

## 1. Problem Statement

The Rust `is_false_positive()` function in `src/license_detection/detection.rs:310-372` is missing a critical check that exists in the Python reference implementation.

**Missing Check**: Python's `is_false_positive()` checks if all matches contain copyright words (`["copyright", "(c)"]`) in their matched text. If all matches contain at least one copyright word, the function returns `False` (indicating the match is NOT a false positive - it's a legitimate license notice).

This check serves as a strong indicator that the matched text is genuinely about licensing/copyright, helping to preserve legitimate matches that might otherwise be filtered.

### Impact

Without this check:

- Legitimate license notices containing "copyright" or "(c)" may be incorrectly classified as false positives
- Reduced detection accuracy for files with copyright notices
- Feature parity gap with Python reference implementation

---

## 2. Current State Analysis

### 2.1 Rust Implementation

**Location:** `src/license_detection/detection.rs:310-372`

```rust
fn is_false_positive(matches: &[LicenseMatch]) -> bool {
    if matches.is_empty() {
        return false;
    }

    // Early return if all matches have full relevance (100)
    let has_full_relevance = matches.iter().all(|m| m.rule_relevance == 100);
    if has_full_relevance {
        return false;
    }

    // MISSING: Copyright word check should be HERE

    let start_line = matches.iter().map(|m| m.start_line).min().unwrap_or(0);
    // ... rest of the function
}
```

### 2.2 Available Data in LicenseMatch

**Location:** `src/license_detection/models.rs:260`

```rust
pub struct LicenseMatch {
    // ...
    /// Matched text snippet (optional for privacy/performance)
    pub matched_text: Option<String>,
    // ...
}
```

Key observations:

- `matched_text` is `Option<String>` - may be `None` if not populated
- When populated, contains the text from the document that matched the rule
- Populated during match creation in `hash_match.rs`, `aho_match.rs`, `seq_match.rs`, `unknown_match.rs`, and `spdx_lid.rs`

### 2.3 Current Test Coverage

**Location:** `src/license_detection/detection.rs:1767-1911`

Existing tests cover:

- `test_is_false_positive_bare_single`
- `test_is_false_positive_gpl_short`
- `test_is_false_positive_late_short_low_relevance`
- `test_is_false_positive_perfect_match`
- `test_is_false_positive_empty`
- `test_is_false_positive_single_license_reference_short`
- `test_is_false_positive_single_license_reference_long_rule`
- `test_is_false_positive_single_license_reference_full_relevance`

**No tests exist for the copyright word check** because the feature is not implemented.

---

## 3. Python Reference Analysis

### 3.1 Location

`reference/scancode-toolkit/src/licensedcode/detection.py:1162-1239`

### 3.2 Python Implementation

```python
def is_false_positive(license_matches, package_license=False):
    """
    Return True if all of the matches in ``license_matches`` List of LicenseMatch
    are false positives.
    """
    if package_license:
        return False

    # FIXME: actually run copyright detection here?
    copyright_words = ["copyright", "(c)"]
    has_copyrights = all(
        any(
            word in license_match.matched_text().lower()
            for word in copyright_words
        )
        for license_match in license_matches 
    )
    has_full_relevance = all(
        license_match.rule.relevance == 100
        for license_match in license_matches
    )
    if has_copyrights or has_full_relevance:
        return False
    
    # ... rest of the function
```

### 3.3 Key Logic Details

| Aspect | Python Behavior |
|--------|-----------------|
| Copyright words | `["copyright", "(c)"]` |
| Check order | Copyright check BEFORE full_relevance check |
| Outer quantifier | `all()` - ALL matches must have copyright |
| Inner quantifier | `any()` - match needs AT LEAST ONE copyright word |
| Case sensitivity | Case-insensitive (`.lower()`) |
| Return value | `False` (not a false positive) if `has_copyrights` is `True` |
| Comment | "# FIXME: actually run copyright detection here?" |

### 3.4 Python's `matched_text()` Method

**Location:** `reference/scancode-toolkit/src/licensedcode/match.py:757-795`

Python's `matched_text()` is a method that:

1. Returns the matched text from the query
2. Returns empty string if no query exists
3. Supports highlighting and whole-lines options

In Rust, this is pre-computed and stored as `Option<String>` on the `LicenseMatch` struct.

---

## 4. Proposed Changes

### 4.1 Add Copyright Word Check to `is_false_positive()`

**File:** `src/license_detection/detection.rs`  
**Location:** Lines 315-320 (after `has_full_relevance` check)

```rust
fn is_false_positive(matches: &[LicenseMatch]) -> bool {
    if matches.is_empty() {
        return false;
    }

    // Check for copyright words - if ALL matches contain "copyright" or "(c)",
    // this is likely a legitimate license notice, not a false positive.
    // Based on Python: detection.py:1173-1186
    let copyright_words = ["copyright", "(c)"];
    let has_copyrights = matches.iter().all(|m| {
        match &m.matched_text {
            Some(text) => {
                let text_lower = text.to_lowercase();
                copyright_words.iter().any(|word| text_lower.contains(word))
            }
            None => false, // No matched_text available, can't confirm copyright
        }
    });

    // Early return if all matches have full relevance (100)
    let has_full_relevance = matches.iter().all(|m| m.rule_relevance == 100);
    
    // If all matches contain copyright words OR have full relevance, not a false positive
    if has_copyrights || has_full_relevance {
        return false;
    }

    // ... rest of existing function unchanged
}
```

### 4.2 Alternative: Early Return Optimization

Since both `has_copyrights` and `has_full_relevance` cause early return with `false`, we can combine them efficiently:

```rust
fn is_false_positive(matches: &[LicenseMatch]) -> bool {
    if matches.is_empty() {
        return false;
    }

    let copyright_words = ["copyright", "(c)"];
    
    // Check each match for copyright words and full relevance
    for m in matches {
        // Check for full relevance
        if m.rule_relevance == 100 {
            return false;
        }
        
        // Check for copyright words
        if let Some(text) = &m.matched_text {
            let text_lower = text.to_lowercase();
            if copyright_words.iter().any(|word| text_lower.contains(word)) {
                return false;
            }
        }
    }
    
    // Continue with remaining checks only if ALL matches lack both conditions
    // But wait - we need "all matches have copyright" not "any match has copyright"
    // So this optimization doesn't work as-is...
}
```

**Issue with alternative**: The Python logic is `all()` - ALL matches must have copyright words. The early-return optimization above would incorrectly return `false` if ANY match has copyright words. We must stick with the `all()` approach.

### 4.3 Corrected Implementation with Proper Semantics

```rust
fn is_false_positive(matches: &[LicenseMatch]) -> bool {
    if matches.is_empty() {
        return false;
    }

    // Early return if all matches have full relevance (100)
    let has_full_relevance = matches.iter().all(|m| m.rule_relevance == 100);

    // Check for copyright words - if ALL matches contain "copyright" or "(c)",
    // this is likely a legitimate license notice, not a false positive.
    // Based on Python: detection.py:1173-1186
    let copyright_words = ["copyright", "(c)"];
    let has_copyrights = matches.iter().all(|m| {
        m.matched_text
            .as_ref()
            .map(|text| {
                let text_lower = text.to_lowercase();
                copyright_words.iter().any(|word| text_lower.contains(word))
            })
            .unwrap_or(false)
    });

    if has_copyrights || has_full_relevance {
        return false;
    }

    // ... rest of existing function unchanged
}
```

### 4.4 Helper Function (Optional)

For clarity and testability, we could extract a helper function:

```rust
/// Check if text contains copyright words ("copyright" or "(c)").
/// Case-insensitive matching.
fn contains_copyright_word(text: &str) -> bool {
    let text_lower = text.to_lowercase();
    COPYRIGHT_WORDS.iter().any(|word| text_lower.contains(word))
}

/// Copyright words used for false positive detection.
const COPYRIGHT_WORDS: &[&str] = &["copyright", "(c)"];
```

---

## 5. Edge Cases and Considerations

### 5.1 `matched_text` is `None`

When `matched_text` is `None`, the match cannot contribute to `has_copyrights`. The current proposed implementation returns `false` for the `has_copyrights` check in this case.

**Question**: Should `matched_text: None` be treated as "might have copyright" or "definitely no copyright"?

**Analysis**:

- Python's `matched_text()` returns empty string if no query exists (never `None`)
- Rust's `Option<String>` allows `None` for privacy/performance reasons
- If `matched_text` is `None`, we cannot determine copyright presence
- Conservative approach: treat `None` as "no copyright" (current proposal)

**Alternative**: Could treat `None` as "skip this check entirely", but this changes semantics and is more complex.

### 5.2 Performance Considerations

The `.to_lowercase()` call creates a new String for each match. This is acceptable because:

1. `is_false_positive()` is called once per detection group (not per file)
2. `matched_text` is typically small (license snippet, not entire file)
3. Alternative approaches (ASCII-only lowercasing, pre-computed lowercase) add complexity for minimal gain

### 5.3 Unicode Considerations

The `(c)` pattern uses ASCII characters. The `to_lowercase()` handles Unicode properly, so "COPYRIGHT" and "Copyright" will both match.

### 5.4 Over-matching Risk

The `contains()` check could match unintended text like:

- "acme-corp" (contains "(c)" if text has parens around c)
- Actually, `(c)` requires literal parentheses, so "acme-corp" won't match
- "copyright" could appear in non-license contexts, but the match must already be a license match for this check to apply

This is acceptable because:

- The check only applies to matches that already passed license detection
- "copyright" in matched text of a license rule is almost always relevant

---

## 6. Test Requirements

Per `docs/TESTING_STRATEGY.md`, this change requires:

### 6.1 Unit Tests (Layer 1)

Add tests in `src/license_detection/detection.rs` within the `#[cfg(test)] mod tests` block:

```rust
#[test]
fn test_is_false_positive_with_copyright_word() {
    // Match containing "copyright" should NOT be false positive
    let mut m = create_test_match_with_params(
        "gpl-2.0",
        "2-aho",
        1,
        10,
        50.0,
        100,
        10,
        50.0,
        50,
        "gpl-2.0.LICENSE",
    );
    m.matched_text = Some("This is copyrighted material under GPL".to_string());
    m.rule_length = 1; // Would normally be filtered as GPL short rule
    
    let matches = vec![m];
    assert!(
        !is_false_positive(&matches),
        "Match with 'copyright' should not be false positive"
    );
}

#[test]
fn test_is_false_positive_with_c_symbol() {
    // Match containing "(c)" should NOT be false positive
    let mut m = create_test_match_with_params(
        "mit",
        "2-aho",
        1500,
        1510,
        30.0,
        10,
        2,
        30.0,
        50,
        "mit.RULE",
    );
    m.matched_text = Some("Licensed under MIT (c) 2024".to_string());
    m.rule_length = 2; // Short rule, late match
    
    let matches = vec![m];
    assert!(
        !is_false_positive(&matches),
        "Match with '(c)' should not be false positive"
    );
}

#[test]
fn test_is_false_positive_without_copyright_word() {
    // Match without copyright words and low relevance SHOULD be false positive
    let mut m = create_test_match_with_params(
        "gpl-2.0",
        "2-aho",
        1,
        10,
        50.0,
        5,
        1,
        50.0,
        50,
        "gpl-2.0.LICENSE",
    );
    m.matched_text = Some("GPL licensed software".to_string());
    m.rule_length = 1;
    
    let matches = vec![m];
    assert!(
        is_false_positive(&matches),
        "GPL with rule_length=1 should still be false positive without copyright word"
    );
}

#[test]
fn test_is_false_positive_partial_copyright() {
    // Only ALL matches need copyright word
    let mut m1 = create_test_match_with_params(
        "mit",
        "2-aho",
        1,
        5,
        50.0,
        10,
        1,
        50.0,
        50,
        "mit.RULE",
    );
    m1.matched_text = Some("Copyright MIT".to_string());
    m1.rule_length = 1;
    
    let mut m2 = create_test_match_with_params(
        "mit",
        "2-aho",
        6,
        10,
        50.0,
        10,
        1,
        50.0,
        50,
        "mit.RULE",
    );
    m2.matched_text = Some("MIT licensed".to_string()); // No copyright word
    m2.rule_length = 1;
    
    let matches = vec![m1, m2];
    assert!(
        is_false_positive(&matches),
        "Not ALL matches have copyright word, so should still be false positive"
    );
}

#[test]
fn test_is_false_positive_all_matches_with_copyright() {
    // ALL matches have copyright word - not false positive
    let mut m1 = create_test_match_with_params(
        "mit",
        "2-aho",
        1,
        5,
        50.0,
        10,
        1,
        50.0,
        50,
        "mit.RULE",
    );
    m1.matched_text = Some("Copyright MIT".to_string());
    m1.rule_length = 1;
    
    let mut m2 = create_test_match_with_params(
        "apache",
        "2-aho",
        6,
        10,
        50.0,
        10,
        1,
        50.0,
        50,
        "apache.RULE",
    );
    m2.matched_text = Some("(c) Apache".to_string());
    m2.rule_length = 1;
    
    let matches = vec![m1, m2];
    assert!(
        !is_false_positive(&matches),
        "ALL matches have copyright word, should NOT be false positive"
    );
}

#[test]
fn test_is_false_positive_matched_text_none() {
    // When matched_text is None, cannot confirm copyright
    let mut m = create_test_match_with_params(
        "gpl-2.0",
        "2-aho",
        1,
        10,
        50.0,
        5,
        1,
        50.0,
        50,
        "gpl-2.0.LICENSE",
    );
    m.matched_text = None; // Explicitly None
    m.rule_length = 1;
    
    let matches = vec![m];
    assert!(
        is_false_positive(&matches),
        "With matched_text=None, copyright check fails, GPL short rule should be filtered"
    );
}

#[test]
fn test_is_false_positive_copyright_case_insensitive() {
    // Case-insensitive matching for copyright word
    let mut m = create_test_match_with_params(
        "mit",
        "2-aho",
        1,
        10,
        50.0,
        10,
        1,
        50.0,
        50,
        "mit.RULE",
    );
    m.matched_text = Some("COPYRIGHT HOLDER NAME".to_string());
    m.rule_length = 1;
    
    let matches = vec![m];
    assert!(
        !is_false_positive(&matches),
        "COPYRIGHT (uppercase) should match case-insensitively"
    );
}
```

### 6.2 Golden Tests (Layer 2)

Run existing golden tests to verify no regression:

```bash
cargo test --license-detection-golden
```

Specifically check tests with "copyright" in name:

- `testdata/license-golden/datadriven/lic1/diaspora_copyright.txt`
- `testdata/license-golden/datadriven/lic4/crown-copyright-canada.txt`

### 6.3 Integration Tests (Layer 3)

No new integration tests required - existing tests should verify the change works in the full pipeline.

---

## 7. Implementation Checklist

### Phase 1: Implementation

- [ ] Add `COPYRIGHT_WORDS` constant at module level (optional, for clarity)
- [ ] Add copyright word check to `is_false_positive()` function
- [ ] Handle `matched_text: None` case appropriately

### Phase 2: Unit Tests

- [ ] Add `test_is_false_positive_with_copyright_word`
- [ ] Add `test_is_false_positive_with_c_symbol`
- [ ] Add `test_is_false_positive_without_copyright_word`
- [ ] Add `test_is_false_positive_partial_copyright`
- [ ] Add `test_is_false_positive_all_matches_with_copyright`
- [ ] Add `test_is_false_positive_matched_text_none`
- [ ] Add `test_is_false_positive_copyright_case_insensitive`

### Phase 3: Verification

- [ ] Run `cargo test` to verify all unit tests pass
- [ ] Run `cargo test --license-detection-golden` to verify no regressions
- [ ] Run `cargo clippy` to verify no warnings

### Phase 4: Documentation

- [ ] Update doc comment on `is_false_positive()` to mention copyright check
- [ ] No CHANGELOG entry required (internal implementation detail)

---

## 8. Risk Assessment

### Low Risk

- **Function scope**: Changes are isolated to one function
- **Backward compatible**: Adding a check can only prevent false positives, not create new ones
- **Clear Python reference**: Implementation follows proven Python logic

### Minimal Risk

- **Performance**: `.to_lowercase()` creates a String, but impact is negligible
- **`matched_text: None`**: Handled conservatively (treats as no copyright)

### Mitigation

- Comprehensive unit tests covering all edge cases
- Golden test verification ensures no regression in real-world scenarios
- Case-insensitive matching tested explicitly

---

## 9. Related Issues

- **PLAN-009**: Fixed other aspects of `is_false_positive()` but missed this check
- **PLAN-008**: `filter_false_positive_license_lists_matches` - separate false positive filtering
- Line 588 in PLAN-009 explicitly lists "Add copyright word check to `is_false_positive()`" as a missing implementation

---

## 10. References

- Python implementation: `reference/scancode-toolkit/src/licensedcode/detection.py:1172-1186`
- Rust implementation: `src/license_detection/detection.rs:310-372`
- LicenseMatch struct: `src/license_detection/models.rs:207-291`
- Matched text population: `src/license_detection/query.rs:806-824`
- Test data with copyright: `testdata/license-golden/datadriven/lic1/diaspora_copyright.txt`
- Testing strategy: `docs/TESTING_STRATEGY.md`

---

## 11. Summary

This plan adds the missing copyright word check to Rust's `is_false_positive()` function, achieving feature parity with Python. The implementation:

1. Checks if ALL matches contain "copyright" or "(c)" (case-insensitive)
2. Returns `false` (not a false positive) if the check passes
3. Handles `matched_text: None` conservatively
4. Adds comprehensive unit test coverage

The change is low-risk, isolated, and directly follows the Python reference implementation.
