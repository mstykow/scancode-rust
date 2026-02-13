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

**Next step:**
This is a complex fix that requires deeper investigation. The bug is documented but not trivial to fix without breaking other functionality.

---

## Files Involved

| File | Role |
|------|------|
| `src/license_detection/aho_match.rs` | **BUG LOCATION** - Line number calculation |
| `src/license_detection/query.rs` | `QueryRun::start_line()` and `end_line()` methods |
| `src/license_detection/match_refine.rs` | `merge_overlapping_matches()` - merges matches with same rule |
| `src/license_detection/detection.rs` | `remove_duplicate_detections()` - dedups by expression |
| `reference/scancode-toolkit/src/licensedcode/data/rules/isc_11.RULE` | Full ISC license text rule |
