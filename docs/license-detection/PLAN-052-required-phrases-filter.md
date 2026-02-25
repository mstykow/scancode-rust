# PLAN-052: Implement filter_matches_missing_required_phrases

## Status: IMPLEMENTED

## Summary

Python has a filter that removes matches where required phrases (marked with `{{...}}` in rule text) weren't matched. Rust has this filter fully implemented, including the previously documented bug fixes for `qcontains()` set containment and `match_coverage` recalculation.

---

## Current Implementation Status

### ✅ Already Implemented

| Component | Field/Method | File | Status |
|-----------|--------------|------|--------|
| **Rule** | `is_continuous` | `models.rs:116` | ✅ Present |
| **Rule** | `is_required_phrase` | `models.rs:104` | ✅ Present |
| **Rule** | `required_phrase_spans` | `models.rs:120` | ✅ Present |
| **Rule** | `stopwords_by_pos` | `models.rs:124` | ✅ Present |
| **LicenseMatch** | `qspan_positions` | `models.rs:318` | ✅ Present |
| **LicenseMatch** | `ispan_positions` | `models.rs:324` | ✅ Present |
| **LicenseMatch** | `rule_start_token` | `models.rs:312` | ✅ Present |
| **LicenseMatch** | `ispan()` | `models.rs:555` | ✅ Present |
| **LicenseMatch** | `qspan()` | `models.rs:563` | ✅ Present |
| **LicenseMatch** | `is_continuous()` | `models.rs:545` | ✅ Present |
| **Tokenizer** | `parse_required_phrase_spans()` | `tokenize.rs:224` | ✅ Present |
| **Index Builder** | Populates `required_phrase_spans` | `index/builder.rs:275` | ✅ Present |
| **Index Builder** | Populates `stopwords_by_pos` | `index/builder.rs:277` | ✅ Present |
| **Filter** | `filter_matches_missing_required_phrases()` | `match_refine.rs:1019` | ✅ Present |
| **Pipeline** | Called in `refine_matches()` | `match_refine.rs:1441` | ✅ Present |

### Implementation Details

**Required Phrase Parsing** (`tokenize.rs:224-279`):
- Parses `{{...}}` markers from rule text
- Filters stopwords during tokenization (matches Python behavior)
- Returns `Vec<Range<usize>>` for token positions

**Filter Implementation** (`match_refine.rs:1019-1205`):
- Handles solo match exception (intentionally skipping Python bug)
- Checks `is_continuous` / `is_required_phrase` rules
- Validates required phrase containment in `ispan`
- Checks for unknown words and stopword count mismatches

**Pipeline Order** (`match_refine.rs:1428-1490`):
1. Merge overlapping matches
2. **Filter matches missing required phrases** ← Implemented here
3. Filter spurious matches
4. Filter below minimum coverage
5. Filter spurious single-token matches
6. Filter too short matches
7. Filter scattered short matches
8. Filter invalid gibberish (binary files)
9. Merge overlapping matches again
10. Filter contained matches
11. Filter overlapping matches
12. Restore non-overlapping discarded
13. Filter false positive matches
14. Filter false positive license lists
15. Update match scores

---

## Known Issues (from BUG-009)

The implementation is complete. Two previously documented issues have been fixed:

### Issue 1: Missing `is_license_notice` in `LicenseMatch`

**Status**: NOT CAUSING TEST FAILURES (Python doesn't serialize these to JSON)

**Impact**: Feature parity gap, but not affecting golden tests

**Note**: `is_license_text` is already present in `LicenseMatch` (models.rs:287). Only `is_license_notice` is missing.

### Issue 2: `qcontains()` set containment ✅ FIXED

**Status**: FIXED in `models.rs:505-510`

The implementation now correctly uses set containment when `qspan_positions` is available:

```rust
pub fn qcontains(&self, other: &LicenseMatch) -> bool {
    if let (Some(self_positions), Some(other_positions)) =
        (&self.qspan_positions, &other.qspan_positions)
    {
        return other_positions.iter().all(|p| self_positions.contains(p));
    }
    // Fallback to range containment...
}
```

### Issue 3: `match_coverage` recalculation in merge ✅ FIXED

**Status**: FIXED in `match_refine.rs:145-149`

The `combine_matches()` function now recalculates coverage after merging:

```rust
if merged.rule_length > 0 {
    merged.match_coverage = (merged.matched_length.min(merged.rule_length) as f32
        / merged.rule_length as f32)
        * 100.0;
}
```

---

## Remaining Work

### Priority 1: Add Missing `is_license_notice` to `LicenseMatch` (Optional)

**Status**: Low priority - not affecting golden tests

**File**: `src/license_detection/models.rs`

Note: `is_license_text` is already present (line 287). Only `is_license_notice` is missing.

### Priority 2: Add Unit Tests for Required Phrases Filter

#### Test 2.1: Required Phrase Filter Edge Cases

```rust
#[test]
fn test_filter_required_phrases_keeps_match_with_all_phrases() {
    // Match where all required phrase spans are contained in ispan
}

#[test]
fn test_filter_required_phrases_discards_missing_phrase() {
    // Match where required phrase span is NOT contained in ispan
}

#[test]
fn test_filter_required_phrases_continuous_rule_non_continuous_match() {
    // is_continuous=true rule but match is not continuous -> discard
}

#[test]
fn test_filter_required_phrases_unknown_in_required_span() {
    // Unknown words in required phrase span -> discard
}

#[test]
fn test_filter_required_phrases_stopword_mismatch() {
    // Stopword count mismatch between rule and query -> discard
}
```

#### Test 2.2: Integration with Real Rules

```rust
#[test]
fn test_gpl_2_0_7_required_phrases() {
    // Use actual gpl-2.0_7.RULE file
    // Verify required_phrase_spans match Python output
    // Verify filter behavior matches Python
}
```

---

## Testing Strategy

Following `docs/TESTING_STRATEGY.md`:

### Unit Tests (Layer 1)

| Test Category | Location | Purpose |
|--------------|----------|---------|
| `parse_required_phrase_spans` | `tokenize.rs` | Verify stopword filtering, edge cases |
| `qcontains` | `models.rs` | Verify set vs range containment |
| `combine_matches` | `match_refine.rs` | Verify coverage recalculation |
| `filter_required_phrases` | `match_refine.rs` | Verify all filter conditions |

### Golden Tests (Layer 2)

Run after fixes:
```bash
cargo test license_golden --lib
```

Expected improvements after fixes:
- `lic1`: Reduce extra matches
- `crapl-0.1.txt`: Should now detect correctly
- `COPYING.gplv3`: Should have single match

### Integration Tests (Layer 3)

Verify full pipeline with:
```bash
cargo test debug_gpl_2_0_9_required_phrases_filter --lib -- --nocapture
```

---

## Reference Files

| Component | Python | Rust |
|-----------|--------|------|
| Span class | `spans.py:42-474` | `spans.rs` |
| Required phrase tokenizer | `tokenize.py:90-174` | `tokenize.rs:224-332` |
| Filter function | `match.py:2154-2328` | `match_refine.rs:1019-1205` |
| Pipeline | `match.py:2691-2833` | `match_refine.rs:1428-1490` |

---

## Related Documents

- **BUG-009**: Detailed analysis of remaining issues
- **PLAN-017**: Original issue #6 with code samples
- **TESTING_STRATEGY.md**: Testing approach guidelines

---

## History

| Date | Action |
|------|--------|
| Initial | Plan created with gap analysis |
| 2024-01 | `parse_required_phrase_spans()` implemented |
| 2024-01 | Stopword filtering added to tokenizer |
| 2024-01 | `filter_matches_missing_required_phrases()` implemented |
| 2024-01 | `qcontains()` set containment fixed |
| 2024-01 | `match_coverage` recalculation in merge fixed |
| Current | Plan updated: Issues 2 and 3 marked as FIXED, status changed to IMPLEMENTED |
