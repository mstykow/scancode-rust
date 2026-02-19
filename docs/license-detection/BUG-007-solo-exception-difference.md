# BUG-007: Solo Exception Behavioral Difference from Python

## Status: Investigation Complete - Decision Needed

---

## 1. Problem Statement

After implementing Bug 6 (merge positions), golden tests still show regression:

- lic1: 228/63 → 213/78 (-15 tests)
- lic2: 776/77 → 717/136 (-59 tests)
- lic3: 251/41 → 236/56 (-15 tests)

---

## 2. Root Cause: Python Solo Exception Bug

### Python's Bug (match.py:2172-2175)

```python
# never discard a solo match, unless matched to "is_continuous" or "is_required_phrase" rule
if len(matches) == 1:
    rule = matches[0]  # BUG: This is a LicenseMatch, not a Rule!
    if not (rule.is_continuous or rule.is_required_phrase):
        return matches, []
```

**The Bug**:

1. `rule = matches[0]` assigns a `LicenseMatch` object, not a `Rule`
2. `rule.is_continuous` accesses a **method** (not called), which is always truthy
3. `rule.is_required_phrase` doesn't exist on `LicenseMatch` (AttributeError would occur, but Python's `or` short-circuits after the truthy `is_continuous`)
4. The condition `not (True or ...)` = `not True` = `False`
5. **The solo exception NEVER triggers in Python**

### Rust's Implementation (match_refine.rs:851-858)

```rust
// Solo match exception: never discard a solo match unless is_continuous or is_required_phrase
if matches.len() == 1
    && let Some(rid) = parse_rule_id(&matches[0].rule_identifier)
    && let Some(rule) = index.rules_by_rid.get(rid)  // Correctly gets the Rule
    && !(rule.is_continuous || rule.is_required_phrase)  // Correctly checks boolean fields
{
    return (matches.to_vec(), Vec::new());
}
```

**Rust is CORRECT** - it properly accesses the Rule and checks boolean attributes.

---

## 3. Behavioral Difference

| Scenario | Python (Buggy) | Rust (Correct) |
|----------|----------------|----------------|
| Solo match to rule without `is_continuous`/`is_required_phrase` | Filter runs, match may be discarded | Exception triggers, match kept immediately |

**Impact**:

- **3317 rules** have `is_continuous: yes`
- **1927 rules** have `is_required_phrase: yes`
- For rules WITHOUT these flags, Rust keeps solo matches immediately while Python runs the full filter

---

## 4. Why This Causes Regression

Rust's correct solo exception makes it **MORE permissive** than Python:

1. Solo matches to non-continuous/non-required-phrase rules are kept immediately
2. These matches bypass the unknown/stopword checks that Python would run
3. Matches that Python would discard (due to unknowns/stopwords) are kept in Rust
4. This causes false positives - more matches than expected

---

## 5. Options

### Option A: Match Python's Buggy Behavior (Bug-for-Bug Compatibility)

Remove the solo exception entirely since Python's never executes.

**Pros**: Exact parity with Python
**Cons**: Replicates a known bug, less correct behavior

### Option B: Keep Rust's Correct Behavior

Accept the difference as an improvement over Python.

**Pros**: More correct behavior
**Cons**: Different results from Python, may affect compatibility

### Option C: Remove Solo Exception AND Fix Filter Logic

If the solo exception is masking other issues, remove it and ensure the filter correctly handles all cases.

**Pros**: Clean implementation
**Cons**: May expose other differences

---

## 6. Recommended Fix

**Option A**: Remove the solo exception to match Python's behavior.

The solo exception was intended to protect solo matches, but since Python's implementation never triggers it, removing it will:

1. Achieve parity with Python
2. Not change behavior for matches to `is_continuous` or `is_required_phrase` rules (they still go through the filter)
3. Only affect solo matches to rules WITHOUT these flags - they will now go through the filter like Python

### Code Change

**File**: `src/license_detection/match_refine.rs` (~lines 851-858)

**Remove**:

```rust
// Solo match exception: never discard a solo match unless is_continuous or is_required_phrase
if matches.len() == 1
    && let Some(rid) = parse_rule_id(&matches[0].rule_identifier)
    && let Some(rule) = index.rules_by_rid.get(rid)
    && !(rule.is_continuous || rule.is_required_phrase)
{
    return (matches.to_vec(), Vec::new());
}
```

This should make Rust's behavior match Python's exactly.

---

## 6. Implemented Fix

**Status**: ✅ DONE

Removed the solo exception to match Python's buggy behavior. The filter now processes all matches the same way Python does.

**Code removed** (match_refine.rs):

```rust
// Solo match exception - REMOVED
```

**Result**: No change in golden test results. The solo exception was not the cause of the regression.

---

## 7. Remaining Investigation

The regression persists after removing the solo exception. Need to investigate:

1. `is_continuous()` method implementation
2. Unknown/stopword check logic
3. Other filter conditions
