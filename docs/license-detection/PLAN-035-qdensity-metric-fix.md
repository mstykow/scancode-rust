# PLAN-035: Fix `qdensity()` Metric to Include Unknown Tokens

**Status**: Proposed  
**Severity**: HIGH  
**Impact**: Spurious match filtering produces different results than Python reference  
**Created**: 2026-02-23  

---

## 1. Problem Description

The Rust implementation of `qdensity()` uses `qregion_len()` as the denominator, while Python's `qdensity()` uses `qmagnitude()`. The key difference is that `qmagnitude()` includes unknown tokens in the calculation, while `qregion_len()` does not.

This discrepancy affects the spurious match filtering thresholds, causing Rust to potentially keep or discard different matches than Python would.

---

## 2. Current State Analysis

### 2.1 Rust Implementation

**Location**: `src/license_detection/models.rs:426-436`

```rust
#[allow(dead_code)]
pub fn qdensity(&self) -> f32 {
    let mlen = self.len();
    if mlen == 0 {
        return 0.0;
    }
    let qregion = self.qregion_len();  // <-- WRONG: should use qmagnitude
    if qregion == 0 {
        return 0.0;
    }
    mlen as f32 / qregion as f32
}
```

**Issue**: Uses `qregion_len()` which only counts the span from min to max matched token position. Does NOT account for unknown tokens in the matched range.

### 2.2 Rust `qregion_len()` Implementation

**Location**: `src/license_detection/models.rs:402-413`

```rust
fn qregion_len(&self) -> usize {
    if let Some(positions) = &self.matched_token_positions {
        if positions.is_empty() {
            return 0;
        }
        let min_pos = *positions.iter().min().unwrap_or(&0);
        let max_pos = *positions.iter().max().unwrap_or(&0);
        max_pos - min_pos + 1
    } else {
        self.end_token.saturating_sub(self.start_token)
    }
}
```

### 2.3 Rust `qmagnitude()` Implementation (Already Correct!)

**Location**: `src/license_detection/models.rs:417-423`

```rust
/// Return the query magnitude: qregion_len + unknowns in matched range.
/// Python: qmagnitude = qregion_len + sum(unknowns_by_pos for pos in qspan[:-1])
pub fn qmagnitude(&self, query: &crate::license_detection::query::Query) -> usize {
    let qregion_len = self.qregion_len();
    let unknowns_in_match = (self.start_token..self.end_token)
        .filter(|&pos| query.unknowns_by_pos.contains_key(&Some(pos as i32)))
        .count();
    qregion_len + unknowns_in_match
}
```

**Key observation**: Rust already has a correct `qmagnitude()` implementation! The issue is that `qdensity()` doesn't use it.

---

## 3. Python Reference Analysis

### 3.1 Python `qdensity()` Implementation

**Location**: `reference/scancode-toolkit/src/licensedcode/match.py:565-579`

```python
def qdensity(self):
    """
    Return the query density of this match as a ratio of its length to its
    qmagnitude, a float between 0 and 1. A dense match has all its matched
    query tokens contiguous and a maximum qdensity of one. A sparse low
    qdensity match has some non-contiguous matched query tokens interspersed
    between matched query tokens. An empty match has a zero qdensity.
    """
    mlen = self.len()
    if not mlen:
        return 0
    qmagnitude = self.qmagnitude()  # <-- Uses qmagnitude, NOT qregion_len
    if not qmagnitude:
        return 0
    return mlen / qmagnitude
```

### 3.2 Python `qmagnitude()` Implementation

**Location**: `reference/scancode-toolkit/src/licensedcode/match.py:488-527`

```python
def qmagnitude(self):
    """
    Return the maximal query length represented by this match start and end
    in the query. This number represents the full extent of the matched
    query region including matched, unmatched AND unknown tokens, but
    excluding STOPWORDS.
    """
    query = self.query
    qspan = self.qspan
    qmagnitude = self.qregion_len()

    # note: to avoid breaking many tests we check query presence
    if query:
        # Compute a count of unknown tokens that are inside the matched
        # range, ignoring end position of the query span
        unknowns_pos = qspan & query.unknowns_span
        qspe = qspan.end
        unknowns_pos = (pos for pos in unknowns_pos if pos != qspe)
        qry_unkxpos = query.unknowns_by_pos
        unknowns_in_match = sum(qry_unkxpos[pos] for pos in unknowns_pos)

        # update the magnitude by adding the count of unknowns in the match
        qmagnitude += unknowns_in_match

    return qmagnitude
```

### 3.3 Key Difference Summary

| Aspect | Python `qdensity()` | Rust `qdensity()` |
|--------|---------------------|-------------------|
| Denominator | `qmagnitude()` | `qregion_len()` |
| Includes unknown tokens | Yes | No |
| Query parameter required | No (uses `self.query`) | No (and can't access query) |

---

## 4. Data Availability Analysis

### 4.1 Does Rust Have Access to Unknown Token Data?

**Yes!** The Rust `Query` struct has the required data:

**Location**: `src/license_detection/query.rs:200`

```rust
/// Mapping from token position to count of unknown tokens after that position
///
/// Unknown tokens are those not found in the dictionary. We track them by
/// counting how many unknown tokens appear after each known position.
pub unknowns_by_pos: HashMap<Option<i32>, usize>,
```

This is populated during query tokenization at lines 350-356:

```rust
} else if !started {
    *unknowns_by_pos.entry(None).or_insert(0) += 1;
} else {
    *unknowns_by_pos.entry(Some(known_pos)).or_insert(0) += 1;
}
```

### 4.2 Is Query Available at Call Sites?

**Yes!** The primary caller of `qdensity()` is `filter_spurious_matches()`:

**Location**: `src/license_detection/match_refine.rs:449`

```rust
fn filter_spurious_matches(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    matches
        .iter()
        .filter(|m| {
            // ...
            let qdens = m.qdensity();  // <-- Current call
            // ...
        })
        .cloned()
        .collect()
}
```

And `filter_spurious_matches()` is called from `refine_matches()`:

**Location**: `src/license_detection/match_refine.rs:1420`

```rust
pub fn refine_matches(
    index: &LicenseIndex,
    matches: Vec<LicenseMatch>,
    query: &Query,  // <-- Query is available!
) -> Vec<LicenseMatch> {
    // ...
    let non_spurious = filter_spurious_matches(&with_required_phrases);  // <-- Not passing query
    // ...
}
```

The `Query` reference is available in `refine_matches()` but not passed to `filter_spurious_matches()`.

---

## 5. Proposed Changes

### 5.1 Change `qdensity()` Signature

**File**: `src/license_detection/models.rs`  
**Lines**: 426-436

**Before**:

```rust
#[allow(dead_code)]
pub fn qdensity(&self) -> f32 {
    let mlen = self.len();
    if mlen == 0 {
        return 0.0;
    }
    let qregion = self.qregion_len();
    if qregion == 0 {
        return 0.0;
    }
    mlen as f32 / qregion as f32
}
```

**After**:

```rust
/// Return the query density of this match as a ratio of its length to its
/// qmagnitude, a float between 0 and 1.
///
/// A dense match has all its matched query tokens contiguous and a maximum
/// qdensity of one. A sparse low qdensity match has some non-contiguous
/// matched query tokens interspersed between matched query tokens.
/// An empty match has a zero qdensity.
///
/// Based on Python: `qdensity()` (match.py:565-579)
pub fn qdensity(&self, query: &crate::license_detection::query::Query) -> f32 {
    let mlen = self.len();
    if mlen == 0 {
        return 0.0;
    }
    let qmag = self.qmagnitude(query);
    if qmag == 0 {
        return 0.0;
    }
    mlen as f32 / qmag as f32
}
```

### 5.2 Change `filter_spurious_matches()` Signature

**File**: `src/license_detection/match_refine.rs`  
**Lines**: 440-474

**Before**:

```rust
fn filter_spurious_matches(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    matches
        .iter()
        .filter(|m| {
            let is_seq_or_unknown = m.matcher == "3-seq" || m.matcher == "5-unknown";
            if !is_seq_or_unknown {
                return true;
            }

            let qdens = m.qdensity();
            // ...
        })
        .cloned()
        .collect()
}
```

**After**:

```rust
/// Filter spurious matches with low density.
///
/// Spurious matches are matches with low density (where the matched tokens
/// are separated by many unmatched tokens). This filter only applies to
/// sequence and unknown matcher types - exact matches are always kept.
///
/// Based on Python: `filter_spurious_matches()` (match.py:1768-1836)
fn filter_spurious_matches(
    matches: &[LicenseMatch],
    query: &Query,
) -> Vec<LicenseMatch> {
    matches
        .iter()
        .filter(|m| {
            let is_seq_or_unknown = m.matcher == "3-seq" || m.matcher == "5-unknown";
            if !is_seq_or_unknown {
                return true;
            }

            let qdens = m.qdensity(query);  // Now passing query
            // ...
        })
        .cloned()
        .collect()
}
```

### 5.3 Update Call Site in `refine_matches()`

**File**: `src/license_detection/match_refine.rs`  
**Line**: 1420

**Before**:

```rust
let non_spurious = filter_spurious_matches(&with_required_phrases);
```

**After**:

```rust
let non_spurious = filter_spurious_matches(&with_required_phrases, query);
```

### 5.4 Update Tests for `qdensity()`

**File**: `src/license_detection/models.rs`  
**Lines**: 1363-1382

The existing tests need to be updated to pass a `Query` reference. Options:

1. Create a minimal mock Query with known `unknowns_by_pos`
2. Use the existing test infrastructure

**New test example**:

```rust
#[test]
fn test_qdensity_with_unknowns() {
    let mut match_result = create_license_match();
    match_result.start_token = 0;
    match_result.end_token = 10;
    match_result.matched_token_positions = Some(vec![0, 5, 10]); // 3 matches, span of 11
    
    // Create a mock Query with unknowns at positions 2, 3, 7
    let mut unknowns_by_pos = HashMap::new();
    unknowns_by_pos.insert(Some(2), 1);
    unknowns_by_pos.insert(Some(7), 1);
    
    // qregion_len = 11, unknowns = 2, qmagnitude = 13
    // qdensity = 3 / 13 = 0.231
    let expected = 3.0 / 13.0;
    
    // Need to create a Query or use a helper function
    // ...
}
```

---

## 6. Impact Analysis on Callers

### 6.1 Direct Callers of `qdensity()`

| Location | Current Usage | Impact |
|----------|---------------|--------|
| `match_refine.rs:449` | `m.qdensity()` | Must add `query` parameter |
| `models.rs:1365` (test) | `match_result.qdensity()` | Must create test Query |
| `models.rs:1373` (test) | `match_result.qdensity()` | Must create test Query |
| `models.rs:1381` (test) | `match_result.qdensity()` | Must create test Query |

### 6.2 Indirect Impact

The fix affects which matches are considered spurious and filtered out. This cascades to:

1. **Final match set**: Different matches may be kept/discarded
2. **Detection results**: Different license expressions may be reported
3. **Golden tests**: May need updates to expected results

### 6.3 Spurious Filter Thresholds

The thresholds in `filter_spurious_matches()` remain unchanged:

```rust
if mlen < 10 && (qdens < 0.1 || idens < 0.1) { return false; }
if mlen < 15 && (qdens < 0.2 || idens < 0.2) { return false; }
if mlen < 20 && hilen < 5 && (qdens < 0.3 || idens < 0.3) { return false; }
if mlen < 30 && hilen < 8 && (qdens < 0.4 || idens < 0.4) { return false; }
if qdens < 0.4 || idens < 0.4 { return false; }
```

With the fix, `qdensity` will typically be **lower** (larger denominator), making the filter **more aggressive** (more matches filtered as spurious). This aligns with Python behavior.

---

## 7. Test Requirements

Per `docs/TESTING_STRATEGY.md`, the following tests are required:

### 7.1 Unit Tests (Layer 1)

**Location**: `src/license_detection/models.rs` (existing test module at line 1468)

| Test Name | Description | Status |
|-----------|-------------|--------|
| `test_qdensity_contiguous` | Verify density = 1.0 for contiguous match | Update to pass Query |
| `test_qdensity_sparse` | Verify density for sparse match | Update to pass Query |
| `test_qdensity_zero` | Verify density = 0.0 for empty match | Update to pass Query |
| `test_qdensity_with_unknowns` | **NEW** - Verify density includes unknowns | Add |

**New test cases needed**:

1. Match with unknowns inside the span → lower density than without unknowns
2. Match with unknowns outside the span → same density as without unknowns
3. Match where all positions have unknowns → verify correct calculation

### 7.2 Unit Tests for `filter_spurious_matches()`

**Location**: `src/license_detection/match_refine.rs` (existing test module at line 1892)

| Test Name | Description | Status |
|-----------|-------------|--------|
| `test_filter_spurious_matches_keeps_non_seq_matchers` | Non-seq matches always kept | Update to pass Query |
| `test_filter_spurious_matches_keeps_high_density_seq` | High density seq matches kept | Update to pass Query |
| `test_filter_spurious_matches_filters_low_density_short` | Low density short matches filtered | Update to pass Query |
| `test_filter_spurious_matches_filters_unknown_matcher` | Unknown matcher filtered when low density | Update to pass Query |
| `test_filter_spurious_matches_keeps_medium_length` | Medium length matches kept | Update to pass Query |
| `test_filter_spurious_matches_empty` | Empty input returns empty | Update to pass Query |
| `test_filter_spurious_with_unknowns` | **NEW** - Test with unknown tokens | Add |

### 7.3 Golden Tests (Layer 2)

**Files affected**: Any golden tests that exercise spurious match filtering

Run full test suite before and after to identify changed outputs:

```bash
cargo test --all
```

Compare golden test outputs to verify they match Python reference more closely.

### 7.4 Integration Tests (Layer 3)

**Location**: `tests/scanner_integration.rs`

Verify end-to-end license detection produces expected results on sample files with unknown tokens.

---

## 8. Risk Assessment

### 8.1 Low Risk

- **Code change is straightforward**: One parameter addition
- **Logic already exists**: `qmagnitude()` is already correctly implemented
- **Type-safe**: Rust compiler will catch all call sites that need updating

### 8.2 Medium Risk

- **Golden test updates**: Some expected outputs may change
- **Edge cases**: Need to verify behavior when `unknowns_by_pos` is empty or contains positions outside match range

### 8.3 Mitigation Strategies

1. **Incremental testing**: Run tests after each change
2. **Golden test comparison**: Compare before/after outputs
3. **Python validation**: Test on real files and compare to Python scancode output

---

## 9. Implementation Checklist

- [ ] 1. Update `qdensity()` signature to accept `&Query` parameter
- [ ] 2. Change `qdensity()` implementation to use `qmagnitude(query)` instead of `qregion_len()`
- [ ] 3. Update `filter_spurious_matches()` signature to accept `&Query` parameter
- [ ] 4. Update call site in `refine_matches()` to pass `query`
- [ ] 5. Update existing `qdensity()` tests with mock Query
- [ ] 6. Add new test `test_qdensity_with_unknowns()`
- [ ] 7. Update `filter_spurious_matches()` tests with Query parameter
- [ ] 8. Add new test for filter behavior with unknowns
- [ ] 9. Run full test suite: `cargo test --all`
- [ ] 10. Run clippy: `cargo clippy --all-targets -- -D warnings`
- [ ] 11. Format code: `cargo fmt`
- [ ] 12. Compare golden test outputs to Python reference
- [ ] 13. Update documentation/comments if needed

---

## 10. Verification Commands

```bash
# Build
cargo build

# Run all tests
cargo test --all

# Run specific tests
cargo test qdensity
cargo test filter_spurious

# Check for clippy warnings
cargo clippy --all-targets -- -D warnings

# Format
cargo fmt --check
```

---

## 11. References

- Python `qdensity()`: `reference/scancode-toolkit/src/licensedcode/match.py:565-579`
- Python `qmagnitude()`: `reference/scancode-toolkit/src/licensedcode/match.py:488-527`
- Python `filter_spurious_matches()`: `reference/scancode-toolkit/src/licensedcode/match.py:1768-1836`
- Rust `qdensity()`: `src/license_detection/models.rs:426-436`
- Rust `qmagnitude()`: `src/license_detection/models.rs:417-423`
- Rust `Query.unknowns_by_pos`: `src/license_detection/query.rs:200`
- Testing Strategy: `docs/TESTING_STRATEGY.md`
- Related Plan: `docs/license-detection/PLAN-029-comprehensive-difference-analysis.md` (section 2.2)
