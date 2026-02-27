# PLAN-003: here-proprietary_4.RULE

## Status: ROOT_CAUSE_IDENTIFIED

## Test File
`testdata/license-golden/datadriven/lic4/here-proprietary_4.RULE`

## Issue
Duplicate detection of `here-proprietary`.

**Expected:** `["here-proprietary"]`

**Actual:** `["here-proprietary", "here-proprietary"]`

## Root Cause Analysis

### The Bug

SPDX-LID matching sets `end_token` to the **last position** (inclusive), but Aho-Corasick sets `end_token` as **exclusive** (one past the last position). This inconsistency prevents proper containment detection in `filter_contained_matches`.

### Detailed Analysis

Two matches are being created for the same license expression:

1. **SPDX-LID match** via `1-spdx-id`:
   - Rule: `here-proprietary.LICENSE`
   - `start_token=0, end_token=5`
   - Covers tokens [0, 1, 2, 3, 4] (exclusive end interpretation)
   - Source: `src/license_detection/spdx_lid.rs:280-281`

2. **Aho-Corasick match** via `2-aho`:
   - Rule: `spdx_license_id_licenseref-proprietary-here_for_here-proprietary.RULE`
   - `start_token=3, end_token=6`
   - Covers tokens [3, 4, 5] (exclusive end)
   - Source: `src/license_detection/aho_match.rs:100-101`

**Key Issue:** Token 5 is in Aho's qspan but NOT in SPDX's qspan!

The `qcontains()` check in `filter_contained_matches` returns `false` because token 5 is in Aho's span but not SPDX's span. Therefore, the Aho match is NOT filtered out.

### Source of the Bug

In `src/license_detection/query.rs:416-417`:
```rust
spdx_end = line_last_known_pos as usize;
```

This sets `spdx_end` to the last known position on the line, which is then used as `end_token` in the SPDX match. But `end_token` should be **exclusive** (one past the last position), not **inclusive**.

### Query Token Analysis

For input `SPDX-License-Identifier: LicenseRef-Proprietary-HERE\n`:
- Query has 6 tokens: `[3678, 2432, 1992, 2436, 3129, 5391]`
- SPDX match: tokens [0, 1, 2, 3, 4] (missing token 5!)
- Aho match: tokens [3, 4, 5] (includes token 5)

The SPDX match should cover tokens [0, 1, 2, 3, 4, 5], meaning `end_token=6`.

## Fix

In `src/license_detection/query.rs:416-417`, change:
```rust
spdx_end = line_last_known_pos as usize;
```
to:
```rust
spdx_end = (line_last_known_pos + 1) as usize;
```

This makes `end_token` exclusive, consistent with Aho-Corasick matches.

## Investigation Files

- `src/license_detection/investigation/here_proprietary_test.rs` - Tests tracing each pipeline stage
- `src/license_detection/query.rs:412-417` - Where `spdx_lines` is populated
- `src/license_detection/spdx_lid.rs:280-281` - Where SPDX match `end_token` is set
- `src/license_detection/aho_match.rs:100-101` - Where Aho match `end_token` is set (exclusive)
- `src/license_detection/match_refine.rs:363-419` - `filter_contained_matches` function

## Next Steps

1. Implement fix in `query.rs:416-417`
2. Run investigation tests to verify fix
3. Run full golden test suite
4. Update status to FIXED
