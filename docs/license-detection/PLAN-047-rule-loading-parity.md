# PLAN-047: Rule Loading Parity Investigation

## Summary

Investigation confirms that Rust outputs incorrect `rule_identifier` format:

- **Python**: `rule_identifier = "bsd-new_375.RULE"` (string identifier from rule file)
- **Rust**: `rule_identifier = "#5350"` (numeric rid wrapped in `#`)

This is a **HIGH severity** issue breaking golden test comparisons and JSON output parity.

**ROOT CAUSE CONFIRMED**: Rust uses `format!("#{}", rid)` for `rule_identifier` instead of the actual string identifier (`rule.identifier`).

---

## Python Reference Implementation

### Rule Identifier Output

**File**: `reference/scancode-toolkit/src/licensedcode/match.py:820-840`

```python
def to_dict(self, ...):
    result = {}
    result['license_expression'] = self.rule.license_expression
    result['license_expression_spdx'] = self.rule.spdx_license_expression()
    result['from_file'] = file_path
    result['start_line'] = self.start_line
    result['end_line'] = self.end_line
    result['matcher'] = self.matcher
    result['score'] = self.score()
    result['matched_length'] = self.len()
    result['match_coverage'] = self.coverage()
    result['rule_relevance'] = self.rule.relevance
    result['rule_identifier'] = self.rule.identifier  # LINE 833: Uses string identifier
    result['rule_url'] = self.rule.rule_url
    return result
```

### Rule Storage and rid Assignment

**File**: `reference/scancode-toolkit/src/licensedcode/index.py:316-386`

```python
# Line 316-317: Convert rules to list and create identifier mapping
self.rules_by_rid = rules_by_rid = list(rules)
self.rules_by_id = {r.identifier: r for r in self.rules_by_rid}

# Line 323: Sort rules by identifier (attr.s compares fields in order)
rules_by_rid.sort()

# Line 383-386: Assign rid sequentially AFTER sorting
for rid, rule in enumerate(rules_by_rid):
    rule.rid = rid  # rid is internal index, NOT used in output
```

### Rule Identifier in Match Creation

**File**: `reference/scancode-toolkit/src/licensedcode/match_hash.py:69`

```python
rule = idx.rules_by_rid[rid]  # Look up by rid
# ... match creation ...
# Output uses rule.identifier, NOT rid
```

**File**: `reference/scancode-toolkit/src/licensedcode/match_aho.py:121,242`

```python
rule = rules_by_rid[rid]  # Look up by rid
# ... match creation ...
# Output uses rule.identifier, NOT rid
```

### Key Insight: Python's Dual-Purpose Design

Python maintains BOTH:

1. `rid` - Internal numeric index for `rules_by_rid[rid]` lookups
2. `identifier` - String identifier for JSON output (e.g., `"bsd-new_375.RULE"`)

The `LicenseMatch` object stores a **reference to the Rule object** (`self.rule`), so it can access `self.rule.identifier` for output.

---

## Rust Current Implementation

### Match Creation (INCORRECT)

**File**: `src/license_detection/hash_match.rs:100-127`

```rust
let license_match = LicenseMatch {
    license_expression: rule.license_expression.clone(),
    // ... other fields ...
    rule_identifier: format!("#{}", rid),  // WRONG: Should be rule.identifier.clone()
    // ...
};
```

**File**: `src/license_detection/aho_match.rs:160-188`

```rust
let license_match = LicenseMatch {
    // ...
    rule_identifier: format!("#{}", rid),  // WRONG
    // ...
};
```

**File**: `src/license_detection/seq_match.rs:744,875`

```rust
rule_identifier: format!("#{}", rid),  // WRONG
```

**File**: `src/license_detection/spdx_lid.rs:288`

```rust
rule_identifier: format!("#{}", rid),  // WRONG
```

### Internal rid Lookup (DEPENDS ON # FORMAT)

**File**: `src/license_detection/match_refine.rs:71,401,458,627,631,929,970,1030,1378`

```rust
// This function PARSES the "#<number>" format
fn parse_rule_id(rule_identifier: &str) -> Option<usize> {
    let trimmed = rule_identifier.trim();
    if let Some(stripped) = trimmed.strip_prefix('#') {
        stripped.parse().ok()
    } else {
        trimmed.parse().ok()
    }
}

// Usage examples (9 locations total):
if let Some(rid) = parse_rule_id(&m.rule_identifier)
    && let Some(rule) = index.rules_by_rid.get(rid)
{
    // ... use rule for filtering, merging, etc.
}
```

### LicenseMatch Struct

**File**: `src/license_detection/models.rs:207-254`

```rust
pub struct LicenseMatch {
    pub license_expression: String,
    // ... other fields ...
    pub rule_identifier: String,  // Currently "#5350", should be "bsd-new_375.RULE"
    // ...
}
```

**NOTE**: The struct does NOT have a `rid` field. The `parse_rule_id()` function extracts rid from the string.

---

## Root Cause Analysis

### The Dual-Purpose Problem

**Python Design** (correct):

- `LicenseMatch` holds a **reference** to the `Rule` object (`self.rule`)
- `rid` is stored on the `Rule` object itself (`rule.rid`)
- Output uses `self.rule.identifier` directly
- Lookups use `idx.rules_by_rid[rid]` where `rid` is extracted from rule object

**Rust Design** (problematic):

- `LicenseMatch` is a standalone struct (no rule reference - would complicate ownership)
- `rule_identifier` serves BOTH purposes:
  1. JSON output (expected: `"bsd-new_375.RULE"`)
  2. Internal rid lookup (currently: `"#5350"` parsed to get `5350`)
- The `#` prefix format was added to enable parsing

### Why This Matters

1. **Golden tests fail**: Expected `"bsd-new_375.RULE"`, got `"#5350"`
2. **JSON output differs**: Users/tools expect human-readable identifiers
3. **Different rules appearing to match**: The identifier is just wrong, not different rules

---

## Step-by-Step Implementation Plan

### Step 1: Add `rid` Field to LicenseMatch

**File**: `src/license_detection/models.rs:207`

Add a new field:

```rust
pub struct LicenseMatch {
    // ... existing fields ...
    
    /// Internal rule ID for looking up rules in rules_by_rid.
    /// Not serialized to JSON.
    #[serde(skip)]
    pub rid: usize,
    
    /// Human-readable rule identifier for JSON output.
    /// e.g., "bsd-new_375.RULE" or "mit.LICENSE"
    pub rule_identifier: String,
    
    // ... other fields ...
}
```

**Update Default implementation**:

```rust
impl Default for LicenseMatch {
    fn default() -> Self {
        LicenseMatch {
            // ... existing defaults ...
            rid: 0,
            rule_identifier: String::new(),
            // ...
        }
    }
}
```

### Step 2: Update All Match Creation Sites

**Files to update** (6 locations):

| File | Line | Change |
|------|------|--------|
| `hash_match.rs` | 113 | `rid, rule_identifier: rule.identifier.clone()` |
| `aho_match.rs` | 174 | `rid, rule_identifier: rule.identifier.clone()` |
| `seq_match.rs` | 744 | `rid, rule_identifier: rule.identifier.clone()` |
| `seq_match.rs` | 875 | `rid, rule_identifier: rule.identifier.clone()` |
| `spdx_lid.rs` | 288 | `rid, rule_identifier: rule.identifier.clone()` |
| `detection.rs` | 4290 | Already uses string format, add `rid` |

**Example change**:

```rust
// BEFORE:
LicenseMatch {
    // ...
    rule_identifier: format!("#{}", rid),
    // ...
}

// AFTER:
LicenseMatch {
    // ...
    rid,
    rule_identifier: rule.identifier.clone(),
    // ...
}
```

### Step 3: Replace parse_rule_id() Calls with Direct rid Access

**File**: `src/license_detection/match_refine.rs`

Replace all 9 occurrences:

| Line | Before | After |
|------|--------|-------|
| 71 | `if let Some(rid) = parse_rule_id(&m.rule_identifier)` | `let rid = m.rid;` |
| 401 | `if let Some(rid) = parse_rule_id(&m.rule_identifier)` | `let rid = m.rid;` |
| 458 | `parse_rule_id(&m.rule_identifier)` | `m.rid` |
| 627 | `let current_ends = parse_rule_id(...)` | `let current_ends = matches[i].rid;` |
| 631 | `let next_starts = parse_rule_id(...)` | `let next_starts = matches[j].rid;` |
| 929 | `if let Some(rid) = parse_rule_id(...)` | `let rid = m.rid;` |
| 970 | `if let Some(rid) = parse_rule_id(...)` | `let rid = m.rid;` |
| 1030 | `let rid = match parse_rule_id(...)` | `let rid = m.rid;` |
| 1378 | `if let Some(rid) = parse_rule_id(...)` | `let rid = m.rid;` |

**After changes**, the `parse_rule_id()` function can be removed or kept as dead code.

### Step 4: Update Tests

**File**: `src/license_detection/match_refine.rs` (tests at lines 1574-1596, 2442-2444)

Remove or update tests for `parse_rule_id()`:

```rust
// These tests can be removed since parse_rule_id() will no longer be used:
#[test]
fn test_parse_rule_id_valid_hashes() { ... }

#[test]
fn test_parse_rule_id_plain_numbers() { ... }

#[test]
fn test_parse_rule_id_invalid_formats() { ... }

#[test]
fn test_parse_rule_id_with_whitespace() { ... }
```

---

## Files to Modify (Summary)

| File | Changes | Lines |
|------|---------|-------|
| `src/license_detection/models.rs` | Add `rid` field, update Default | ~254, ~322-354 |
| `src/license_detection/hash_match.rs` | Update match creation | 113 |
| `src/license_detection/aho_match.rs` | Update match creation | 174 |
| `src/license_detection/seq_match.rs` | Update match creation (2 locations) | 744, 875 |
| `src/license_detection/spdx_lid.rs` | Update match creation | 288 |
| `src/license_detection/detection.rs` | Add `rid` field | 4290 |
| `src/license_detection/match_refine.rs` | Replace 9 `parse_rule_id()` calls, remove tests | 71,401,458,627,631,929,970,1030,1378,1574-1596,2442-2444 |

**Total**: 7 files, ~20 locations

---

## Verification Steps

1. **Build and test**:

   ```bash
   cargo build
   cargo test --lib
   ```

2. **Run golden tests**:

   ```bash
   cargo test test_golden --test license_detection_golden_test
   ```

3. **Compare JSON output**:

   ```bash
   cargo run -- <test-file> -o output.json
   # Check that rule_identifier is now "mit.LICENSE" not "#1234"
   ```

---

## Expected Impact

After this fix:

- `rule_identifier` will output human-readable strings like `"bsd-new_375.RULE"`
- Golden tests will match Python output format
- JSON output will be compatible with tools expecting the Python format
- The `rid` field enables efficient internal lookups without string parsing

---

## Related Documents

- `docs/license-detection/PLAN-046-regression-root-cause-analysis.md` - Original analysis identifying the issue
- `docs/license_detection_comparison_report.md` - Pipeline comparison

## Implementation Status

**IMPLEMENTED** - 2026-02-24

### Verification Results

#### Step 1: `rid` field added to `LicenseMatch` ✅

- `src/license_detection/models.rs:210-211` - `#[serde(skip)] pub rid: usize,`

#### Step 2: All match creation sites updated ✅

| File | Line | Status |
|------|------|--------|
| `hash_match.rs` | 113-114 | ✅ `rid, rule_identifier: rule.identifier.clone()` |
| `aho_match.rs` | 174-175 | ✅ `rid, rule_identifier: rule.identifier.clone()` |
| `seq_match.rs` | 744-745 | ✅ `rid, rule_identifier: candidate.rule.identifier.clone()` |
| `seq_match.rs` | 876-877 | ✅ `rid, rule_identifier: candidate.rule.identifier.clone()` |
| `spdx_lid.rs` | 288-289 | ✅ `rid, rule_identifier: rule.identifier.clone()` |
| `detection.rs` | 4284,4297 | ✅ Test helper uses string format |

#### Step 3: `parse_rule_id()` calls replaced ✅

All 9 production code sites now use direct `m.rid` access instead of `parse_rule_id()`.

**Note:** `parse_rule_id()` is still used in test helper functions (`create_test_match()`, `create_test_match_with_tokens()`, `create_test_match_with_flags()`) to derive `rid` from `rule_identifier` strings for backward compatibility in tests.

---

## Regression Analysis

### Golden Test Results

**Before (baseline):**

- lic4: 304 passed, 46 failed
- external: 2175 passed, 392 failed
- Total: 3580 passed, 583 failed

**After PLAN-047:**

- lic4: 303 passed, 47 failed (-1)
- external: 2170 passed, 397 failed (-5)
- Total: 3574 passed, 589 failed (**regression of 6 tests**)

### Root Cause of Regression

The `rule_identifier` format change affects **sorting behavior** in two locations:

#### Location 1: Match Grouping (match_refine.rs:162-169)

```rust
sorted.sort_by(|a, b| {
    a.rule_identifier
        .cmp(&b.rule_identifier)  // Primary sort key
        .then_with(|| a.qstart().cmp(&b.qstart()))
        ...
});
```

This groups matches by the same rule for merging.

#### Location 2: Overlap Resolution Tiebreaker (match_refine.rs:514-521)

```rust
matches.sort_by(|a, b| {
    a.qstart()
        .cmp(&b.qstart())
        ...
        .then_with(|| a.rule_identifier.cmp(&b.rule_identifier))  // Final tiebreaker
});
```

### Sorting Difference Analysis

**Before (numeric `#<number>` format):**

- Lexicographic sort: `"#1" < "#10" < "#100" < "#2"` (non-numeric)
- Sorting was deterministic but arbitrary based on rid assignment

**After (string identifier format):**

- Alphabetical sort: `"apache-2.0.LICENSE" < "bsd-new_375.RULE" < "mit.LICENSE"`
- Sorting is now deterministic based on human-readable identifiers

### Impact

1. **Different match grouping order**: When multiple matches exist for the same file region, the string-based sorting groups them differently than numeric sorting.

2. **Different tiebreaker selection**: When two matches have identical positions/scores, the tiebreaker now selects based on alphabetical identifier order instead of numeric rid order.

3. **This is NOT necessarily a bug**: The Python implementation also uses string identifiers for sorting. The regression may indicate:
   - Rust's rid assignment order differs from Python's
   - Different rules are being matched in edge cases
   - Match merging behaves differently due to grouping order changes

### Specific Regressed Tests

The 6 tests that regressed after PLAN-047:

| Test File | Suite |
|-----------|-------|
| `datadriven/external/fossology-tests/Dual-license/BSD-style_or_LGPL-2.1+.txt` | external |
| `datadriven/external/fossology-tests/EFL/epanel.h` | external |
| `datadriven/external/fossology-tests/Freeware/app_exec.c` | external |
| `datadriven/external/fossology-tests/GPL/curve.c` | external |
| `datadriven/external/fossology-tests/GPL/gpl-test2.txt` | external |
| `datadriven/lic4/lgpl_21.txt` | lic4 |

### Deeper Root Cause Analysis

The old code used `rule_identifier` as a string tiebreaker in `filter_overlapping_matches()` with `#<rid>` format:

```rust
// Old behavior: rule_identifier = "#17" or "#42"
// String comparison: "#17" < "#42" (lexicographic, but numeric prefix makes this work)
```

The new code uses actual identifiers:

```rust
// New behavior: rule_identifier = "apache-2.0_7.RULE" or "mit.LICENSE"
// String comparison: alphabetical order
```

**The critical issue**: String comparison of `#17` vs `#42` differs from alphabetical comparison of `apache-2.0_7.RULE` vs `mit.LICENSE`.

This changes which match survives overlap filtering in edge cases. When the "wrong" match survives (based on the new tiebreaker order), it can subsequently get filtered out by false positive detection, causing the test to fail.

### Python Behavior

Python's `filter_overlapping_matches()` in `licensedcode/match.py` does NOT use `rule_identifier` as a tiebreaker. The tiebreaker is `matched_length` then `rule_relevance`. Python achieves deterministic ordering through rule sorting during index load, not during match filtering.

### Investigation Needed

1. **Should tiebreaker use `rid` (numeric) instead of `rule_identifier` (string)?**
   - This would preserve the old behavior's numeric ordering
   - But it's arbitrary based on rid assignment order

2. **Should tiebreaker be removed entirely to match Python?**
   - Would achieve Python parity
   - But causes non-deterministic ordering when positions/scores are identical
   - Rust tests expect deterministic output

3. **Does false positive detection need adjustment?**
   - The tests may fail because false positive detection removes matches that Python keeps
   - May need to investigate why different matches survive overlap filtering

---

## Conclusion

PLAN-047 implementation is **complete and correct**. The `rid` field is properly isolated and `rule_identifier` now outputs human-readable strings matching Python's format.

**Status: REGRESSION DETECTED**

The 6-test regression requires further investigation to determine:

- Whether the tiebreaker should use `rid` (numeric) instead of `rule_identifier` (string)
- Whether the tiebreaker should be removed entirely to match Python (but causes non-deterministic ordering)
- Whether false positive detection needs adjustment

**Next step**: Create PLAN-048 to investigate tiebreaker behavior and false positive interaction in overlap filtering.
