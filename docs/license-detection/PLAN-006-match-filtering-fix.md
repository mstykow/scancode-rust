# PLAN-006: Match Filtering Logic Fix in find_longest_match

## Problem Statement

After PLAN-005 implementation, there's a 4-test regression. The `find_longest_match` function is too strict in filtering tokens.

## Root Cause Analysis

### The Critical Difference: `matchables` Set Contents

**The bug is in how `matchables` is populated and what it represents.**

#### Python's `matchables` Property

**File:** `reference/scancode-toolkit/src/licensedcode/query.py:820-825`

```python
@property
def matchables(self):
    """
    Return a set of every matchable token ids positions for this query.
    """
    return self.low_matchables | self.high_matchables
```

The Python `matchables` set contains **ALL** matchable positions (both high AND low):

- `low_matchables`: Positions of tokens with IDs >= `len_legalese` (non-legalese tokens)
- `high_matchables`: Positions of tokens with IDs < `len_legalese` (legalese tokens)

#### Python's `find_longest_match` Filtering Logic

**File:** `reference/scancode-toolkit/src/licensedcode/seq.py:66`

```python
cura = a[i]
if cura < len_good and i in matchables:
    # ... process token
```

Python checks TWO independent conditions:

1. `cura < len_good` - Token is "legalese" (high-value token)
2. `i in matchables` - Position is in the matchable set

**But `matchables` includes BOTH high AND low token positions!**

This means:

- If `cura < len_good` is TRUE and position is in `matchables`, the token is processed
- If `cura >= len_good` is FALSE, the condition short-circuits and position is NOT checked

#### Rust's Current Filtering Logic

**File:** `src/license_detection/seq_match.rs:270-271`

```rust
if (cur_a as usize) < len_legalese
    && matchables.contains(&i)
```

Rust checks the SAME two conditions as Python.

### The Real Issue: `matchables` Parameter in `seq_match`

**File:** `src/license_detection/seq_match.rs:445`

```rust
let matchables = query_run.matchables(false);
```

When `include_low=false`, Rust's `matchables()` returns **ONLY** `high_matchables`.

**But Python passes the FULL matchables set (both low and high)!**

**File:** `reference/scancode-toolkit/src/licensedcode/match_seq.py:100-103`

```python
block_matches = match_blocks(
    a=qtokens, b=itokens, a_start=qstart, a_end=qfinish + 1,
    b2j=high_postings, len_good=len_legalese,
    matchables=query_run.matchables)  # <-- FULL matchables (high | low)
```

### Impact Analysis

| Scenario | Python Behavior | Rust Behavior (Current) | Correct? |
|----------|-----------------|-------------------------|----------|
| High token at matchable position | Process | Process | Yes |
| High token at non-matchable position | Skip | Skip | Yes |
| Low token at any position | Skip (short-circuit) | Skip | Yes |

Wait, this looks correct... Let me re-analyze.

### Deeper Analysis: What Does `matchables` Actually Mean?

Looking at Python's `match_blocks` call more carefully:

```python
matchables=query_run.matchables
```

And Python's `matchables` property:

```python
return self.low_matchables | self.high_matchables
```

The `matchables` set represents **positions that have not been matched yet**. It's used to:

1. Prevent re-matching already matched positions
2. Allow extending matches into low-token areas

### The `extend_match` Function is Also Affected

**File:** `reference/scancode-toolkit/src/licensedcode/seq.py:84-104`

```python
def extend_match(besti, bestj, bestsize, a, b, alo, ahi, blo, bhi, matchables):
    if bestsize:
        while (besti > alo and bestj > blo
               and a[besti - 1] == b[bestj - 1]
               and (besti - 1) in matchables):  # <-- Only checks position, NOT token type!
            besti -= 1
            bestj -= 1
            bestsize += 1

        while (besti + bestsize < ahi and bestj + bestsize < bhi
               and a[besti + bestsize] == b[bestj + bestsize]
               and (besti + bestsize) in matchables):  # <-- Only checks position!
            bestsize += 1

    return Match(besti, bestj, bestsize)
```

**CRITICAL: `extend_match` only checks position in `matchables`, NOT token type!**

This allows the match to extend into low-token areas (non-legalese).

### Current Rust `extend_match` Logic

**File:** `src/license_detection/seq_match.rs:300-317`

```rust
if best_size > 0 {
    while best_i > query_lo
        && best_j > rule_lo
        && query_tokens[best_i - 1] == rule_tokens[best_j - 1]
        && matchables.contains(&(best_i - 1))  // Correct - only position check
    {
        best_i -= 1;
        best_j -= 1;
        best_size += 1;
    }

    while best_i + best_size < query_hi
        && best_j + best_size < rule_hi
        && query_tokens[best_i + best_size] == rule_tokens[best_j + best_size]
        && matchables.contains(&(best_i + best_size))  // Correct - only position check
    {
        best_size += 1;
    }
}
```

The extend logic is correct!

### Final Root Cause: `matchables` Parameter Value

The bug is in `seq_match.rs:445`:

```rust
let matchables = query_run.matchables(false);  // Returns ONLY high_matchables
```

Should be:

```rust
let matchables = query_run.matchables(true);  // Returns high_matchables | low_matchables
```

But wait, let me check what Python passes again...

**File:** `reference/scancode-toolkit/src/licensedcode/match_seq.py:100-103`

```python
matchables=query_run.matchables
```

And `query_run.matchables` is:

```python
@property
def matchables(self):
    return self.low_matchables | self.high_matchables
```

So Python passes **BOTH** high and low matchables, but Rust passes **ONLY** high matchables!

### Why This Matters for `find_longest_match`

In `find_longest_match`:

```python
if cura < len_good and i in matchables:
```

The `i in matchables` check means "has this position not been matched yet?"

If we only pass high_matchables, then low token positions will fail the `i in matchables` check even though they haven't been matched!

But wait... if `cura < len_good` is false (low token), Python short-circuits and doesn't even check `i in matchables`. So low tokens are always skipped in the main loop anyway.

Let me re-examine the extend_match behavior:

The `extend_match` can extend the match into low-token areas because it only checks `position in matchables`, not `token < len_good`.

**If `matchables` only contains high positions, extend_match cannot extend into low-token areas!**

This is the bug!

### Example Scenario

Query: `[legalese, legalese, low, low]` (positions 0, 1, 2, 3)
Rule: `[legalese, legalese, low, low]`

- `high_matchables` = {0, 1}
- `low_matchables` = {2, 3}
- `matchables` (Python) = {0, 1, 2, 3}
- `matchables` (Rust, current) = {0, 1}

When `find_longest_match` finds a match at positions 0, 1:

**Python:**

- `extend_match` can extend to positions 2, 3 because they're in `matchables`
- Final match: positions 0-3

**Rust (current):**

- `extend_match` cannot extend to positions 2, 3 because they're NOT in `matchables`
- Final match: positions 0-1 only

This causes shorter matches and potential missed detections!

---

## Implementation Plan

### Step 1: Fix `matchables` Parameter in `seq_match`

**Goal:** Pass the full matchables set (high + low) to `match_blocks`.

**File:** `src/license_detection/seq_match.rs:445`

**Change:**

```rust
// Before:
let matchables = query_run.matchables(false);

// After:
let matchables = query_run.matchables(true);
```

**Risk:** Low. This aligns Rust with Python behavior.

### Step 2: Verify `find_longest_match` Logic

**Goal:** Ensure `find_longest_match` filtering is correct.

The current logic at lines 270-271:

```rust
if (cur_a as usize) < len_legalese
    && matchables.contains(&i)
```

This is correct:

- First check: Token is legalese (high-value)
- Second check: Position hasn't been matched

The second check is redundant for the initial match (all positions start as matchable), but becomes important after matches are subtracted.

### Step 3: Verify `extend_match` Logic

**Goal:** Ensure `extend_match` only checks position, not token type.

The current logic at lines 300-317 is correct - it only checks `matchables.contains()` without checking token type.

### Step 4: Add Unit Tests

**Test 1: `test_extend_match_into_low_tokens`**

Verify that matches can extend into low-token areas when those positions are in `matchables`.

```rust
#[test]
fn test_extend_match_into_low_tokens() {
    // Query: [0, 1, 99, 98] where 0,1 are legalese, 99,98 are low
    // Rule:  [0, 1, 99, 98]
    // Expected: Full match of length 4 when matchables includes all positions
    let query_tokens = vec![0, 1, 99, 98];
    let rule_tokens = vec![0, 1, 99, 98];
    let mut high_postings: HashMap<u16, Vec<usize>> = HashMap::new();
    high_postings.insert(0, vec![0]);
    high_postings.insert(1, vec![1]);

    // CRITICAL: matchables must include ALL positions (high AND low)
    let matchables: HashSet<usize> = (0..query_tokens.len()).collect();

    let result = find_longest_match(
        &query_tokens, &rule_tokens, 0, 4, 0, 4,
        &high_postings, 5, &matchables,
    );

    // Should find full match because extend_match can extend into low tokens
    assert_eq!(result, (0, 0, 4), "Should extend into low-token positions");
}
```

**Test 2: `test_extend_match_blocked_by_non_matchable`**

Verify that `extend_match` respects non-matchable positions.

```rust
#[test]
fn test_extend_match_blocked_by_non_matchable() {
    // Query: [0, 1, 99, 98] where position 2 is NOT matchable
    let query_tokens = vec![0, 1, 99, 98];
    let rule_tokens = vec![0, 1, 99, 98];
    let mut high_postings: HashMap<u16, Vec<usize>> = HashMap::new();
    high_postings.insert(0, vec![0]);
    high_postings.insert(1, vec![1]);

    // Position 2 is NOT in matchables
    let matchables: HashSet<usize> = [0, 1, 3].into_iter().collect();

    let result = find_longest_match(
        &query_tokens, &rule_tokens, 0, 4, 0, 4,
        &high_postings, 5, &matchables,
    );

    // Should NOT extend past position 2
    assert_eq!(result.2, 2, "Should stop at non-matchable position");
}
```

**Test 3: `test_matchables_includes_low_tokens`**

Verify that `matchables(true)` returns both high and low positions.

```rust
#[test]
fn test_matchables_includes_low_tokens() {
    // This test should be in query.rs tests
    // Verify that QueryRun::matchables(true) includes both high and low
}
```

### Step 5: Run Golden Tests

After the fix, run golden tests to verify the regression is fixed:

```bash
cargo test --release license_detection::golden_test
```

Expected result: 4 fewer failures (the regression tests should pass).

---

## Summary of Changes

| File | Line | Change |
|------|------|--------|
| `src/license_detection/seq_match.rs` | 445 | Change `matchables(false)` to `matchables(true)` |

---

## Detailed Comparison: Python vs Rust

### Python `find_longest_match` (seq.py:62-78)

```python
for i in range(alo, ahi):
    newj2len = {}
    cura = a[i]
    if cura < len_good and i in matchables:  # Line 66
        for j in b2j_get(cura, nothing):
            # ... LCS logic
```

### Rust `find_longest_match` (seq_match.rs:266-297)

```rust
for i in query_lo..query_hi {
    let mut new_j2len: HashMap<usize, usize> = HashMap::new();
    let cur_a = query_tokens[i];

    if (cur_a as usize) < len_legalese
        && matchables.contains(&i)
        && let Some(positions) = high_postings.get(&cur_a)
    {
        for &j in positions {
            // ... LCS logic
        }
    }
    j2len = new_j2len;
}
```

**Difference:** Rust also checks `high_postings.get(&cur_a)`. This is an optimization to avoid iterating over tokens not in the rule. Python handles this by having `b2j` only contain high tokens.

This is correct - the `high_postings` only contains high tokens, so if `cur_a < len_legalese` but `cur_a` is not in `high_postings`, we skip.

### Python `extend_match` (seq.py:84-104)

```python
def extend_match(besti, bestj, bestsize, a, b, alo, ahi, blo, bhi, matchables):
    if bestsize:
        while (besti > alo and bestj > blo
               and a[besti - 1] == b[bestj - 1]
               and (besti - 1) in matchables):  # No token type check!
            besti -= 1
            bestj -= 1
            bestsize += 1

        while (besti + bestsize < ahi and bestj + bestsize < bhi
               and a[besti + bestsize] == b[bestj + bestsize]
               and (besti + bestsize) in matchables):  # No token type check!
            bestsize += 1
```

### Rust `extend_match` (seq_match.rs:300-317)

```rust
if best_size > 0 {
    while best_i > query_lo
        && best_j > rule_lo
        && query_tokens[best_i - 1] == rule_tokens[best_j - 1]
        && matchables.contains(&(best_i - 1))  // Correct - no token type check
    {
        best_i -= 1;
        best_j -= 1;
        best_size += 1;
    }

    while best_i + best_size < query_hi
        && best_j + best_size < rule_hi
        && query_tokens[best_i + best_size] == rule_tokens[best_j + best_size]
        && matchables.contains(&(best_i + best_size))  // Correct
    {
        best_size += 1;
    }
}
```

**Status:** Rust implementation matches Python.

---

## Expected Impact on Golden Tests

After fixing the `matchables` parameter:

1. **Longer matches:** Matches can extend into low-token areas
2. **Better coverage:** More tokens matched means higher coverage scores
3. **Fewer missed detections:** Rules with mixed high/low tokens will be detected

---

## References

- Python `seq.py`: `reference/scancode-toolkit/src/licensedcode/seq.py`
- Python `match_seq.py`: `reference/scancode-toolkit/src/licensedcode/match_seq.py`
- Python `query.py`: `reference/scancode-toolkit/src/licensedcode/query.py`
- Rust implementation: `src/license_detection/seq_match.rs`
- Rust query: `src/license_detection/query.rs`
