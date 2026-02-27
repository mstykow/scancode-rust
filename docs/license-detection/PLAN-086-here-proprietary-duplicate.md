# PLAN-086: here-proprietary_4.RULE Investigation

## Status: NEEDS DIFFERENT APPROACH

**Previous fix was incorrect**: The validation found that Python's `query_run.subtract()` is called on a LOCAL variable, not the main query.

**Validation findings**:
- Python's `index.py:672` subtracts from a local `query_run` inside the SPDX matching function
- The main query is only modified for long license texts (`is_license_text && length > 120 && coverage > 98`)
- SPDX matches ARE tracked in `already_matched_qspans` for 100% coverage matches

**Next investigation needed**:
- Check if Rust correctly tracks SPDX matches in `matched_qspans`
- Verify the Aho matcher respects `matched_qspans` when checking matchables
- The issue may be in how Aho filters matches, not in query subtraction

### Root Cause (Confirmed via investigation)

**Python's approach** (index.py:664-675):
1. SPDX-LID matcher runs and creates a match
2. `query_run.subtract(spdx_match.qspan)` is called unconditionally (line 672)
3. This removes matched positions from `query.high_matchables` and `query.low_matchables`
4. When Aho matcher runs next (line 684), it calls `get_matched_spans()` (match_aho.py:141-159)
5. This checks if `qspan` is entirely within `matchables` (line 152)
6. Since SPDX already removed those positions, Aho match is **discarded**

**Rust's current approach** (mod.rs:168-201):
1. SPDX-LID matcher runs and creates a match
2. Subtract only happens if `is_license_text && rule_length > 120 && match_coverage > 98.0` (line 176-180)
3. This condition is NOT met for SPDX matches (they typically have `is_license_text=false`)
4. `matched_qspans` is updated (line 174) but Aho matcher doesn't use it
5. Aho matcher reads `query_run.matchables(true)` which still contains all positions
6. Both matches appear in output → **duplicate**

**The key difference**: Python unconditionally subtracts SPDX match qspan from query matchables. Rust only subtracts under a specific condition that SPDX matches don't meet.

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

The fix involves **subtracting SPDX match qspan from query matchables unconditionally**, matching Python's behavior. This prevents Aho from matching the same positions.

### Design Principle

**Match Python's behavior exactly**: After SPDX-LID matching, subtract the matched positions from the query so subsequent matchers (Aho, Seq) don't re-match them.

### The Fix

**File**: `src/license_detection/mod.rs`

**Location**: Lines 168-183 (SPDX-LID matching phase)

**Current code** (incorrect):
```rust
// Phase 1b: SPDX-LID matching
{
    let spdx_matches = spdx_lid_match(&self.index, &query);
    let merged_spdx = merge_overlapping_matches(&spdx_matches);
    for m in &merged_spdx {
        if m.match_coverage >= 99.99 && m.end_token > m.start_token {
            matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
        }
        // WRONG: Only subtracts under specific conditions
        if m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0 {
            let span =
                query::PositionSpan::new(m.start_token, m.end_token.saturating_sub(1));
            query.subtract(&span);
        }
    }
    all_matches.extend(merged_spdx);
}
```

**Fixed code** (matching Python):
```rust
// Phase 1b: SPDX-LID matching
{
    let spdx_matches = spdx_lid_match(&self.index, &query);
    let merged_spdx = merge_overlapping_matches(&spdx_matches);
    for m in &merged_spdx {
        if m.match_coverage >= 99.99 && m.end_token > m.start_token {
            matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
        }
        // FIX: Unconditionally subtract SPDX match qspan (matches Python line 672)
        // This prevents Aho from re-matching the same positions
        if m.end_token > m.start_token {
            let span = query::PositionSpan::new(m.start_token, m.end_token.saturating_sub(1));
            query.subtract(&span);
        }
        
        // Keep the long license text subtraction for additional coverage
        if m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0 {
            let span =
                query::PositionSpan::new(m.start_token, m.end_token.saturating_sub(1));
            query.subtract(&span);
        }
    }
    all_matches.extend(merged_spdx);
}
```

**Key change**: Remove the conditional `is_license_text && rule_length > 120 && match_coverage > 98.0` guard for SPDX matches. Always subtract the matched positions.

### Why This Fix Works

1. Python unconditionally subtracts SPDX match positions (index.py:672)
2. This removes positions from `query.high_matchables` and `query.low_matchables`
3. Aho's `get_matched_spans()` checks if positions are in `matchables` (match_aho.py:152)
4. Since positions were removed, Aho discards the duplicate match
5. Rust needs to do the same to prevent duplicates

### Verification Steps

1. Run the specific golden test:
   ```bash
   cargo test testdata_license_golden_datadriven_lic4 --release
   ```

2. Run all license golden tests:
   ```bash
   cargo test golden_tests::license --release
   ```

3. Verify `here-proprietary_4.RULE` produces 1 match instead of 2

**Expected result**: `here-proprietary_4.RULE` should produce 1 match, not 2.

---

## Alternative Approaches Considered

### Alternative 1: Fix Only in `filter_contained_matches()` (Previously Attempted)

**Result**: Caused regressions (-2 passed, +2 failed)

**Why it failed**: Modifying `filter_contained_matches()` to check `license_expression` before deduplicating same-qspan matches broke other tests where same-position matches with different expressions should still interact with containment logic.

### Alternative 2: Same-Position Deduplication in `merge_overlapping_matches()`

**Pros**: Catches duplicates at the merge stage
**Cons**: 
- Doesn't match Python's approach (Python prevents at source)
- Adds complexity to merge logic
- May not catch all cases

**Decision**: Rejected. Fix at the source (SPDX subtraction) matches Python's behavior.

---

## Files to Modify

| File | Change |
|------|--------|
| `src/license_detection/mod.rs` | Unconditionally subtract SPDX match qspan from query matchables |

---

## Verification Checklist

Before marking as complete:

- [ ] SPDX match qspan is subtracted unconditionally from query matchables
- [ ] Aho matcher no longer matches positions already matched by SPDX-LID
- [ ] `here-proprietary_4.RULE` produces 1 match instead of 2
- [ ] All existing tests pass
- [ ] No regressions in other golden tests
- [ ] Code passes `cargo clippy` without warnings
- [ ] Code is formatted with `cargo fmt`

---

## References

- Python reference: `reference/scancode-toolkit/src/licensedcode/index.py:664-675` (SPDX match and subtract)
- Python reference: `reference/scancode-toolkit/src/licensedcode/match_aho.py:141-159` (`get_matched_spans` matchable check)
- Python reference: `reference/scancode-toolkit/src/licensedcode/query.py:328-335` (`Query.subtract()`)
- Python reference: `reference/scancode-toolkit/src/licensedcode/query.py:863-871` (`QueryRun.subtract()`)
- Rust current: `src/license_detection/mod.rs:168-183` (SPDX-LID matching phase)
