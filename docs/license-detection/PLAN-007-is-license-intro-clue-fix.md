# PLAN-007: Fix `is_license_intro_match()` and `is_license_clue_match()` Heuristics

## Status: Draft

## Problem Statement

### What is Wrong

The current Rust implementation of `is_license_intro_match()` and `is_license_clue_match()` in `src/license_detection/detection.rs` uses **incorrect heuristics** based on string matching instead of the proper rule flags loaded from the rule files.

**Current (Incorrect) Implementation** (lines 220-227 in `detection.rs`):

```rust
/// Check if a match is a license intro.
fn is_license_intro_match(match_item: &LicenseMatch) -> bool {
    match_item.matcher.starts_with("5-unknown") || match_item.rule_identifier.contains("intro")
}

/// Check if a match is a license clue.
fn is_license_clue_match(match_item: &LicenseMatch) -> bool {
    match_item.matcher == "5-unknown" || match_item.rule_identifier.contains("clue")
}
```

**Why This is Wrong:**

1. Uses string matching on `rule_identifier` (e.g., checking if "intro" is in the identifier string) instead of the actual `is_license_intro` and `is_license_clue` boolean flags from the Rule
2. Incorrectly uses `matcher.starts_with("5-unknown")` as a primary condition
3. Results in ~30 golden tests failing because matches are incorrectly grouped/separated during detection grouping

### Where This Affects the Code

These functions are called in `group_matches_by_region_with_threshold()` (lines 179-200) to determine how matches are grouped:

- **Line 179-181**: Previous match checked - if it's an "unknown" matcher AND `is_license_intro_match()`, the current match is added to the group
- **Line 182-186**: Current match checked - if `is_license_intro_match()`, starts a new group  
- **Line 187-192**: Current match checked - if `is_license_clue_match()`, creates a separate singleton group

The incorrect heuristics cause:

- Intro matches to not be properly identified (they're not grouped with subsequent proper matches)
- Clue matches to be incorrectly identified or missed
- Detection grouping to produce wrong results

---

## Python Reference Analysis

### Rule Definition (models.py)

The `Rule` class in Python defines these flags at `reference/scancode-toolkit/src/licensedcode/models.py`:

```python
# Lines 1410-1439
is_license_intro = attr.ib(
    default=False,
    repr=False,
    metadata=dict(
        help='True if this is rule text is a license introduction: '
        'An intro is a short introductory statement placed just before an '
        'actual license text, notice or reference that it introduces. For '
        'instance "Licensed under ..." would be an intro text typically '
        'followed by some license notice. An "intro" is a weak clue that '
        'there may be some license statement afterwards. It should be '
        'considered in the context of the detection that it precedes. '
        'Ideally it should be merged with the license detected immediately '
        'after. Mutually exclusive from any other is_license_* flag')
)

is_license_clue = attr.ib(
    default=False,
    repr=False,
    metadata=dict(
        help='True if this is rule text is a clue to a license '
        'but cannot be considered in a proper license detection '
        'as a license text/notice/reference/tag/intro as it is'
        'merely a clue and does not actually point to or refer to '
        'the actual license directly. This is still valuable information '
        'useful in determining the license/origin of a file, but this '
        'should not be summarized/present in the license expression for '
        'a package or a file, nor its list of license detections. '
        'considered in the context of the detection that it precedes. '
        'Mutually exclusive from any other is_license_* flag')
)
```

### Detection Functions (detection.py)

**`is_license_intro()` function** at `detection.py:1349-1365`:

```python
def is_license_intro(license_match):
    """
    Return True if `license_match` LicenseMatch object is matched completely to
    a unknown license intro present as a Rule.
    """
    from licensedcode.match_aho import MATCH_AHO_EXACT

    return (
        (
            license_match.rule.is_license_intro or license_match.rule.is_license_clue or
            license_match.rule.license_expression == 'free-unknown'
        )
        and (
            license_match.matcher == MATCH_AHO_EXACT
            or license_match.coverage() == 100
        )
    )
```

**Key observations:**

1. Checks `license_match.rule.is_license_intro` - the **rule flag**, not string matching
2. Also checks `license_match.rule.is_license_clue` - both flags are considered for intro detection
3. Also checks `license_match.rule.license_expression == 'free-unknown'` - special case
4. Requires either `matcher == "2-aho"` OR `coverage() == 100`

**`is_unknown_intro()` function** at `detection.py:1250-1262`:

```python
def is_unknown_intro(license_match):
    """
    Return True if the LicenseMatch is unknown and can be considered
    as a license intro to other license matches.
    I.e. this is not an unknown when followed by other proper matches.
    """
    return (
        license_match.rule.has_unknown and
        (
            license_match.rule.is_license_intro or license_match.rule.is_license_clue or
            license_match.rule.license_expression == 'free-unknown'
        )
    )
```

**Key observations:**

1. Uses `license_match.rule.is_license_intro` and `license_match.rule.is_license_clue` - the **rule flags**
2. Also checks `license_match.rule.has_unknown` for the rule

### Rule Loading

Rules are loaded from `.RULE` files with YAML frontmatter containing these flags. Example from `reference/scancode-toolkit/src/licensedcode/data/rules/`:

```yaml
---
license_expression: mit
is_license_intro: yes
relevance: 90
---
Licensed under the MIT license
```

The Python `Rule.from_file()` method parses the YAML frontmatter and sets these boolean attributes.

---

## Rust Code Analysis

### Current LicenseMatch Struct (models.rs:174-224)

The `LicenseMatch` struct **already has** the required fields:

```rust
pub struct LicenseMatch {
    // ... other fields ...
    
    /// True if this match is from a license intro rule
    pub is_license_intro: bool,

    /// True if this match is from a license clue rule
    pub is_license_clue: bool,
}
```

### Current Rule Struct (models.rs:58-171)

The `Rule` struct **already has** the required fields:

```rust
pub struct Rule {
    // ... other fields ...
    
    /// True if this is an introductory statement before actual license text
    pub is_license_intro: bool,

    /// True if this is a clue but not a proper license detection
    pub is_license_clue: bool,
    
    // ... other fields ...
}
```

### Rule Loading (rules/loader.rs)

The `RuleFrontmatter` struct (lines 150-221) **already parses** these flags:

```rust
#[derive(Debug, Deserialize)]
struct RuleFrontmatter {
    // ... other fields ...
    
    #[serde(default, deserialize_with = "deserialize_yes_no_bool")]
    is_license_intro: Option<bool>,

    #[serde(default, deserialize_with = "deserialize_yes_no_bool")]
    is_license_clue: Option<bool>,
    
    // ... other fields ...
}
```

And `parse_rule_file()` (lines 223-342) **already sets** these fields:

```rust
Ok(Rule {
    // ... other fields ...
    is_license_intro: fm.is_license_intro.unwrap_or(false),
    is_license_clue: fm.is_license_clue.unwrap_or(false),
    // ... other fields ...
})
```

### Match Creation (aho_match.rs, hash_match.rs, etc.)

**Aho-Corasick matcher** (`aho_match.rs:172`):

```rust
is_license_intro: rule.is_license_intro,
```

This correctly copies `rule.is_license_intro` to `LicenseMatch.is_license_intro`. ✓

**Hash matcher** (`hash_match.rs:116`):

```rust
is_license_intro: rule.is_license_intro,
```

Also correctly copies. ✓

**SPDX-LID matcher** (`spdx_lid.rs:288`):

```rust
is_license_intro: rule.is_license_intro,
```

Also correctly copies. ✓

### The Problem: Incorrect Functions in detection.rs

**`is_license_intro_match()` at line 220** - Uses string matching instead of the flag:

```rust
fn is_license_intro_match(match_item: &LicenseMatch) -> bool {
    match_item.matcher.starts_with("5-unknown") || match_item.rule_identifier.contains("intro")
    //                      ^^^^^^^^^^^^^^^^^^ WRONG           ^^^^^^^^^^^^^^^^^^^^^ WRONG
}
```

**`is_license_clue_match()` at line 225** - Also uses string matching:

```rust
fn is_license_clue_match(match_item: &LicenseMatch) -> bool {
    match_item.matcher == "5-unknown" || match_item.rule_identifier.contains("clue")
    //                  ^^^^^^^^^^^^^ WRONG           ^^^^^^^^^^^^^^^^^^^^^ WRONG
}
```

### The Correct `is_license_intro()` Function at line 426

Interestingly, there's **another function** `is_license_intro()` at line 426 that **is implemented correctly**:

```rust
fn is_license_intro(match_item: &LicenseMatch) -> bool {
    (match_item.is_license_intro
        || match_item.is_license_clue
        || match_item.license_expression == "free-unknown")
        && (match_item.matcher == "2-aho" || match_item.match_coverage >= 99.99)
}
```

This matches Python's logic! But it's used in `filter_license_intros()` not in `group_matches_by_region_with_threshold()`.

---

## Proposed Changes

### Summary

The fix is straightforward: Replace the incorrect `is_license_intro_match()` and `is_license_clue_match()` functions to use the already-populated boolean flags from `LicenseMatch`.

### Change 1: Fix `is_license_intro_match()` function

**File:** `src/license_detection/detection.rs`  
**Line:** 220-222

**Before:**

```rust
/// Check if a match is a license intro.
fn is_license_intro_match(match_item: &LicenseMatch) -> bool {
    match_item.matcher.starts_with("5-unknown") || match_item.rule_identifier.contains("intro")
}
```

**After:**

```rust
/// Check if a match is a license intro for grouping purposes.
///
/// A match is considered a license intro if its rule has the is_license_intro
/// or is_license_clue flag set, OR if the license_expression is 'free-unknown'.
///
/// Note: This is used for detection grouping. For filtering purposes (removing
/// intros from detections), use `is_license_intro()` which has additional
/// matcher and coverage requirements.
///
/// Based on Python: is_unknown_intro() at detection.py:1250-1262
fn is_license_intro_match(match_item: &LicenseMatch) -> bool {
    match_item.is_license_intro
        || match_item.is_license_clue
        || match_item.license_expression == "free-unknown"
}
```

### Change 2: Fix `is_license_clue_match()` function

**File:** `src/license_detection/detection.rs`  
**Line:** 225-227

**Before:**

```rust
/// Check if a match is a license clue.
fn is_license_clue_match(match_item: &LicenseMatch) -> bool {
    match_item.matcher == "5-unknown" || match_item.rule_identifier.contains("clue")
}
```

**After:**

```rust
/// Check if a match is a license clue for grouping purposes.
///
/// A match is considered a license clue if its rule has the is_license_clue
/// flag set. License clues are low-quality matches that should be reported
/// separately from proper license detections.
///
/// Based on Python: has_correct_license_clue_matches() at detection.py:1265-1272
fn is_license_clue_match(match_item: &LicenseMatch) -> bool {
    match_item.is_license_clue
}
```

### Change 3: Consider whether `is_license_intro_match()` should also check matcher

Looking at Python's `is_license_intro()` function (detection.py:1349-1365), there's a secondary condition:

```python
and (
    license_match.matcher == MATCH_AHO_EXACT
    or license_match.coverage() == 100
)
```

However, Python's `is_unknown_intro()` (detection.py:1250-1262) does NOT have this condition.

**Decision:** For grouping purposes (which is what `is_license_intro_match()` is used for), we should NOT require the matcher/coverage condition. The grouping logic needs to identify ALL intro-like matches, not just high-quality ones. The filtering function `is_license_intro()` already has the stricter logic.

### No Additional Changes Needed

1. **Rule struct** - Already has `is_license_intro` and `is_license_clue` fields ✓
2. **LicenseMatch struct** - Already has `is_license_intro` and `is_license_clue` fields ✓
3. **Rule loader** - Already parses and sets these fields from YAML ✓
4. **Match creators** - Already copy these flags from Rule to LicenseMatch ✓

---

## Testing Strategy

### How Python Tests This

Python tests the intro/clue functionality through:

1. **Unit tests** for `is_license_intro()` function in `test_detection.py`
2. **Integration tests** that verify detection grouping with intro rules
3. **Golden tests** comparing output against expected JSON files

### What Tests to Add/Modify in Rust

#### 1. Add Unit Tests for Fixed Functions

Add tests in `src/license_detection/detection.rs` tests module:

```rust
#[test]
fn test_is_license_intro_match_with_flag() {
    let m = LicenseMatch {
        is_license_intro: true,
        ..create_test_match_with_params("mit", "2-aho", 1, 10, 95.0, 100, 95.0, 100, "mit_1.RULE")
    };
    assert!(is_license_intro_match(&m));
}

#[test]
fn test_is_license_intro_match_with_clue_flag() {
    let m = LicenseMatch {
        is_license_clue: true,
        ..create_test_match_with_params("mit", "2-aho", 1, 10, 95.0, 100, 95.0, 100, "mit_1.RULE")
    };
    assert!(is_license_intro_match(&m));
}

#[test]
fn test_is_license_intro_match_free_unknown() {
    let m = LicenseMatch {
        license_expression: "free-unknown".to_string(),
        ..create_test_match_with_params("free-unknown", "2-aho", 1, 10, 95.0, 100, 95.0, 100, "free-unknown.RULE")
    };
    assert!(is_license_intro_match(&m));
}

#[test]
fn test_is_license_intro_match_false_without_flag() {
    let m = LicenseMatch {
        is_license_intro: false,
        is_license_clue: false,
        license_expression: "mit".to_string(),
        ..create_test_match_with_params("mit", "2-aho", 1, 10, 95.0, 100, 95.0, 100, "mit_1.RULE")
    };
    assert!(!is_license_intro_match(&m));
}

#[test]
fn test_is_license_clue_match_with_flag() {
    let m = LicenseMatch {
        is_license_clue: true,
        ..create_test_match_with_params("mit", "2-aho", 1, 10, 95.0, 100, 95.0, 100, "mit_1.RULE")
    };
    assert!(is_license_clue_match(&m));
}

#[test]
fn test_is_license_clue_match_false_without_flag() {
    let m = LicenseMatch {
        is_license_clue: false,
        ..create_test_match_with_params("mit", "2-aho", 1, 10, 95.0, 100, 95.0, 100, "mit_1.RULE")
    };
    assert!(!is_license_clue_match(&m));
}
```

#### 2. Add Integration Test for Detection Grouping

Test that intro matches are properly grouped with subsequent proper matches:

```rust
#[test]
fn test_group_matches_intro_followed_by_proper_match() {
    // Create an intro match
    let intro = LicenseMatch {
        is_license_intro: true,
        license_expression: "unknown".to_string(),
        matcher: "2-aho".to_string(),
        start_line: 1,
        end_line: 2,
        match_coverage: 100.0,
        ..create_test_match_with_params("unknown", "2-aho", 1, 2, 100.0, 5, 100.0, 100, "intro.RULE")
    };
    
    // Create a proper match that follows
    let proper = LicenseMatch {
        is_license_intro: false,
        license_expression: "mit".to_string(),
        matcher: "2-aho".to_string(),
        start_line: 3,
        end_line: 10,
        match_coverage: 100.0,
        ..create_test_match_with_params("mit", "2-aho", 3, 10, 100.0, 50, 100.0, 100, "mit.LICENSE")
    };
    
    let matches = vec![intro, proper];
    let groups = group_matches_by_region(&matches);
    
    // Intro should be grouped with the proper match
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].matches.len(), 2);
}
```

#### 3. Verify Golden Tests Pass

Run the golden test suite and verify that previously failing tests now pass:

```bash
cargo test --lib license_detection::golden_test
```

### How to Verify the Fix Works

1. **Run all license detection tests:**

   ```bash
   cargo test --lib license_detection::
   ```

2. **Run golden tests specifically:**

   ```bash
   cargo test --lib license_detection::golden_test
   ```

3. **Run on test data and compare with Python output:**

   ```bash
   # Run Rust version
   cargo run -- testdata/some-dir -o rust-output.json
   
   # Run Python version (from reference/scancode-toolkit)
   cd reference/scancode-toolkit
   ./scancode testdata/some-dir -o python-output.json
   
   # Compare outputs
   diff rust-output.json python-output.json
   ```

4. **Check specific intro/clue rules:**
   - Find rule files with `is_license_intro: yes` in `reference/scancode-toolkit/src/licensedcode/data/rules/`
   - Create test files that match these rules
   - Verify detection grouping is correct

---

## Implementation Checklist

- [ ] Fix `is_license_intro_match()` function to use `match_item.is_license_intro` flag
- [ ] Fix `is_license_clue_match()` function to use `match_item.is_license_clue` flag  
- [ ] Add unit tests for `is_license_intro_match()`
- [ ] Add unit tests for `is_license_clue_match()`
- [ ] Add integration test for intro + proper match grouping
- [ ] Run all existing tests to ensure no regressions
- [ ] Run golden tests and verify previously failing tests now pass
- [ ] Update documentation/comments if needed

---

## Risk Assessment

### Low Risk

This is a low-risk change because:

1. **The data is already present** - `LicenseMatch.is_license_intro` and `LicenseMatch.is_license_clue` are already populated from the Rule
2. **The logic is straightforward** - Simply use the boolean flags instead of string matching
3. **The existing `is_license_intro()` function** already demonstrates the correct pattern
4. **Tests exist** - Golden tests will catch any regressions

### Potential Edge Cases

1. **Rules without these flags** - Default to `false`, handled by `unwrap_or(false)` in loader
2. **Both flags set** - Should not happen (mutually exclusive per Python docs), but both would return `true` which is correct
3. **`free-unknown` expression** - Handled by explicit check in the new implementation

---

## References

- Python `is_license_intro()` implementation: `reference/scancode-toolkit/src/licensedcode/detection.py:1349-1365`
- Python `is_unknown_intro()` implementation: `reference/scancode-toolkit/src/licensedcode/detection.py:1250-1262`
- Python `has_correct_license_clue_matches()` implementation: `reference/scancode-toolkit/src/licensedcode/detection.py:1265-1272`
- Python Rule flags definition: `reference/scancode-toolkit/src/licensedcode/models.py:1410-1439`
- Rust detection grouping: `src/license_detection/detection.rs:148-208`
- Rust LicenseMatch struct: `src/license_detection/models.rs:174-224`
- Rust Rule struct: `src/license_detection/models.rs:58-171`

---

## Analysis Results

### Status: FIX WAS NEVER IMPLEMENTED

Despite commit `41f08305` claiming to fix PLAN-007, the actual implementation was **never applied**. The code still uses the incorrect string-based heuristics.

### Evidence

The current implementation at `detection.rs:271-279` still reads:

```rust
fn is_license_intro_match(match_item: &LicenseMatch) -> bool {
    match_item.matcher.starts_with("5-unknown") || match_item.rule_identifier.contains("intro")
}

fn is_license_clue_match(match_item: &LicenseMatch) -> bool {
    match_item.matcher == "5-unknown" || match_item.rule_identifier.contains("clue")
}
```

These are the **exact same incorrect heuristics** that PLAN-007 was supposed to fix.

### Why Golden Tests Showed Minimal Improvement

The commit message claimed "24 fewer failures" but this was due to OTHER fixes in the same commit (PLAN-008 through PLAN-014), not PLAN-007. The PLAN-007 changes were simply never applied.

### Root Cause Analysis

1. **Commit claimed fix but didn't apply it**: The commit `41f08305` modified the grouping logic to use `should_group_together()` (PLAN-013) but never replaced the `is_license_intro_match()` and `is_license_clue_match()` implementations.

2. **Multiple layers of intro/clue detection**: There are THREE different functions with similar purposes:
   - `is_license_intro_match()` (line 272) - Used in grouping logic - **BROKEN (string heuristics)**
   - `is_unknown_intro()` (line 490) - Used in `has_unknown_intro_before_detection()` - **BROKEN (string heuristics)**
   - `is_license_intro()` (line 501) - Used in `filter_license_intros()` - **CORRECT (uses boolean flags)**

3. **Missing `has_unknown` field**: Python's `is_unknown_intro()` checks `license_match.rule.has_unknown`, which is a property that returns `True` if `'unknown' in license_expression`. The Rust implementation lacks this field and the corresponding logic.

### Python vs Rust Comparison

| Function | Python Implementation | Rust Implementation | Status |
|----------|----------------------|---------------------|--------|
| `is_unknown_intro()` | `rule.has_unknown AND (rule.is_license_intro OR rule.is_license_clue OR expr == 'free-unknown')` | `matcher.starts_with("5-unknown") && rule_identifier.contains("intro")` | **WRONG** |
| `is_license_intro()` | `(rule.is_license_intro OR rule.is_license_clue OR expr == 'free-unknown') AND (matcher == "2-aho" OR coverage == 100)` | `(is_license_intro OR is_license_clue OR expr == 'free-unknown') AND (matcher == "2-aho" OR coverage >= 99.99)` | **CORRECT** |
| `is_license_clue_match()` | `rule.is_license_clue` | `matcher == "5-unknown" OR rule_identifier.contains("clue")` | **WRONG** |

### Affected Test Cases

Tests that should be fixed by PLAN-007 (from FAILURES.md):

1. **cjdict-liconly.txt**: "Fails due to incorrect license intro detection in grouping logic - the Rust implementation uses a heuristic-based function instead of using the actual `is_license_intro` boolean field from LicenseMatch."

2. **CRC32.java**: "Fails due to `is_license_clue_match()` checking `matcher == "5-unknown"` or `rule_identifier.contains("clue")` instead of the `is_license_clue` boolean field from the rule."

3. **checker-2200.txt**: "Fails due to incorrect grouping logic where `is_license_intro_match()` and `is_license_clue_match()` functions use heuristics instead of checking the actual `is_license_intro` and `is_license_clue` fields."

4. **cpl-1.0_5.txt**: "Fails due to incorrect `is_license_intro_match()` function using string heuristics instead of the actual `is_license_intro` field from the LicenseMatch."

5. **diaspora_copyright.txt**: "Fails due to the `is_license_intro_match` and `is_license_clue_match` functions using heuristic checks instead of the actual boolean fields from `LicenseMatch`."

6. **discourse_COPYRIGHT.txt**: "Fails due to `group_matches_by_region_with_threshold` using string-based heuristics instead of the `is_license_intro` and `is_license_clue` boolean fields."

---

## Remaining TODOs

### Must Fix

1. **Replace `is_license_intro_match()` implementation** (`detection.rs:272-274`):
   ```rust
   fn is_license_intro_match(match_item: &LicenseMatch) -> bool {
       match_item.is_license_intro
           || match_item.is_license_clue
           || match_item.license_expression == "free-unknown"
   }
   ```

2. **Replace `is_license_clue_match()` implementation** (`detection.rs:277-279`):
   ```rust
   fn is_license_clue_match(match_item: &LicenseMatch) -> bool {
       match_item.is_license_clue
   }
   ```

3. **Fix `is_unknown_intro()` implementation** (`detection.rs:490-492`):
   - Add `has_unknown` computed property or inline check
   - Python checks: `rule.has_unknown AND (rule.is_license_intro OR rule.is_license_clue OR expr == 'free-unknown')`
   - `has_unknown` is `True` when `'unknown' in license_expression`
   
   ```rust
   fn is_unknown_intro(m: &LicenseMatch) -> bool {
       let has_unknown = m.license_expression.contains("unknown");
       has_unknown && (m.is_license_intro || m.is_license_clue || m.license_expression == "free-unknown")
   }
   ```

### Should Add

4. **Add `has_unknown` field to Rule struct** (`models.rs`):
   - Alternatively, compute inline since it's just `license_expression.contains("unknown")`

5. **Add unit tests** for the fixed functions:
   - `test_is_license_intro_match_with_flag`
   - `test_is_license_intro_match_with_clue_flag`
   - `test_is_license_intro_match_free_unknown`
   - `test_is_license_clue_match_with_flag`
   - `test_is_unknown_intro_with_has_unknown`

6. **Verify `is_license_intro` and `is_license_clue` are populated correctly**:
   - Check `aho_match.rs`, `hash_match.rs`, `seq_match.rs`, `spdx_lid.rs`, `unknown_match.rs`
   - All should copy these fields from Rule to LicenseMatch

### Validation

7. **Run golden tests after fix**:
   ```bash
   cargo test --lib license_detection::golden_test::golden_tests::test_golden_lic1
   ```

8. **Verify specific test cases pass**:
   - cjdict-liconly.txt
   - CRC32.java  
   - checker-2200.txt
   - cpl-1.0_5.txt
   - diaspora_copyright.txt
   - discourse_COPYRIGHT.txt

---

## Estimated Impact

After implementing the above fixes, expect:
- **~15-20 additional golden tests to pass** (tests failing due to incorrect intro/clue detection)
- No regressions (the current implementation is already broken for these cases)
