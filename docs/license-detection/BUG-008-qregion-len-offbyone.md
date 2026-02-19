# BUG-008: qregion_len() Off-by-One and Filter Logic Bugs

## Status: Ready for Implementation

---

## 1. Summary

Multiple bugs found causing golden test regression:

| Bug | Location | Severity | Impact |
|-----|----------|----------|--------|
| **qregion_len() off-by-one** | models.rs:400 | **CRITICAL** | `is_continuous()` always returns false |
| qkey_span construction | match_refine.rs:965 | HIGH | Wrong positions for non-contiguous ispan |
| Extra unknown check | match_refine.rs:988-989 | MEDIUM | More restrictive than Python |
| Stopword check condition | match_refine.rs:1002 | MEDIUM | Wrong condition |

---

## 2. Bug 1: qregion_len() Off-by-One (CRITICAL)

### The Problem

**File**: `src/license_detection/models.rs:400`

```rust
fn qregion_len(&self) -> usize {
    if let Some(positions) = &self.matched_token_positions {
        // non-contiguous case...
    } else {
        self.end_token.saturating_sub(self.start_token) + 1  // WRONG!
    }
}
```

### Why It's Wrong

- Rust's `end_token` is **exclusive** (like Rust ranges: `0..10` means 0-9)
- Python's `Span(end)` is **inclusive** (Span(0, 9) means 0-9)
- Python's `magnitude = end - start + 1` is correct for inclusive
- Rust's `qregion_len = end_token - start_token + 1` is WRONG for exclusive

### Example

For a match covering positions 0-9 (10 tokens):

| | Python | Rust |
|---|--------|------|
| Representation | `Span(0, 9)` inclusive | `start_token=0, end_token=10` exclusive |
| `len()` | 10 | 10 ✓ |
| `qregion_len()` | 9-0+1 = 10 | 10-0+1 = **11** ✗ |

### Impact on `is_continuous()`

```rust
pub fn is_continuous(&self, query: &Query) -> bool {
    let len = self.len();           // 10
    let qregion_len = self.qregion_len();  // 11 (wrong!)
    let qmagnitude = self.qmagnitude(query);
    len == qregion_len && qregion_len == qmagnitude  // 10 == 11 = FALSE
}
```

`is_continuous()` returns `false` for ALL contiguous matches!

### Fix

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
        self.end_token.saturating_sub(self.start_token)  // Remove +1
    }
}
```

---

## 3. Bug 2: qkey_span Construction

### The Problem

**File**: `src/license_detection/match_refine.rs:965`

```rust
let ipos = ispan_min + qi;  // Assumes ispan is contiguous!
```

### Why It's Wrong

After merge, `ispan` can have gaps (non-contiguous). Using `ispan_min + qi` assumes positions are sequential.

### Python's Approach (match.py:2249-2254)

```python
qkey_poss = (
    qpos for qpos, ipos in zip(qspan, ispan)
    if ipos in ikey_span
)
qkey_span = Span(qkey_poss)
```

Python uses `zip(qspan, ispan)` to pair positions correctly.

### Fix

```rust
let qkey_span: Vec<usize> = qspan
    .iter()
    .zip(ispan.iter())
    .filter_map(|(&qpos, &ipos)| {
        if ikey_span.contains(&ipos) {
            Some(qpos)
        } else {
            None
        }
    })
    .collect();
```

---

## 4. Bug 3: Extra Unknown Check

### The Problem

**File**: `src/license_detection/match_refine.rs:988-989`

```rust
query.unknowns_by_pos.contains_key(&Some(qpos as i32))
    || query.unknowns_by_pos.contains_key(&Some(qpos as i32 - 1))  // EXTRA!
```

### Why It's Wrong

Python only checks `qpos in unknown_by_pos`, not `qpos - 1`.

### Fix

Remove the extra check:

```rust
let contains_unknown = qkey_span.iter().take(qkey_span.len() - 1).any(|&qpos| {
    query.unknowns_by_pos.contains_key(&Some(qpos as i32))
});
```

---

## 5. Bug 4: Stopword Check Condition

### The Problem

**File**: `src/license_detection/match_refine.rs:1002`

```rust
if !ikey_span.contains(&ipos) {  // Wrong!
```

### Why It's Wrong

Python checks `qpos not in qkey_span` (query position membership), not `ipos not in ikey_span` (rule position membership).

### Python's Approach (match.py:2290-2291)

```python
if qpos not in qkey_span or qpos == qkey_span_end:
    continue
```

### Fix

```rust
let qkey_span_set: HashSet<usize> = qkey_span.iter().copied().collect();
let qkey_span_end = qkey_span.last().copied();

for (&qpos, &ipos) in qspan.iter().zip(ispan.iter()) {
    if !qkey_span_set.contains(&qpos) || Some(qpos) == qkey_span_end {
        continue;
    }
    // ...
}
```

---

## 6. Implementation Order

1. **Fix qregion_len()** - Most critical, fixes `is_continuous()`
2. **Fix qkey_span construction** - Use `zip(qspan, ispan)`
3. **Fix unknown check** - Remove extra `qpos - 1` check
4. **Fix stopword check** - Use `qkey_span` for membership check

---

## 7. Verification

After fixes:

1. Run `cargo test --release -q --lib license_detection::golden_test`
2. Expect significant improvement in lic1-4 results
3. External should remain improved or stay similar
