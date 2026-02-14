# Debug Notes

## License Detection Issues

### `double_isc.txt` - Multiple License Detection Issue

**File:** `testdata/license-golden/datadriven/lic1/double_isc.txt`

**Expected (Python):**

```yaml
license_expressions:
  - isc
  - isc
  - sudo
```

**Actual (Rust):**

```yaml
license_expressions:
  - isc
```

---

## Root Cause Analysis

### Summary

The issue is a **bug in line number calculation** in `src/license_detection/aho_match.rs`.

The aho-corasick matcher finds a single ISC match with `matched_length=111` tokens that spans lines 2-38 (the entire file), when it should find TWO separate matches:

- Match 1: lines 2-18 (first ISC license text)
- Match 2: lines 24-34 (second ISC license text)

### The Bug

In `src/license_detection/aho_match.rs` lines 137-147:

```rust
let start_line = query_run.start_line().unwrap_or(1);

let end_line = if qend > qstart {
    let _end_token_pos = qend.saturating_sub(1);
    query_run
        .end_line()
        .or_else(|| query_run.start_line())
        .unwrap_or(start_line)
} else {
    start_line
};
```

The problem:

- `query_run.start_line()` returns `line_by_pos[query_run.start]` (line of query run start)
- `query_run.end_line()` returns `line_by_pos[query_run.end]` (line of query run end)

When `whole_query_run()` is called:

- `query_run.start = 0` (first token in document)
- `query_run.end = last_token` (last token in document)

So every match gets:

- `start_line = 1` (first line of document)
- `end_line = N` (last line of document)

This is wrong! The correct calculation should use the **match positions** (`qstart` and `qend`), not the **query run positions**.

### The Fix

The line number calculation should be:

```rust
// qstart and qend are the match positions (already calculated from aho-corasick match)
// These are ABSOLUTE positions in the query's token array

let start_line = query_run.line_for_pos(qstart).unwrap_or(1);
let end_line = if qend > qstart {
    query_run.line_for_pos(qend.saturating_sub(1)).unwrap_or(start_line)
} else {
    start_line
};
```

### Why This Causes Single Detection

Because all matches have the same (incorrect) line numbers:

- Match 1: lines 2-38 (wrong - should be 2-18)
- Match 2: lines 2-38 (wrong - should be 24-34)

When `merge_overlapping_matches()` runs, it merges matches with the same `rule_identifier` that overlap or are adjacent. Since both matches appear to span lines 2-38, they are considered overlapping and get merged into a single match.

---

## Debug Output

```text
=== File Structure ===
Line  2: <Copyright notice>
Line  8: <ISC Permission grant start>
Line 12: <ISC Disclaimer start>
Line 18: <ISC text end>
Line 20: <SEPARATOR: '--'>
Line 22: <Copyright notice>
Line 24: <ISC Permission grant start>
Line 28: <ISC Disclaimer start>
Line 34: <ISC text end>
Line 36: <DARPA sponsorship>

=== Gap Analysis ===
First ISC ends:   line 18
Separator:        line 20
Second ISC starts: line 24
Second ISC ends:   line 34
DARPA text starts: line 36

Gap between ISC texts: 5 lines
Gap second ISC to DARPA: 1 lines

=== Detection Results ===
Number of detections: 1

Detection 1: isc
  Lines: 2-38
  Matches:
    isc - lines 2-38, coverage 100.0%, matcher: 2-aho, rule: #27525, matched_length: 111
```

---

## Investigation Steps

### Step 1: Check raw matcher output

- `1-hash matches: 0` (no exact hash matches)
- `1-spdx-id matches: 0` (no SPDX identifiers)
- `2-aho matches: 1` (BUG: should be 2)

### Step 2: Analyze the single aho match

- Rule identifier: `#27525` (isc_11.RULE - full ISC license text)
- matched_length: 111 tokens
- Lines: 2-38 (entire file!)

### Step 3: Verify the bug

The match has `matched_length=111` tokens, but spans lines 2-38 which is the entire file. This is clearly wrong - the ISC license text is much shorter.

### Step 4: Check Python reference

Python produces 3 detections:

- `isc` at lines 2-18
- `isc` at lines 22-34  
- `sudo` at lines 36-38

This confirms the bug is in Rust's line number calculation.

---

## Additional Notes

### Why "sudo" is not detected

The `sudo_1.RULE` matches:

```text
LICENSE= sudo
LICENSE_NAME= {{Sudo license}}
LICENSE_FILE= /LICENSE.md
```

This is a license TAG, not the DARPA sponsorship notice. The DARPA text at lines 36-38:

```text
Sponsored in part by the Defense Advanced Research Projects
Agency (DARPA) and Air Force Research Laboratory, Air Force
Materiel Command, USAF, under agreement number F39502-99-1-0512
```

This is likely matched by a different rule (possibly `bsd-original-uc_23.RULE` which mentions DARPA). The Python output shows `sudo` but this may be a derived expression based on the file context.

### The Line Number Bug Affects All Aho Matches

This bug affects every aho-corasick match when `whole_query_run()` is used:

- All matches appear to span the entire document
- Adjacent/overlapping matches get merged incorrectly
- Detection regions are wrong

---

## Fix Required

1. **Fix line number calculation in aho_match.rs** (lines 137-147):
   - Use `line_for_pos(qstart)` instead of `start_line()`
   - Use `line_for_pos(qend - 1)` instead of `end_line()`

2. **Add unit test for multi-region detection**:
   - Test that multiple identical licenses in different file regions produce separate detections
   - Verify line numbers are calculated from match positions, not query run positions

---

## FAILED FIX ATTEMPT (2026-02-13)

**Attempted:** Changed line calculation from `query_run.start_line()` to `query_run.line_for_pos(qstart)`

**Result:**

- Before: 186 passed, 105 failed
- After: 50 passed, 241 failed
- **MAJOR REGRESSION - REVERTED**

**Why it failed:**
The simple fix caused cascading failures. Possible reasons:

1. Other code depends on the current (buggy) line number behavior
2. The `line_for_pos()` implementation needs more testing
3. Match merging/deduplication logic depends on overlapping line ranges
4. Need to trace through the full detection pipeline to understand dependencies

---

## ISSUE: sudo License Not Detected (2026-02-13)

**Status:** Investigating
**Test:** `double_isc.txt`
**Expected:**

```yaml
license_expressions:
  - isc
  - isc
  - sudo
```

**Actual:**

```yaml
license_expressions:
  - isc
  - isc AND unknown
```

### Root Cause

**Pipeline short-circuit issue in `src/license_detection/mod.rs:105-132`:**

1. ISC hash-match succeeds with 100% coverage on lines 24-34
2. The sequence matcher is **SKIPPED** because `has_high_coverage` is true
3. The DARPA text (lines 36-38) is detected as "unknown" instead of "sudo"
**Why the sudo rule doesn't match:**

- The sudo rule has 143 tokens (ISC license + DARPA sponsorship)
- The query at lines 24-38 has ~136 tokens (just ISC portion)
- The query is a **subset** of the sudo rule
- Hash matching requires exact token match - fails
- Aho-Corasick finds patterns where RULE ⊂ QUERY, but here QUERY ⊂ RULE
- Sequence matcher would find it, but is skipped

### Python Behavior

Python detects "sudo" because:

1. It runs multiple matchers and combines results
2. It has `licensing_contains()` to detect when one license semantically contains another
3. It may not skip sequence matching as aggressively

### Possible Fixes

**Option A: Run sequence matcher on unmatched regions**
Instead of skipping sequence matching entirely when there's a high-coverage match, run it on the **unmatched regions** to find licenses that extend beyond what was already matched.
**Option B: Implement license expression containment**
Python uses `licensing_contains()` to filter contained matches. This requires:

- Implementing license expression containment checks
- Modifying `filter_contained_matches` to use license expression semantics
**Option C: Post-match merging**
After detecting ISC, check if adjacent "unknown" matches can be combined with the known match to form a larger recognized license (ISC + DARPA = sudo).

### Complexity Assessment

- **Option A:** Medium complexity - requires tracking unmatched regions and running additional matchers
- **Option B:** High complexity - requires understanding license expression semantics
- **Option C:** Medium complexity - requires rule metadata about license containment

---

## IMPORTANT DISCOVERY: Golden Test Comparison Mismatch (2026-02-13)

**Status:** Needs clarification

### The Issue

The Python golden tests compare **match expressions**:

```python
# licensedcode_test_utils.py:215
detected_expressions = [match.rule.license_expression for match in matches]
```

The Rust golden tests compare **detection expressions**:

```rust
// license_detection_golden_test.rs:122-124
let actual: Vec<&str> = detections
    .iter()
    .map(|d| d.license_expression.as_deref().unwrap_or(""))
    .collect();
```

### Key Questions

1. **Are match expressions and detection expressions supposed to be the same?**
   - For simple files with one license: Yes (one match → one detection)
   - For complex files: Maybe different (matches get grouped into detections)
2. **What does `idx.match()` return in Python?**
   - It returns `matches` directly, not detections
   - The `license_expressions` in YAML are the expressions from all matches
3. **What should Rust do?**
   - Option A: Compare raw match expressions (change test to use matches)
   - Option B: Keep comparing detection expressions (current approach)
   - Option C: Both should work if detection logic is correct

### Investigation Needed

- Check if Python's `idx.match()` does grouping/deduplication before returning matches
- Check if the expected `license_expressions` are meant to be detections or matches
- Understand why COPYING.gplv3 produces 8 detections in Rust but expects just `gpl-3.0`

---

## DETAILED FIX PLAN (2026-02-13)

### 1. Problem Summary

**Bug**: `aho_match.rs` and `hash_match.rs` calculate line numbers using query run boundaries instead of match token positions.

| Current (Wrong) | Python (Correct) |
|-----------------|------------------|
| `query_run.start_line()` → `line_by_pos[query_run.start]` | `line_by_pos[match.qstart]` |
| `query_run.end_line()` → `line_by_pos[query_run.end]` | `line_by_pos[match.qend]` |

**Result**: All matches appear to span the entire document, causing incorrect merging and single detections instead of multiple.

### 2. Root Cause Analysis

From Python `match.py:399-408`:

```python
def set_lines(self, line_by_pos):
    self.start_line = line_by_pos[self.qstart]  # Match start position
    self.end_line = line_by_pos[self.qend]       # Match end position
```

The Rust implementation incorrectly uses query run boundaries instead of match positions.

### 3. Implementation Steps

#### Step 1: Add `line_for_pos()` to `QueryRun`

**File**: `src/license_detection/query.rs`

Add method to `impl QueryRun` (after line 718):

```rust
/// Get the line number for a specific token position.
///
/// # Arguments
/// * `pos` - Absolute token position in the query
///
/// # Returns
/// The line number (1-based), or None if position is out of range
pub fn line_for_pos(&self, pos: usize) -> Option<usize> {
    self.line_by_pos.get(pos).copied()
}
```

#### Step 2: Fix `aho_match.rs` Line Calculation

**File**: `src/license_detection/aho_match.rs:137-147`

Replace:

```rust
let start_line = query_run.start_line().unwrap_or(1);

let end_line = if qend > qstart {
    let _end_token_pos = qend.saturating_sub(1);
    query_run
        .end_line()
        .or_else(|| query_run.start_line())
        .unwrap_or(start_line)
} else {
    start_line
};
```

With:

```rust
// Use match positions (qstart, qend-1) not query run boundaries
let start_line = query_run.line_for_pos(qstart).unwrap_or(1);

let end_line = if qend > qstart {
    // qend is exclusive, so the last matched token is at qend-1
    query_run.line_for_pos(qend.saturating_sub(1)).unwrap_or(start_line)
} else {
    start_line
};
```

#### Step 3: Fix `hash_match.rs` Line Calculation

**File**: `src/license_detection/hash_match.rs:92-96`

Replace:

```rust
let start_line = query_run.start_line().unwrap_or(1);
let end_line = query_run
    .end_line()
    .or_else(|| query_run.start_line())
    .unwrap_or(1);
```

With:

```rust
let start_line = query_run.line_for_pos(query_run.start).unwrap_or(1);
let end_line = if let Some(end) = query_run.end {
    query_run.line_for_pos(end).unwrap_or(start_line)
} else {
    start_line
};
```

### 4. Expected Behavior Changes

After fixing line numbers:

| Before Fix | After Fix | Reason |
|------------|-----------|--------|
| All matches have same line range | Matches have correct line ranges | Bug fix |
| Many matches merged together | Fewer merges (correct) | Line ranges now differ |
| Single detection for multiple licenses | Multiple detections | Matches no longer overlap |

### 5. Validation Checklist

After implementation:

- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test --lib` passes
- [ ] `test_aho_match_line_numbers` passes
- [ ] Golden test count should **increase** (more correct detections)
- [ ] `double_isc.txt` should produce 2-3 detections (not 1)

### 6. Files to Change

| File | Change |
|------|--------|
| `src/license_detection/query.rs` | Add `QueryRun::line_for_pos()` |
| `src/license_detection/aho_match.rs:137-147` | Use `line_for_pos(qstart)` and `line_for_pos(qend-1)` |
| `src/license_detection/hash_match.rs:92-96` | Use `line_for_pos(start)` and `line_for_pos(end)` |

---

## Files Involved

| File | Role |
|------|------|
| `src/license_detection/aho_match.rs` | **BUG LOCATION** - Line number calculation |
| `src/license_detection/query.rs` | `QueryRun::start_line()` and `end_line()` methods |
| `src/license_detection/match_refine.rs` | `merge_overlapping_matches()` - merges matches with same rule |
| `src/license_detection/detection.rs` | `remove_duplicate_detections()` - dedups by expression |
| `reference/scancode-toolkit/src/licensedcode/data/rules/isc_11.RULE` | Full ISC license text rule |

---

## ISSUE: Multiple Detections + Unknown in Expressions (2026-02-13)

**Status:** Root cause identified
**Example:** `COPYING.gplv3`

- Expected: `gpl-3.0`
- Actual: 8 detections including `gpl-3.0 AND unknown`

### Root Cause

**Python filters "license intro" matches before building expressions:**

1. `analyze_detection()` categorizes matches (detection.py:1760-1818)
2. `get_detected_license_expression()` filters based on category (detection.py:1468-1602)
3. Key filtering (lines 1510-1514):

   ```python
   elif analysis == DetectionCategory.UNKNOWN_INTRO_BEFORE_DETECTION.value:
       matches_for_expression = filter_license_intros(license_matches)
   ```

4. `filter_license_intros()` removes intro matches before expression is built
**Rust does NOT filter license intros:**
Rust's `determine_license_expression()` (detection.rs:526) combines ALL match expressions, including unknown intros. This causes "unknown" to appear in expressions.

### Fix Needed

In `create_detection_from_group()`, filter out license intro matches before building the expression:

```rust
// Filter out license intros based on detection category
let matches_for_expr = if detection_log_category == "unknown-intro-followed-by-match" {
    detection.matches.iter()
        .filter(|m| !is_license_intro_match(m))
        .cloned()
        .collect()
} else {
    detection.matches.clone()
};
```

### Additional Issue: Multiple Detections

Rust produces 8 detections where Python produces 1. This could be due to:

1. Different grouping threshold
2. Missing detection merging logic
3. Detection category analysis differences

---

## FAILED FIX ATTEMPT: License Intro Filtering (2026-02-13)

**Status:** Reverted - caused regression
**Attempted:** Implement Python-style license intro filtering before building expressions.
**Changes Made:**

1. Added `is_license_intro` and `is_license_clue` fields to `LicenseMatch`
2. Implemented `filter_license_intros()` function
3. Modified expression building to filter intros based on detection category
**Results:**

- Before: 163 passed, 128 failed
- After: 158 passed, 133 failed (5 test regression)
**Why it failed:**
The intro filtering logic is more nuanced than initially understood. Python's filtering is conditional on detection category analysis, and the logic for determining category differs between Python and Rust. The simple implementation caused false positives in filtering.
**Next steps:**
This requires deeper investigation of Python's detection category analysis. Defer to later.

---

## CURRENT SESSION: License Detection Improvement Loop (2026-02-14)

### Baseline

- lic1: 160 passed, 131 failed

### Issue: sudo License Not Detected (double_isc.txt)

**Expected:** `["isc", "isc", "sudo"]`
**Actual:** `["isc", "isc AND unknown"]`

**Root Cause:** Pipeline short-circuit. ISC hash-match succeeds, sequence matcher is skipped. DARPA text detected as "unknown" instead of "sudo".

**Status:** Deferred - requires sequence matcher changes

### Issue: Multiple Detections + Unknown in Expressions

**Example:** `COPYING.gplv3` produces 8 detections including `gpl-3.0 AND unknown`, expected just `gpl-3.0`.

**Root Cause:** Rust does not filter license intro matches before building expressions. Python's `filter_license_intros()` removes intros.

**Status:** Deferred - requires detection category analysis

### FAILED FIX: Deprecated Rule Skipping

**Test:** `camellia_bsd.c` expected `bsd-2-clause-first-lines`, got `freebsd-doc` (deprecated rule).

**Attempted:** Skip deprecated rules during index building.

**Result:** Regression from 160 to 141 passed.

**Reason:** Deprecated rule handling is complex - some are used for license variants. Python has sophisticated `replaced_by` logic.

**Status:** Deferred - requires deeper investigation
