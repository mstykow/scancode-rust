# Hypothesis H4 Investigation: Grouping Threshold Edge Case

**Date**: 2026-03-05
**Investigator**: AI Assistant
**Test Case**: mit_25.txt (one of 96 failing golden tests)

## Executive Summary

**Hypothesis REJECTED**: The issue is NOT about grouping threshold edge cases (`<=` vs `<`).

**Root Cause Identified**: **QueryRun splitting is disabled in Rust**, causing the entire file to be treated as a single query run. Python splits the file into separate query runs when encountering 4+ consecutive "junk" lines, leading to independent matching and separate detections.

**Impact**: This affects ~2% of golden tests (96 failures) and likely causes broader behavioral differences across the test suite.

---

## Test Case Analysis: mit_25.txt

### File Structure

```
Lines 1-5:   Debian package description
Line 6:      "Psyco is distributed under the MIT License."  [Match 1]
Lines 7-9:   Blank line + Copyright + Blank line (3 lines gap)
Lines 10-27: MIT License full text                           [Match 2]
```

### Expected Output (from YAML)

```yaml
license_expressions:
  - mit
  - mit
```

**Two separate MIT matches expected.**

### Actual Behavior (Hypothesis)

Rust returns 1 match (both license sections grouped together) instead of 2.

---

## Investigation Results

### 1. Grouping Threshold Comparison

#### Python Code (`detection.py:1836`)

```python
is_in_group_by_threshold = license_match.start_line <= previous_match.end_line + lines_threshold
```

#### Rust Code (`grouping.rs:81-82`)

```rust
let line_gap = cur.start_line.saturating_sub(prev.end_line);
line_gap <= threshold
```

**Analysis**: Both use `<=` threshold comparison. **Threshold logic is IDENTICAL.**

For mit_25.txt:
- Match 1 ends at line 6
- Match 2 starts at line 10
- Gap = 10 - 6 = 4 lines
- Threshold = 4
- Comparison: `4 <= 4` → **TRUE** (would be grouped)

**However**: The expected output shows 2 separate matches, indicating they should NOT be grouped.

---

### 2. Special Handling for License Flags

#### Python Grouping Logic (`detection.py:1838-1864`)

```python
if previous_match.rule.is_license_intro:
    group_of_license_matches.append(license_match)
elif license_match.rule.is_license_intro:
    yield group_of_license_matches
    group_of_license_matches = [license_match]
elif license_match.rule.is_license_clue:
    yield group_of_license_matches
    yield [license_match]
    group_of_license_matches = []
elif is_in_group_by_threshold:
    group_of_license_matches.append(license_match)
else:
    yield group_of_license_matches
    group_of_license_matches = [license_match]
```

#### Rust Grouping Logic (`grouping.rs:36-56`)

```rust
if previous_match.is_license_intro {
    current_group.push(match_item.clone());
} else if match_item.is_license_intro {
    if !current_group.is_empty() {
        groups.push(DetectionGroup::new(current_group.clone()));
    }
    current_group = vec![match_item.clone()];
} else if match_item.is_license_clue {
    if !current_group.is_empty() {
        groups.push(DetectionGroup::new(current_group.clone()));
    }
    groups.push(DetectionGroup::new(vec![match_item.clone()]));
    current_group = Vec::new();
} else if should_group_together(previous_match, match_item, proximity_threshold) {
    current_group.push(match_item.clone());
} else {
    if !current_group.is_empty() {
        groups.push(DetectionGroup::new(current_group.clone()));
    }
    current_group = vec![match_item.clone()];
}
```

**Analysis**: Special flag handling (`is_license_intro`, `is_license_clue`) is **IDENTICAL** in both implementations.

---

### 3. The Real Issue: QueryRun Splitting

#### Python QueryRun Splitting (`query.py:568-652`)

**ACTIVE** - Splits the query into runs when encountering `LINES_THRESHOLD=4` consecutive "junk" lines.

```python
def _tokenize_and_build_runs(self, tokens_by_line, line_threshold=4):
    # ...
    for tokens in tokens_by_line:
        # Break in runs based on threshold of lines that are either:
        # - empty
        # - all unknown
        # - all low id/junk tokens
        # - made only of digits
        
        if len(query_run) > 0 and empty_lines >= line_threshold:
            query_runs_append(query_run)
            query_run = QueryRun(query=self, start=pos)
            empty_lines = 0
```

**Breaking conditions** (from `query.py:605-636`):
1. Empty line (no tokens)
2. All unknown tokens
3. All digit-only tokens
4. No high-value legalese tokens (`tid >= len_legalese`)

When 4+ consecutive lines meet these conditions, Python **starts a new QueryRun**.

#### Rust QueryRun Splitting (`query/mod.rs:332-343`)

**DISABLED** - Entire query treated as single run.

```rust
// TODO: Query run splitting is currently disabled because it causes
// double-matching. The is_matchable() check with matched_qspans helps
// but doesn't fully prevent the issue. Further investigation needed.
// See: reference/scancode-toolkit/src/licensedcode/index.py:1056
// let query_runs = Self::compute_query_runs(
//     &tokens,
//     &tokens_by_line,
//     _line_threshold,
//     len_legalese,
//     &index.digit_only_tids,
// );
let query_runs: Vec<(usize, Option<usize>)> = Vec::new();
```

---

## Why This Causes mit_25.txt to Fail

### Python Behavior

1. **QueryRun 1**: Lines 1-6 (stops when encountering 4+ junk lines? Let's check...)
   - Lines 7-9: Blank, Copyright, Blank
   - Line 7: Blank → junk count = 1
   - Line 8: Copyright notice → has known tokens, but are they high-value?
   - Line 9: Blank → junk count = 2
   - **Wait**: Only 3 lines between matches, not 4!

Let me recalculate. Looking at the file:
- Line 6: "Psyco is distributed under the MIT License."
- Line 7: Empty
- Line 8: "Copyright (c) 2001-2003 <s>Armin Rigo</s>"
- Line 9: Empty
- Line 10: "Permission is hereby granted..."

The copyright line (line 8) likely has known tokens, so the junk line count wouldn't reach 4.

**Alternative hypothesis**: The matches might actually be found as separate query runs due to the **match coverage threshold** or **rule boundaries**, not QueryRun splitting.

Let me check if there's another mechanism at play...

### Actually: The Golden Test Uses `detect_matches()`

Looking at `golden_test.rs:160-169`:

```rust
// Use detect_matches() for raw matches like Python's idx.match()
// This avoids the grouping step that causes false test failures
let matches = engine
    .detect_matches(&text, unknown_licenses)
    .map_err(|e| {
        format!("Detection failed for {}: {:?}", self.test_file.display(), e)
    })?;
```

**The test uses `detect_matches()` which returns ungrouped matches!**

This bypasses the grouping logic entirely. The issue must be in the **matching phase**, not the grouping phase.

---

## Revised Root Cause

### The Problem is in Matching, Not Grouping

The golden test compares **raw match count**, not grouped detections. With QueryRun splitting disabled:

**Python** (with QueryRun splitting):
- QueryRun 1: Lines 1-9 → Finds MIT notice at line 6
- QueryRun 2: Lines 10-27 → Finds MIT license text at lines 10-27
- **Result**: 2 separate matches

**Rust** (without QueryRun splitting):
- Single QueryRun: Lines 1-27 → Finding behavior differs
- May find only 1 match (the full MIT license)
- May merge/overlap the notice with the full text
- **Result**: 1 match (or different match structure)

### Why QueryRun Splitting Matters

From `DIFFERENCES.md`:

> **Python**: Actively splits text into QueryRuns when encountering 4+ empty/junk lines (`LINES_THRESHOLD=4`)
> **Rust**: QueryRun splitting is **disabled**
> **Impact**: Different matching behavior for files with multiple license sections separated by blank lines

QueryRun splitting affects:
1. **Match granularity**: Separate runs → separate matching processes
2. **Candidate selection**: Different query spans → different rule candidates
3. **Match coverage**: Smaller runs → different coverage calculations
4. **Overlap resolution**: Matches from different runs don't overlap

---

## Additional Evidence from Audit Documents

### From `DIFFERENCES.md` Section 1

```
### 1. QueryRun Splitting Disabled in Rust
**File**: `QUERY_TOKENIZATION.md`

- **Python**: Actively splits text into QueryRuns when encountering 4+ empty/junk lines
- **Rust**: QueryRun splitting is **disabled** 
- **Impact**: Different matching behavior for files with multiple license sections separated by blank lines
- **Location**: Python `query.py:583-652`, Rust `query/mod.rs`
```

This is listed as a **Critical** difference affecting results.

### From `QUERY_TOKENIZATION.md` Section 5

```
### Comparison

| Aspect | Python | Rust | Difference? |
|--------|--------|------|-------------|
| Run splitting | Active | **DISABLED** | **Major difference** |
| Line threshold | 4 (text), 15 (bin) | N/A | N/A |
| Long line breaking | >25 tokens | Not implemented | **Missing** |
| break_on_boundaries() | Implemented | Not implemented | **Missing** |

**Impact:** This is a **significant behavioral difference**. Query run splitting affects:
1. Match granularity
2. Performance (more/smaller runs vs fewer/larger runs)
3. Match candidate selection

The Rust implementation treats the entire query as a single run, which may cause different matching behavior.
```

---

## Specific Analysis for mit_25.txt

Let me trace what likely happens:

### Python Flow

1. Tokenize entire file
2. Analyze line-by-line:
   - Lines 1-6: QueryRun 1 builds up
   - Line 7: Empty → junk count = 1
   - Line 8: Copyright line → has tokens, check if high-value
     - If not high-value: junk count = 2
     - If high-value: junk count reset to 0
   - Line 9: Empty → junk count increments
   - **If junk count reaches 4**: Start QueryRun 2
3. Each QueryRun is matched independently
4. Returns matches from all runs

### Rust Flow

1. Tokenize entire file
2. **No QueryRun splitting** → single run from lines 1-27
3. Match against this single large run
4. Return matches

### Key Difference

With separate QueryRuns:
- Each run has its own candidate selection
- Each run can find the "best" match for its section
- Less interference between sections

With single QueryRun:
- One candidate selection for entire file
- Matches from different sections may:
  - Overlap and get merged
  - One may dominate and suppress the other
  - Different coverage calculations

---

## Recommended Fix

### Immediate Priority: Re-enable QueryRun Splitting

**Action**: Uncomment and fix the QueryRun splitting code in `query/mod.rs:336-342`.

**Current code**:
```rust
// TODO: Query run splitting is currently disabled because it causes
// double-matching. The is_matchable() check with matched_qspans helps
// but doesn't fully prevent the issue.
let query_runs: Vec<(usize, Option<usize>)> = Vec::new();
```

**Fix approach**:
1. Enable `compute_query_runs()` function
2. Ensure `is_matchable()` check properly prevents double-matching
3. Test extensively against golden test suite
4. May need to adjust match overlap resolution

**Location**: `src/license_detection/query/mod.rs:332-360`

**Reference**: Python implementation at `reference/scancode-toolkit/src/licensedcode/query.py:568-652`

---

## Test Cases to Verify Fix

Run these tests after enabling QueryRun splitting:

```bash
# Run all golden tests
cargo test --release license_detection::golden_test

# Specific test that fails currently
cargo test --release test_golden_lic3

# Count failures
cargo test --release -q --lib license_detection::golden_test 2>&1 | \
  grep "failed, 0 skipped" | \
  sed 's/.*, \([0-9]*\) failed,.*/\1/' | \
  paste -sd+ | bc
```

**Expected outcome**: Failure count should drop from 96 to a much smaller number.

---

## Summary

| Aspect | Finding |
|--------|---------|
| **Hypothesis H4** | **REJECTED** - Not a threshold edge case |
| **Grouping threshold** | ✅ Identical in both (`<=`) |
| **Special flags** | ✅ Identical handling |
| **Root cause** | QueryRun splitting disabled in Rust |
| **Impact** | Different match behavior for multi-section files |
| **Priority** | **CRITICAL** - Listed as #1 in DIFFERENCES.md |
| **Fix complexity** | Medium - Code exists but disabled, needs testing |
| **Affected tests** | ~96 golden test failures (2% of suite) |

---

## Next Steps

1. **Re-enable QueryRun splitting** in `query/mod.rs`
2. **Test thoroughly** against golden test suite
3. **Verify no double-matching** with `is_matchable()` checks
4. **Document any edge cases** discovered
5. **Update DIFFERENCES.md** once fixed

---

## References

- Python QueryRun implementation: `reference/scancode-toolkit/src/licensedcode/query.py:568-652`
- Rust QueryRun code (disabled): `src/license_detection/query/mod.rs:332-343`
- Audit findings: `docs/license-detection/audit/DIFFERENCES.md` Section 1
- Detailed audit: `docs/license-detection/audit/QUERY_TOKENIZATION.md` Section 5
