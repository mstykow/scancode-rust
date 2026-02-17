# PLAN-009: Fix `is_false_positive()` Function

**Status:** Draft  
**Priority:** P3 (HIGH IMPACT)  
**Estimated Effort:** Medium  
**Affected Tests:** ~10 golden tests  

## 1. Problem Statement

The Rust `is_false_positive()` function in `src/license_detection/detection.rs` incorrectly uses `matched_length` (character count) instead of `rule.length` (token count) for threshold comparison. This causes:

1. **Legitimate short matches to be incorrectly filtered** - Rules with token count = 1 should be filtered, but Rust checks character count ≤ 3 instead
2. **Spurious matches not being filtered** - Some matches that should be filtered pass through because their character count > 3 even though their token count is 1
3. **Missing checks** - Rust is missing several Python checks including:
   - `has_full_relevance` early-return check
   - `matches_is_license_tag_flags` check
   - `any()` vs `all()` semantics for the late match threshold

### Failing Tests

From `FAILURES.md`:
- gpl-2.0_30.txt
- gpl_48.txt  
- gpl_12.txt
- gpl-2.0-plus_1.txt
- flex-readme.txt
- exif_not_lgpl3.txt
- do-not-skip-short-gpl-matches.txt
- gpl_or_mit_1.txt
- gpl-2.0_17.txt

---

## 2. Python Reference Analysis

### Location
`reference/scancode-toolkit/src/licensedcode/detection.py:1162-1239`

### Key Constants
```python
# detection.py:96
FALSE_POSITIVE_RULE_LENGTH_THRESHOLD = 3

# detection.py:93  
FALSE_POSITIVE_START_LINE_THRESHOLD = 1000
```

### Python Implementation
```python
def is_false_positive(license_matches, package_license=False):
    """
    Return True if all of the matches in ``license_matches`` List of LicenseMatch
    are false positives.
    """
    if package_license:
        return False

    # Copyright check
    copyright_words = ["copyright", "(c)"]
    has_copyrights = all(
        any(word in license_match.matched_text().lower() for word in copyright_words)
        for license_match in license_matches 
    )
    
    # Early return for full relevance (CRITICAL - missing in Rust)
    has_full_relevance = all(
        license_match.rule.relevance == 100
        for license_match in license_matches
    )
    if has_copyrights or has_full_relevance:
        return False

    has_low_relevance = all(
        license_match.rule.relevance < 60
        for license_match in license_matches
    )

    start_line_region = min(
        license_match.start_line for license_match in license_matches
    )
    
    # KEY: Uses rule.length (token count), NOT matched_length (character count)
    match_rule_length_values = [
        license_match.rule.length for license_match in license_matches
    ]

    all_match_rule_length_one = all(
        match_rule_length == 1
        for match_rule_length in match_rule_length_values
    )
    
    bare_rules = ['gpl_bare', 'freeware_bare', 'public-domain_bare']
    is_bare_rule = all(
        any(bare_rule in license_match.rule.identifier for bare_rule in bare_rules)
        for license_match in license_matches
    )

    is_gpl = all(
        'gpl' in license_match.rule.identifier
        for license_match in license_matches
    )

    # MISSING in Rust: is_license_tag check
    matches_is_license_tag_flags = all(
        license_match.rule.is_license_tag for license_match in license_matches
    )

    is_single_match = len(license_matches) == 1

    # Check 1: Single bare rule with low relevance
    if is_single_match and is_bare_rule and has_low_relevance:
        return True

    # Check 2: GPL with all rules having length == 1 (token count)
    if is_gpl and all_match_rule_length_one:
        return True

    # Check 3: Late match with low relevance and short rules
    # NOTE: Uses any() for rule length check, not all()
    if has_low_relevance and start_line_region > FALSE_POSITIVE_START_LINE_THRESHOLD and any(
        match_rule_length_value <= FALSE_POSITIVE_RULE_LENGTH_THRESHOLD
        for match_rule_length_value in match_rule_length_values
    ):
        return True

    # Check 4: MISSING in Rust - is_license_tag with short rules
    if matches_is_license_tag_flags and all_match_rule_length_one:
        return True

    return False
```

### Critical Differences

| Aspect | Python | Rust (Current) | Issue |
|--------|--------|----------------|-------|
| Length metric | `rule.length` (token count) | `matched_length` (character count) | Wrong metric |
| GPL check | `all_match_rule_length_one` (== 1) | `all_short` (≤ 3) | Wrong comparison |
| Late match check | `any()` for rule length | `all_short` (all ≤ 3) | Wrong quantifier |
| Full relevance early-return | Present | Missing | Missing check |
| is_license_tag check | Present | Missing | Missing check |

---

## 3. Rust Code Analysis

### Current Implementation
**Location:** `src/license_detection/detection.rs:307-346`

```rust
fn is_false_positive(matches: &[LicenseMatch]) -> bool {
    if matches.is_empty() {
        return false;
    }

    let start_line = matches.iter().map(|m| m.start_line).min().unwrap_or(0);

    let bare_rules = ["gpl_bare", "freeware_bare", "public-domain_bare"];
    let is_bare_rule = matches.iter().all(|m| {
        bare_rules
            .iter()
            .any(|bare| m.rule_identifier.to_lowercase().contains(bare))
    });

    let is_gpl = matches
        .iter()
        .all(|m| m.rule_identifier.to_lowercase().contains("gpl"));

    // WRONG: Uses matched_length (characters) instead of rule.length (tokens)
    let all_short = matches
        .iter()
        .all(|m| m.matched_length <= FALSE_POSITIVE_RULE_LENGTH_THRESHOLD);

    let all_low_relevance = matches.iter().all(|m| m.rule_relevance < 60);

    let is_single = matches.len() == 1;

    if is_single && is_bare_rule && all_low_relevance {
        return true;
    }

    // WRONG: Should check rule.length == 1, not matched_length <= 3
    if is_gpl && all_short {
        return true;
    }

    // WRONG: Should use any() for length check, not all()
    if all_low_relevance && start_line > FALSE_POSITIVE_START_LINE_THRESHOLD && all_short {
        return true;
    }

    false
}
```

### Data Model Issue

**Rule struct** (`src/license_detection/models.rs:58-171`):
- Has `length_unique: usize` (unique token count)
- Has `high_length: usize` (legalese token occurrences)
- **MISSING**: Plain `length: usize` field (total token count including duplicates)

**LicenseMatch struct** (`src/license_detection/models.rs:174-224`):
- Has `matched_length: usize` - documented as "Length of matched text in characters"
- **MISSING**: `rule_length: usize` field to store the rule's token count

### Python's Rule.length Field

From `reference/scancode-toolkit/src/licensedcode/models.py:1699-1704`:
```python
length = attr.ib(
    default=0,
    metadata=dict(
        help='Computed length of a rule text in number of tokens aka. words, '
             'ignoring unknown words and stopwords')
)
```

This is the **total token count** (with duplicates), computed during index building.

---

## 4. Proposed Changes

### 4.1 Add `length` Field to Rule Struct

**File:** `src/license_detection/models.rs`

Add a new field after `tokens`:

```rust
pub struct Rule {
    // ... existing fields ...
    
    /// Token IDs for the text (assigned during indexing)
    pub tokens: Vec<u16>,

    /// Total count of tokens in the rule (including duplicates)
    /// Corresponds to Python's rule.length
    pub length: usize,
    
    // ... rest of fields ...
}
```

### 4.2 Populate `length` During Index Building

**File:** `src/license_detection/index/builder.rs`

Around line 230, where `rule_length` is computed:

```rust
// Compute token length (this is already computed)
let rule_length = rule_token_ids.len();

// Add this line to store it on the rule
rule.length = rule_length;
```

### 4.3 Add `rule_length` Field to LicenseMatch Struct

**File:** `src/license_detection/models.rs`

```rust
pub struct LicenseMatch {
    // ... existing fields ...
    
    /// Length of matched text in characters
    pub matched_length: usize,

    /// Token count of the matched rule (from rule.length)
    pub rule_length: usize,
    
    // ... rest of fields ...
}
```

### 4.4 Populate `rule_length` in Match Creators

**Files to update:**
- `src/license_detection/hash_match.rs`
- `src/license_detection/aho_match.rs`
- `src/license_detection/seq_match.rs`
- `src/license_detection/test_utils.rs`

Each match creation site needs to set `rule_length: rule.tokens.len()` or `rule_length: rule.length`.

### 4.5 Rewrite `is_false_positive()` Function

**File:** `src/license_detection/detection.rs`

```rust
fn is_false_positive(matches: &[LicenseMatch]) -> bool {
    if matches.is_empty() {
        return false;
    }

    // Early return if any match has full relevance (100)
    let has_full_relevance = matches.iter().all(|m| m.rule_relevance == 100);
    if has_full_relevance {
        return false;
    }

    // Note: Python also checks for copyright words, but this requires
    // matched_text to be available. Consider adding if needed for parity.

    let start_line = matches.iter().map(|m| m.start_line).min().unwrap_or(0);

    let bare_rules = ["gpl_bare", "freeware_bare", "public-domain_bare"];
    let is_bare_rule = matches.iter().all(|m| {
        bare_rules
            .iter()
            .any(|bare| m.rule_identifier.to_lowercase().contains(bare))
    });

    let is_gpl = matches
        .iter()
        .all(|m| m.rule_identifier.to_lowercase().contains("gpl"));

    // FIXED: Use rule_length (token count) instead of matched_length (characters)
    let rule_length_values: Vec<usize> = matches.iter().map(|m| m.rule_length).collect();
    
    let all_rule_length_one = rule_length_values.iter().all(|&l| l == 1);

    let all_low_relevance = matches.iter().all(|m| m.rule_relevance < 60);

    let is_single = matches.len() == 1;

    // Check 1: Single bare rule with low relevance
    if is_single && is_bare_rule && all_low_relevance {
        return true;
    }

    // Check 2: GPL with all rules having length == 1 (FIXED: was all_short)
    if is_gpl && all_rule_length_one {
        return true;
    }

    // Check 3: Late match with low relevance
    // FIXED: Use any() instead of all(), and rule_length instead of matched_length
    if all_low_relevance 
        && start_line > FALSE_POSITIVE_START_LINE_THRESHOLD 
        && rule_length_values.iter().any(|&l| l <= FALSE_POSITIVE_RULE_LENGTH_THRESHOLD)
    {
        return true;
    }

    // Note: is_license_tag check requires adding is_license_tag field to LicenseMatch
    // This is a separate enhancement - see PLAN for that work

    false
}
```

### 4.6 Alternative: Access Rule from LicenseMatch (Simpler Approach)

If we can access the original Rule from a LicenseMatch (via the LicenseIndex), we could avoid adding `rule_length` to LicenseMatch:

```rust
fn is_false_positive_with_index(matches: &[LicenseMatch], index: &LicenseIndex) -> bool {
    // Get rule lengths from the index
    let rule_length_values: Vec<usize> = matches
        .iter()
        .filter_map(|m| {
            index
                .rules_by_rid
                .iter()
                .find(|r| r.identifier == m.rule_identifier)
                .map(|r| r.length)
        })
        .collect();
    // ... rest of logic using rule_length_values
}
```

However, this requires passing the index to the function, which changes the signature and may require broader refactoring.

---

## 5. Testing Strategy

### 5.1 Update Existing Unit Tests

**File:** `src/license_detection/detection.rs`

Update `create_test_match_with_params` to include `rule_length`:

```rust
fn create_test_match_with_params(
    license_expression: &str,
    matcher: &str,
    start_line: usize,
    end_line: usize,
    score: f32,
    matched_length: usize,
    match_coverage: f32,
    rule_relevance: u8,
    rule_identifier: &str,
    rule_length: usize,  // ADD THIS PARAMETER
) -> LicenseMatch {
    LicenseMatch {
        // ... existing fields ...
        matched_length,
        rule_length,  // ADD THIS FIELD
        // ...
    }
}
```

### 5.2 Add New Unit Tests

```rust
#[test]
fn test_is_false_positive_gpl_single_token_rule() {
    // GPL match with rule.length == 1 should be false positive
    let matches = vec![create_test_match_with_params(
        "gpl-2.0",
        "2-aho",
        1,
        5,
        50.0,
        10,        // matched_length (characters)
        50.0,
        80,
        "gpl-2.0_bare.RULE",
        1,         // rule_length (tokens) - KEY: == 1
    )];
    assert!(is_false_positive(&matches));
}

#[test]
fn test_is_false_positive_gpl_multi_token_rule() {
    // GPL match with rule.length > 1 should NOT be false positive
    let matches = vec![create_test_match_with_params(
        "gpl-2.0",
        "2-aho",
        1,
        10,
        80.0,
        100,       // matched_length (characters)
        80.0,
        90,
        "gpl-2.0.LICENSE",
        50,        // rule_length (tokens) - KEY: > 1
    )];
    assert!(!is_false_positive(&matches));
}

#[test]
fn test_is_false_positive_full_relevance_early_return() {
    // Match with relevance == 100 should NOT be false positive
    let matches = vec![create_test_match_with_params(
        "gpl-2.0",
        "2-aho",
        1,
        5,
        50.0,
        5,
        50.0,
        100,       // rule_relevance == 100
        "gpl-2.0_bare.RULE",
        1,         // Even with short rule
    )];
    assert!(!is_false_positive(&matches));
}

#[test]
fn test_is_false_positive_late_match_any_short_rule() {
    // Late match with ANY short rule (not all) should be false positive
    let matches = vec![
        create_test_match_with_params(
            "mit",
            "2-aho",
            1500,
            1510,
            30.0,
            100,
            30.0,
            50,
            "mit.LICENSE",
            50,     // Long rule - this one is fine
        ),
        create_test_match_with_params(
            "mit",
            "2-aho",
            1511,
            1515,
            30.0,
            10,
            30.0,
            50,
            "mit_short.RULE",
            2,      // Short rule - ANY short rule triggers false positive
        ),
    ];
    assert!(is_false_positive(&matches));
}
```

### 5.3 Golden Test Verification

Run the golden tests to verify the fix:

```bash
# Run specific failing tests
cargo test test_extract_from_testdata -- --test-threads=1 2>&1 | grep -E "(FAIL|PASS)"

# Run all license detection golden tests
cargo test --license-detection-golden
```

### 5.4 Python Comparison Test

Create a test that directly compares Python and Rust behavior:

```rust
#[test]
fn test_parity_with_python_is_false_positive() {
    // Test case derived from Python's test_detection_returns_correct_no_gpl3_false_positive
    // This tests the false-positive-gpl3.txt case
    
    // Load the test file
    let content = fs::read_to_string("testdata/false-positive/false-positive-gpl3.txt")
        .expect("Test file not found");
    
    // Run detection
    let index = get_license_index();
    let matches = detect_licenses(&content, &index);
    
    // Should return no matches (all false positives)
    assert!(matches.is_empty(), "Expected no matches for false-positive-gpl3.txt");
}
```

---

## 6. Implementation Checklist

### Phase 1: Data Model Changes
- [ ] Add `length: usize` field to `Rule` struct in `models.rs`
- [ ] Add `rule_length: usize` field to `LicenseMatch` struct in `models.rs`
- [ ] Update `create_test_match_with_params` helper function

### Phase 2: Populate Data
- [ ] Set `rule.length` in `index/builder.rs` during index building
- [ ] Set `rule_length` in `hash_match.rs` when creating matches
- [ ] Set `rule_length` in `aho_match.rs` when creating matches
- [ ] Set `rule_length` in `seq_match.rs` when creating matches
- [ ] Update `test_utils.rs` match creation helpers

### Phase 3: Fix the Function
- [ ] Add `has_full_relevance` early-return check
- [ ] Change from `matched_length` to `rule_length` for all checks
- [ ] Change GPL check from `all_short` (≤ 3) to `all_rule_length_one` (== 1)
- [ ] Change late match check from `all()` to `any()` for rule length
- [ ] (Optional) Add `is_license_tag` check if that field is propagated

### Phase 4: Tests
- [ ] Update existing unit tests with `rule_length` parameter
- [ ] Add new unit tests for edge cases
- [ ] Run golden tests to verify ~10 failing tests now pass
- [ ] Run full test suite to ensure no regressions

### Phase 5: Documentation
- [ ] Update doc comments on `Rule.length` and `LicenseMatch.rule_length`
- [ ] Document the change in CHANGELOG or commit message

---

## 7. Risk Assessment

### Low Risk
- Adding new fields to structs is backward compatible for internal use
- Unit test changes are isolated

### Medium Risk
- Changing the false positive logic could affect other tests not yet identified
- The `any()` vs `all()` change in late match detection needs careful testing

### Mitigation
- Run full golden test suite after changes
- Compare output against Python reference for a sample of real-world files
- Consider incremental rollout (fix one check at a time)

---

## 8. Related Issues

- Missing `is_license_tag` flag propagation from Rule to LicenseMatch (separate issue)
- Missing copyright word check in `is_false_positive()` (lower priority)
- `filter_false_positive_license_lists_matches` not implemented (separate PLAN)

---

## 9. References

- Python implementation: `reference/scancode-toolkit/src/licensedcode/detection.py:1162-1239`
- Python constants: `reference/scancode-toolkit/src/licensedcode/detection.py:93-96`
- Rust implementation: `src/license_detection/detection.rs:307-346`
- Rust constants: `src/license_detection/detection.rs:24-28`
- FAILURES.md section: Lines 270-276
