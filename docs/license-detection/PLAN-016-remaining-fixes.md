# PLAN-016: Remaining License Detection Fixes

## Status: Ready for Implementation

---

## Current State

| Metric | Value |
|--------|-------|
| lic1 passed | 188 |
| lic1 failed | 103 |

---

## Priority 1: Query Run Matching with Matched Position Tracking

### Problem

Query run splitting is disabled because it causes double-matching. Python tracks matched positions across all phases and passes them to `is_matchable()`. Rust doesn't.

### Python's Approach

From `reference/scancode-toolkit/src/licensedcode/index.py:739-771`:

```python
already_matched_qspans = []

# After near-duplicate matching
for match in matched:
    qspan = match.qspan
    query.subtract(qspan)
    already_matched_qspans.append(qspan)

# In query run matching
for query_run in query.query_runs:
    if not query_run.is_matchable(include_low=False, qspans=already_matched_qspans):
        continue
    # ... matching logic
```

### Implementation Steps

1. **Track matched qspans in `detect()` pipeline**:
   - Add `matched_qspans: Vec<PositionSpan>` variable
   - After each match, append the match's qspan to this list

2. **Pass matched_qspans to `is_matchable()` in Phase 4**:
   - Modify the query run matching phase to pass exclude positions
   - Skip query runs that have no matchable positions

3. **Re-enable `compute_query_runs()`**:
   - Uncomment the call in `Query::with_options()`
   - Verify query runs are created correctly

### Files to Modify

- `src/license_detection/mod.rs` - Add matched_qspans tracking
- `src/license_detection/query.rs` - Ensure `is_matchable()` accepts exclude positions

### Expected Impact

~20 tests where combined rules should match instead of partial rules.

---

## Priority 2: `has_unknown_intro_before_detection()` Post-Loop Logic

### Problem

The `has_unknown_intro_before_detection()` function is missing post-loop logic that Python has.

### Python Reference

From `reference/scancode-toolkit/src/licensedcode/detection.py:1323-1331`:

```python
# After the main loop, if we had unknown intro but no proper detection followed
if has_unknown_intro:
    filtered = filter_license_intros(matches)
    if matches != filtered:
        # Check if filtered matches have insufficient coverage
        # Return true if so (meaning the unknown intro can be discarded)
```

### Implementation Steps

1. Find `has_unknown_intro_before_detection()` in `src/license_detection/detection.rs`
2. Add the post-loop check from Python
3. Add unit tests

### Expected Impact

~10 tests fixed.

---

## Priority 3: Missing Filters

### 3.1 `filter_matches_missing_required_phrases()`

**Python:** `match.py:2154-2316`

Filters matches that don't contain required phrases marked with `{{...}}` in the rule text.

### 3.2 `filter_spurious_matches()`

**Python:** `match.py:1768-1836`

Filters low-density sequence matches (matched tokens are scattered, not contiguous).

### 3.3 `filter_too_short_matches()`

**Python:** `match.py:1706-1737`

Filters matches where `match.is_small()` returns true.

### Implementation Steps

For each filter:

1. Read Python implementation
2. Implement in `src/license_detection/match_refine.rs`
3. Add to the refinement pipeline
4. Add unit tests

### Expected Impact

~15-20 tests fixed.

---

## Implementation Order

1. **Priority 1**: Query run matching (highest impact)
2. **Priority 2**: Post-loop logic
3. **Priority 3**: Missing filters

---

## Verification Commands

```bash
# Run specific tests
cargo test --release --lib license_detection::detection
cargo test --release --lib license_detection::match_refine

# Run golden tests
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic1

# Check code quality
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings
```
