# PLAN-086: here-proprietary_4.RULE Investigation

## Status: FIX CAUSED REGRESSION - NEEDS REVISION

**Attempt**: Modifying `filter_contained_matches()` to check `license_expression` before deduplicating same-qspan matches.

**Before fix**: 4115 passed, 248 failed
**After fix**: 4113 passed, 250 failed (net -2 passed, +2 failed)

**Issue**: The fix correctly handles `here-proprietary_4.RULE` but breaks other tests where same-position matches with different expressions should still interact with containment logic.

**Root cause**: The fix skips the `qcontains` check entirely when `same_qspan` is true, which prevents legitimate containment filtering for other cases.

**Next approach**: Need to investigate why the duplicate matches are created in the first place (SPDX-LID + Aho both matching same text) and fix at the source, rather than in the filtering stage.

## Problem Statement

**File**: `testdata/license-golden/datadriven/lic4/here-proprietary_4.RULE`

| Expected (Python) | Actual (Rust) |
|-------------------|---------------|
| `["here-proprietary"]` (1) | `["here-proprietary", "here-proprietary"]` (2) |

**Issue**: Duplicate detection - same license expression appears twice.

## Root Cause Analysis

### File Content
```
SPDX-License-Identifier: LicenseRef-Proprietary-HERE
```

### Python Behavior
Running Python reference:
```bash
./reference/scancode-toolkit/scancode --license --license-text --json-pp - testdata/license-golden/datadriven/lic4/here-proprietary_4.RULE
```

**Result**: 1 detection with 1 match
- `license_expression: "here-proprietary"`
- `matcher: "1-spdx-id"`
- `start_line: 1, end_line: 1`
- `rule_identifier: "spdx-license-identifier-here_proprietary-dca785f31180436b2f12a8879c6893bcb87f2e61"`

### Rust Behavior
Running Rust scanner:
```bash
cargo run --release --bin scancode-rust -- testdata/license-golden/datadriven/lic4 -o /tmp/rust-output.json
```

**Result**: 1 detection with **2 matches**
Both matches:
- `license_expression: "here-proprietary"`
- `start_line: 1, end_line: 1`
- Identical positions

### Duplicate Source

Two rules can match this text:

1. **`spdx_license_id_licenseref-proprietary-here_for_here-proprietary.RULE`**
   - `license_expression: here-proprietary`
   - `is_license_reference: yes`
   - `is_required_phrase: yes`
   - `relevance: 50`
   - Token: `licenseref-proprietary-here`

2. **`here-proprietary_4.RULE`** (from license file)
   - `license_expression: here-proprietary`
   - `is_license_tag: yes`
   - `relevance: 100`
   - Contains `{{HERE Proprietary}}` placeholder

### Why Duplicates Occur

1. **SPDX-LID matcher** (`spdx_lid_match()`) creates a match from parsing `LicenseRef-Proprietary-HERE`
2. **Aho-Corasick matcher** (`aho_match()`) also matches the token sequence
3. Both matches have:
   - Same `license_expression: "here-proprietary"`
   - Same token positions (`start_line: 1, end_line: 1`)
   - **Different `rule_identifier` values**

### Why `merge_overlapping_matches()` Doesn't Merge

**Current behavior** (match_refine.rs:196-339):
```rust
// Line 219: Groups by rule_identifier FIRST
if current_group.is_empty() || current_group[0].rule_identifier == m.rule_identifier {
    current_group.push(m);
} else {
    grouped.push(current_group);
    current_group = vec![m];
}
```

Matches with different `rule_identifier` are processed in **separate groups**, so they are never compared or merged together.

### Why `filter_contained_matches()` Should Handle This

Python's `filter_contained_matches()` (match.py:1137-1156):
```python
# Equals matched spans - removes duplicates across different rules
if current_match.qspan == next_match.qspan:
    if current_match.coverage() >= next_match.coverage():
        discarded_append(matches_pop(j))
        continue
    else:
        discarded_append(matches_pop(i))
        i -= 1
        break
```

Rust's `filter_contained_matches()` (match_refine.rs:392-400):
```rust
if current.qstart() == next.qstart() && current.end_token == next.end_token {
    if current.match_coverage >= next.match_coverage {
        discarded.push(matches.remove(j));
        continue;
    } else {
        discarded.push(matches.remove(i));
        i = i.saturating_sub(1);
        break;
    }
}
```

The logic appears equivalent, but duplicates are still appearing in output.

### Key Code Paths

1. `src/license_detection/mod.rs:169-183` - SPDX-LID matching phase
2. `src/license_detection/mod.rs:185-201` - Aho-Corasick matching phase
3. `src/license_detection/match_refine.rs:196-339` - `merge_overlapping_matches()` function
4. `src/license_detection/match_refine.rs:363-419` - `filter_contained_matches()` function
5. `src/license_detection/match_refine.rs:1574` - Where `filter_contained_matches()` is called

---

## Implementation Plan

### Overview

The fix involves adding a **same-position deduplication pass** before the rule-based grouping in `merge_overlapping_matches()`. This aligns with Python's behavior where matches with identical positions are deduplicated regardless of rule.

### Design Principle

**Deduplicate same-position matches early, keeping the one with higher coverage/relevance.**

This is more efficient than relying solely on `filter_contained_matches()` later in the pipeline, and ensures consistent behavior across all code paths.

### Phase 1: Add Same-Position Deduplication to `merge_overlapping_matches()`

**File**: `src/license_detection/match_refine.rs`

**Location**: Before the rule_identifier grouping (before line 215)

**Implementation**:

```rust
pub fn merge_overlapping_matches(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    if matches.is_empty() {
        return Vec::new();
    }

    if matches.len() == 1 {
        return matches.to_vec();
    }

    // PHASE 1: Deduplicate matches with identical positions (qspan and ispan)
    // This handles the case where different rules produce matches at the same position.
    // Keep the match with higher coverage; if equal, keep higher relevance.
    // Based on Python's filter_contained_matches() qspan equality check.
    let mut deduped: Vec<LicenseMatch> = matches.to_vec();
    deduped.sort_by(|a, b| {
        a.qstart()
            .cmp(&b.qstart())
            .then_with(|| a.end_token.cmp(&b.end_token))
            .then_with(|| b.match_coverage.partial_cmp(&a.match_coverage).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| b.rule_relevance.cmp(&a.rule_relevance))
    });

    let mut i = 0;
    while i < deduped.len().saturating_sub(1) {
        let mut j = i + 1;
        while j < deduped.len() {
            let current = &deduped[i];
            let next = &deduped[j];

            // Same position check - qspan AND ispan must match
            if current.qstart() == next.qstart()
                && current.end_token == next.end_token
                && current.ispan() == next.ispan()
            {
                // Remove the lower coverage match
                // Coverage already considered in sort, so remove j
                deduped.remove(j);
                continue;
            }

            // If positions differ, no more same-position duplicates possible
            if next.qstart() != current.qstart() || next.end_token != current.end_token {
                break;
            }

            j += 1;
        }
        i += 1;
    }

    // PHASE 2: Existing rule-based grouping and merging
    // (continue with existing logic using deduped instead of matches)
    let mut sorted: Vec<&LicenseMatch> = deduped.iter().collect();
    // ... rest of existing code ...
}
```

**Key Points**:
1. Sort by position first, then by coverage (descending), then by relevance (descending)
2. Only consider matches as duplicates if BOTH qspan AND ispan are identical
3. Remove lower-priority matches in the same-position pass
4. Continue with existing rule-based merging after deduplication

### Phase 2: Add Comprehensive Tests

**File**: `src/license_detection/match_refine.rs` (in the tests module)

**Tests to Add**:

```rust
#[test]
fn test_merge_overlapping_matches_same_position_different_rule() {
    // Reproduces PLAN-086: Same position, different rule_identifier
    let mut m1 = create_test_match("#1", 1, 1, 0.9, 100.0, 100);
    m1.rule_identifier = "#spdx-license-identifier".to_string();
    m1.license_expression = "here-proprietary".to_string();

    let mut m2 = create_test_match("#2", 1, 1, 0.85, 95.0, 95);
    m2.rule_identifier = "#aho-rule".to_string();
    m2.license_expression = "here-proprietary".to_string();
    // Same position
    m2.start_token = m1.start_token;
    m2.end_token = m1.end_token;

    let matches = vec![m1.clone(), m2.clone()];

    let merged = merge_overlapping_matches(&matches);

    // Should deduplicate to 1 match
    assert_eq!(merged.len(), 1);
    // Should keep the higher coverage match (m1)
    assert_eq!(merged[0].rule_identifier, "#spdx-license-identifier");
    assert_eq!(merged[0].match_coverage, 100.0);
}

#[test]
fn test_merge_overlapping_matches_same_position_keeps_higher_coverage() {
    // Same position, lower coverage match comes first
    let mut m1 = create_test_match("#1", 1, 1, 0.8, 80.0, 80);
    m1.license_expression = "apache-2.0".to_string();

    let mut m2 = create_test_match("#2", 1, 1, 0.95, 95.0, 95);
    m2.license_expression = "apache-2.0".to_string();
    m2.start_token = m1.start_token;
    m2.end_token = m1.end_token;

    let matches = vec![m1, m2];

    let merged = merge_overlapping_matches(&matches);

    assert_eq!(merged.len(), 1);
    // Should keep the higher coverage (95.0)
    assert_eq!(merged[0].match_coverage, 95.0);
}

#[test]
fn test_merge_overlapping_matches_same_position_different_ispan_not_deduped() {
    // Same qspan but different ispan - should NOT be deduplicated
    let mut m1 = create_test_match("#1", 1, 1, 0.9, 100.0, 100);
    m1.start_token = 0;
    m1.end_token = 2;
    m1.rule_start_token = 0;
    m1.rule_end_token = 2;

    let mut m2 = create_test_match("#2", 1, 1, 0.85, 95.0, 95);
    m2.start_token = 0;
    m2.end_token = 2;
    m2.rule_start_token = 10;  // Different ispan
    m2.rule_end_token = 12;

    let matches = vec![m1, m2];

    let merged = merge_overlapping_matches(&matches);

    // Should NOT deduplicate because ispan differs
    assert_eq!(merged.len(), 2);
}

#[test]
fn test_merge_overlapping_matches_same_position_multiple_matches() {
    // Three matches at same position, different rules
    let mut m1 = create_test_match("#1", 1, 1, 0.9, 90.0, 90);
    let mut m2 = create_test_match("#2", 1, 1, 0.95, 95.0, 95);
    let mut m3 = create_test_match("#3", 1, 1, 0.85, 85.0, 85);

    // Same positions
    for m in [&mut m1, &mut m2, &mut m3].iter_mut() {
        m.start_token = 0;
        m.end_token = 2;
        (**m).rule_start_token = 0;
        (**m).rule_end_token = 2;
    }

    let matches = vec![m1, m2, m3];

    let merged = merge_overlapping_matches(&matches);

    // Should deduplicate to 1 match
    assert_eq!(merged.len(), 1);
    // Should keep the highest coverage (95.0)
    assert_eq!(merged[0].match_coverage, 95.0);
}

#[test]
fn test_merge_overlapping_matches_preserves_different_positions() {
    // Matches at different positions should not be affected
    let m1 = create_test_match("#1", 1, 10, 0.9, 100.0, 100);
    let m2 = create_test_match("#2", 20, 30, 0.85, 95.0, 95);

    let matches = vec![m1.clone(), m2.clone()];

    let merged = merge_overlapping_matches(&matches);

    assert_eq!(merged.len(), 2);
}
```

### Phase 3: Golden Test Verification

**Run golden tests** to verify the fix:

```bash
# Run the specific golden test for PLAN-086
cargo test testdata_license_golden_datadriven_lic4 --release

# Run all license golden tests
cargo test golden_tests::license --release
```

**Expected result**: `here-proprietary_4.RULE` should produce 1 match, not 2.

### Phase 4: Regression Testing

**Run full test suite** to ensure no regressions:

```bash
cargo test --release
```

**Key areas to verify**:
1. All existing `merge_overlapping_matches` tests pass
2. All `filter_contained_matches` tests pass
3. License detection golden tests pass
4. No new duplicate detection issues introduced

### Phase 5: Documentation

**Update** `docs/license-detection/architecture/04-refinement.md` if necessary to document the new deduplication behavior.

---

## Alternative Approaches Considered

### Alternative 1: Fix Only in `filter_contained_matches()`

**Pros**: Matches Python's architecture more closely
**Cons**: 
- Duplicates go through more of the pipeline before being removed
- Less efficient
- The issue might persist due to subtle differences in how `filter_contained_matches()` processes matches

**Decision**: Rejected. Earlier deduplication is more efficient and catches the issue at the source.

### Alternative 2: Add Separate `deduplicate_same_position_matches()` Function

**Pros**: More modular, easier to test in isolation
**Cons**: 
- Additional function call in the pipeline
- Requires coordination with existing merge logic

**Decision**: Rejected. Integrating into `merge_overlapping_matches()` is simpler and more efficient.

---

## Files to Modify

| File | Change |
|------|--------|
| `src/license_detection/match_refine.rs` | Add same-position deduplication to `merge_overlapping_matches()` |
| `src/license_detection/match_refine.rs` | Add unit tests for new behavior |

---

## Verification Checklist

Before marking as complete:

- [ ] `merge_overlapping_matches()` deduplicates same-position matches from different rules
- [ ] Match with higher coverage is kept when positions are identical
- [ ] Match with higher relevance is kept when coverage is equal
- [ ] Matches with different positions are not affected
- [ ] All existing tests pass
- [ ] PLAN-086 golden test passes (1 match instead of 2)
- [ ] No regressions in other golden tests
- [ ] Code passes `cargo clippy` without warnings
- [ ] Code is formatted with `cargo fmt`

---

## References

- Python reference: `reference/scancode-toolkit/src/licensedcode/match.py:1075-1184` (`filter_contained_matches`)
- Python reference: `reference/scancode-toolkit/tests/licensedcode/test_match.py:770-781` (test case for same-span different-rule)
- Rust current: `src/license_detection/match_refine.rs:196-339` (`merge_overlapping_matches`)
- Rust current: `src/license_detection/match_refine.rs:363-419` (`filter_contained_matches`)
