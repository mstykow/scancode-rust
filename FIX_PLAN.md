# Fix Plan: QueryRun Splitting and MAX_DIST

## Current Status
- **Baseline**: 96 failing golden test cases
- **Investigation Results**: All 3 hypotheses investigations identified QueryRun splitting as the critical missing feature

## Root Cause Analysis

### Critical Issue: QueryRun Splitting Disabled
**Location**: `src/license_detection/query/mod.rs:332-343`
**Python Reference**: `reference/scancode-toolkit/src/licensedcode/query.py:568-652`

**Current Rust Code** (DISABLED):
```rust
// TODO: Query run splitting is currently disabled because it causes
// double-matching. The is_matchable() check with matched_qspans helps
// but doesn't fully prevent the issue. Further investigation needed.
let query_runs: Vec<(usize, Option<usize>)> = Vec::new();
```

**Python Behavior**:
- Splits text into multiple QueryRuns when encountering 4+ consecutive "junk" lines
- Junk lines = empty, unknown, or low-value tokens
- Allows finding separate matches in different file sections
- Documented in DIFFERENCES.md as item #1 Critical difference

### Secondary Issue: MAX_DIST Threshold
**Location**: `src/license_detection/match_refine/merge.rs:11`
**Current**: `const MAX_DIST: usize = 100;`
**Should be**: `const MAX_DIST: usize = 50;` (matches Python)

**Impact**: Fixes 3-4 failing tests by itself

## Implementation Plan

### Step 1: Fix MAX_DIST (Easy Win)
**Action**: Change `MAX_DIST` from 100 to 50 in `src/license_detection/match_refine/merge.rs:11`

**Rationale**: 
- Python uses 50, Rust uses 100
- Rust merges more aggressively for gaps 51-100 tokens
- Fixes specific test cases

**Expected Impact**: ~3-4 tests fixed

### Step 2: Enable QueryRun Splitting (Critical)

**IMPORTANT**: The `compute_query_runs()` function **already exists** at `src/license_detection/query/mod.rs:372-431`. The fix requires:
1. Uncommenting the call at lines 336-342
2. Fixing edge cases and double-matching prevention

#### Step 2.1: Uncomment the Call
**Location**: `src/license_detection/query/mod.rs:336-342`

Change from:
```rust
let query_runs: Vec<(usize, Option<usize>)> = Vec::new();
```

To:
```rust
let query_runs = Self::compute_query_runs(
    &tokens,
    &tokens_by_line,
    _line_threshold,
    len_legalese,
    &index.digit_only_tids,
);
```

#### Step 2.2: Fix Missing Edge Cases in `compute_query_runs()`
**Location**: `src/license_detection/query/mod.rs:372-431`
**Python Reference**: `reference/scancode-toolkit/src/licensedcode/query.py:602-641`

**Missing Edge Cases**:

1. **Line 602-604 handling** - Python resets `query_run.start = pos` when `len(query_run) == 0`:
   ```python
   if len(query_run) == 0:
       query_run.start = pos
   ```
   Rust implementation may not handle this correctly.

2. **`line_has_known_tokens` check** - Python has TWO token checks:
   - `line_has_known_tokens` - True if ANY token is not None (different from empty line)
   - `line_has_good_tokens` - True if ANY token < len_legalese
   
   **Python code (lines 609-631)**:
   ```python
   line_has_known_tokens = False
   line_has_good_tokens = False
   line_is_all_digit = all([
       tid is None or tid in digit_only_tids for tid in tokens
   ])

   for token_id in tokens:
       if token_id is not None:
           tokens_append(token_id)
           line_has_known_tokens = True
           if token_id < len_legalese:
               line_has_good_tokens = True
           query_run.end = pos
           pos += 1

   if line_is_all_digit:
       empty_lines += 1
       continue

   if not line_has_known_tokens:  # <-- THIS CHECK IS MISSING IN RUST
       empty_lines += 1
       continue
   ```

   **Rust code (lines 401-418)** - Missing `line_has_known_tokens`:
   ```rust
   let line_is_all_digit = line_tokens.iter().all(|tid| digit_only_tids.contains(tid));
   let line_has_good_tokens = line_tokens.iter().any(|tid| (*tid as usize) < len_legalese);

   for _tid in line_tokens {
       run_end = Some(pos);
       pos += 1;
   }

   if line_is_all_digit {
       empty_lines += 1;
       continue;
   }

   if line_has_good_tokens {
       empty_lines = 0;
   } else {
       empty_lines += 1;
   }
   ```

   **Fix Required**: Add `line_has_known_tokens` check before `line_has_good_tokens` logic.

### Step 2.5: Investigate `query.subtract()` Requirement (NEW)

**Critical Investigation Needed**: Python uses `query.subtract(qspan)` to modify the query's matchable positions, but Rust only tracks `matched_qspans` for `is_matchable()` checks.

**Python Implementation** (`reference/scancode-toolkit/src/licensedcode/index.py:767-771`):
```python
for match in matched:
    qspan = match.qspan
    query.subtract(qspan)           # <-- MODIFIES query.high_matchables and low_matchables
    already_matched_qspans.append(qspan)
```

**Python `query.subtract()`** (`reference/scancode-toolkit/src/licensedcode/query.py:328-334`):
```python
def subtract(self, qspan):
    """
    Subtract the qspan matched positions from the query matchable positions.
    """
    if qspan:
        self.high_matchables.difference_update(qspan)
        self.low_matchables.difference_update(qspan)
```

**Rust Implementation** (`src/license_detection/mod.rs:225-227, 286`):
```rust
// Python: if not whole_query_run.is_matchable(include_low=False, qspans=already_matched_qspans)
let matched_qspans: Vec<PositionSpan> = Vec::new();
let skip_seq_matching = !whole_run.is_matchable(false, &matched_qspans);
// ...
if !query_run.is_matchable(false, &matched_qspans) {
```

**Key Question**: Is `is_matchable()` with `matched_qspans` sufficient, or do we need to actually modify `query.high_matchables` and `query.low_matchables` like Python?

**Investigation Tasks**:
1. Check if Python's `is_matchable()` uses `matched_qspans` parameter OR the modified `query.high_matchables/low_matchables`
2. If Python uses modified sets, determine if Rust's `is_matchable()` with exclusion list is equivalent
3. Look for cases where double-matching occurs with current Rust implementation

**Reference Files**:
- `reference/scancode-toolkit/src/licensedcode/query.py:798-818` - Python `is_matchable()` method
- `src/license_detection/query/mod.rs:826` - Rust `is_matchable()` method

### Step 3: Testing Strategy

#### Specific Test Cases to Verify
Run these golden tests after each change:

1. **GPL + MPL combined test** - Verifies double-matching prevention:
   ```bash
   cargo test --release -q --lib gpl_mpl
   ```

2. **Multiple license detection** - Files with 4+ empty lines between licenses:
   ```bash
   cargo test --release -q --lib license_detection::golden_test
   ```

3. **Edge cases** - Files with only digits, only unknowns:
   - Check that `line_has_known_tokens` properly increments `empty_lines`

#### Verification Steps
1. Run golden tests after each change:
   ```bash
   cargo test --release -q --lib license_detection::golden_test 2>&1 | grep "failed, 0 skipped" | sed 's/.*, \([0-9]*\) failed,.*/\1/' | paste -sd+ | bc
   ```

2. Compare specific failing test outputs with Python reference:
   ```bash
   cd reference/scancode-playground && venv/bin/python src/scancode/cli.py --license testfile.txt
   ```

3. Verify no regressions in passing tests (4703 - 96 = 4607 tests should still pass)

### Step 4: Validation
1. Check that implementation matches Python behavior
2. Verify no double-matching occurs (compare output matches with Python)
3. Run full test suite
4. Document any remaining differences in DIFFERENCES.md

## Files to Modify
1. `src/license_detection/match_refine/merge.rs` - MAX_DIST change (line 11)
2. `src/license_detection/query/mod.rs` - Uncomment call (lines 336-342), fix edge cases (lines 372-431)
3. `src/license_detection/mod.rs` - Potentially add `query.subtract()` if investigation shows it's needed

## References
- `docs/license-detection/audit/DIFFERENCES.md` - Known differences documentation
- `reference/scancode-toolkit/src/licensedcode/query.py:568-652` - Python `_tokenize_and_build_runs()`
- `reference/scancode-toolkit/src/licensedcode/query.py:328-334` - Python `query.subtract()`
- `reference/scancode-toolkit/src/licensedcode/index.py:767-771` - Python usage of `query.subtract()`
- `reference/scancode-toolkit/src/licensedcode/__init__.py:13` - MAX_DIST definition

## Success Criteria
- Golden test failures reduced from 96 to < 50
- No regressions in passing tests
- Implementation matches Python behavior
- No double-matching in multi-license files
- Code is well-documented and tested
