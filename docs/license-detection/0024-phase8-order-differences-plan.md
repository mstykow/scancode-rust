# Phase 8: Minor/Order Differences Implementation Plan

**Status:** ✅ Completed  
**Created:** 2025-03-01  
**Last Updated:** 2025-03-01

## Executive Summary

Phase 8 addresses ~4 test failures caused by differences in detection ordering between Rust and Python implementations. These failures are cosmetic (don't affect semantic meaning) but must be fixed for exact parity.

### Problem Statement

Some tests fail because license detections appear in a different order than Python's output, even though the same licenses are detected.

**Example Failures:**

| Test File | Expected | Actual |
|-----------|----------|--------|
| `spdx-license-ids/README.md` | `["bsd-zero", "adobe-scl", "adobe-glyph", ...]` | `["adobe-glyph", "bsd-zero", "adobe-scl", ...]` |
| `mixed_ansible.txt` | `["gpl-2.0", "cc-by-4.0"]` | `["cc-by-4.0", "gpl-2.0"]` |

---

## Root Cause Analysis

### CRITICAL: Previous Analysis Was Incorrect

The expected order in the YAML files is **NOT alphabetical**:
- `mixed_ansible.txt` expects `["gpl-2.0", "cc-by-4.0"]` (gpl before cc-by alphabetically would be wrong)
- Alphabetically sorted would be: `["cc-by-4.0", "gpl-2.0"]`

This proves the expected order is based on **file position**, not alphabetical sorting.

### What the Golden Test Compares

The golden test at `src/license_detection/golden_test.rs:163-167` compares:

```rust
let actual: Vec<&str> = detections
    .iter()
    .flat_map(|d| d.matches.iter())
    .map(|m| m.license_expression.as_str())
    .collect();
```

This is a **flattened list of all match expressions** from all detections, in match order.

The Python test infrastructure at `licensedcode_test_utils.py:215` does:

```python
detected_expressions = [match.rule.license_expression for match in matches]
```

Both extract match-level license expressions in the order matches appear.

### Python's Match Ordering

Python sorts matches using `matches.sort()` at `index.py:1139`, which uses `LicenseMatch.__lt__`:

```python
# match.py:350-354
def __lt__(self, other):
    if not isinstance(other, LicenseMatch):
        return NotImplemented
    return self.qstart < other.qstart  # Sort by file position (token start)
```

**Key insight**: Python sorts matches by **file position (`qstart`)**, NOT alphabetically!

### Rust's Current Match Ordering

Rust sorts matches using `sort_matches_by_line()` at `detection.rs:225-231`:

```rust
pub fn sort_matches_by_line(matches: &mut [LicenseMatch]) {
    matches.sort_by(|a, b| {
        a.start_line
            .cmp(&b.start_line)
            .then_with(|| a.end_line.cmp(&b.end_line))
    });
}
```

This sorts by `start_line`, which should be similar to Python's `qstart` ordering.

### `sort_unique_detections()` is NOT the Issue

The `sort_unique_detections()` function at `detection.py:1003-1014` sorts **UniqueDetection** objects (used in `plugin_license.py`), NOT the raw matches that the golden tests compare.

The golden tests use the raw match order from `idx.match()`, which is sorted by file position.

---

## Updated Root Cause Hypothesis

Since both Python and Rust should be sorting matches by file position, the difference must come from:

1. **Different match detection order** - The order matches are found may differ
2. **Different match filtering** - Different matches may be filtered out
3. **Different detection grouping** - Matches may be grouped differently
4. **Different line number calculation** - `start_line` vs `qstart` may not align perfectly

### Investigation Needed

1. **Compare actual match sequences**: Run both Python and Rust on the same file and compare match-by-match
2. **Check `qstart` vs `start_line`**: Verify they represent the same thing
3. **Check match filtering**: Are the same matches being kept/filtered?
4. **Check detection grouping**: Are matches being grouped into detections the same way?

---

## Investigation Steps

### Step 1: Debug Output Comparison

Add debug output to both Python and Rust to compare:

**Rust** (add to `detect()` in `mod.rs`):
```rust
#[cfg(debug_assertions)]
for m in &sorted {
    eprintln!("MATCH: {} at line {} (qstart={})", 
        m.license_expression, m.start_line, m.start_token);
}
```

**Python** (run separately):
```python
for match in matches:
    print(f"MATCH: {match.rule.license_expression} at line {match.start_line} (qstart={match.qstart})")
```

### Step 2: Compare Match Sequences

Run on the failing test files:
```bash
# On spdx-license-ids/README.md
# On mixed_ansible.txt
```

Compare the order of matches and identify where they diverge.

### Step 3: Check if Issue is Token vs Line Based

Python's `qstart` is token-based, while Rust uses `start_line`. This could cause ordering differences when multiple matches are on the same line.

---

## Potential Fixes (After Investigation)

### If Issue is Token vs Line Ordering

Change `sort_matches_by_line` to sort by token position:

```rust
pub fn sort_matches_by_position(matches: &mut [LicenseMatch]) {
    matches.sort_by(|a, b| {
        a.start_token
            .cmp(&b.start_token)
            .then_with(|| a.end_token.cmp(&b.end_token))
    });
}
```

### If Issue is Match Detection Order

Investigate the Aho-Corasick and hash matching order in both implementations.

### If Issue is Match Filtering

Compare the `refine_matches` logic between Python and Rust.

---

## Test Cases to Verify

### Primary Test Files

These are the specific test files that should pass after the fix:

1. **`spdx-license-ids/README.md`**
   - Expected: `["bsd-zero", "adobe-scl", "adobe-glyph", "afl-1.1", "cc0-1.0"]`
   - Note: This order is NOT alphabetical (would be adobe-glyph, adobe-scl, afl-1.1, bsd-zero, cc0-1.0)
   - The order reflects file position ordering

2. **`mixed_ansible.txt`**
   - Expected: `["gpl-2.0", "cc-by-4.0"]`
   - Note: gpl-2.0 appears before cc-by-4.0 in the file (line 9 vs line 12)

### Test Command

```bash
# Run specific golden tests with verbose output
cargo test --lib test_golden_lic3 -- --test-threads=1 2>&1 | grep -i "mixed_ansible"

# Run all golden tests to check for regressions
cargo test --release -q --lib license_detection::golden_test
```

---

## Additional Considerations

### Why the Original Analysis Was Wrong

1. `sort_unique_detections()` is used for the final `license_detections` output in `plugin_license.py`, not for the raw matches in the datadriven tests
2. The golden tests compare `[match.rule.license_expression for match in matches]`, not detection expressions
3. Matches in Python are sorted by `qstart` (file position), not alphabetically

### Next Steps

1. Run detailed comparison of match sequences between Python and Rust
2. Identify the exact point where ordering diverges
3. Update this plan with correct root cause and fix

---

## Validation Checklist

- [ ] Investigate actual match ordering differences
- [ ] Identify root cause of ordering differences
- [ ] Implement correct fix
- [ ] Run `cargo test --lib license_detection::golden_test`
- [ ] Verify `mixed_ansible.txt` test passes
- [ ] Verify `spdx-license-ids/README.md` test passes
- [ ] Run `cargo clippy` with no warnings
- [ ] Run `cargo fmt`

---

## Files to Investigate

| File | Purpose |
|------|---------|
| `src/license_detection/mod.rs` | Main detection flow |
| `src/license_detection/detection.rs` | Match sorting, detection grouping |
| `src/license_detection/query.rs` | Query/token handling |
| `reference/scancode-toolkit/src/licensedcode/index.py` | Python match flow |
| `reference/scancode-toolkit/src/licensedcode/match.py` | Python match comparison |

---

## Estimated Effort

- **Investigation:** 2-4 hours
- **Implementation:** 1-2 hours (depends on root cause)
- **Testing:** 1 hour
- **Total:** 4-7 hours

---

## References

- **Roadmap:** `docs/license-detection/0016-feature-parity-roadmap.md`
- **Python Match Sorting:** `reference/scancode-toolkit/src/licensedcode/match.py:350-354`
- **Python Index Match Flow:** `reference/scancode-toolkit/src/licensedcode/index.py:1139`
- **Python Test Utils:** `reference/scancode-toolkit/src/licensedcode_test_utils.py:215`
- **Rust Match Sorting:** `src/license_detection/detection.rs:225-231`
