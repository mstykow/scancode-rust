# PLAN-018: Fix Clippy Warnings

## Status: READY TO IMPLEMENT

## Summary
Clean up clippy warnings in the license detection code by aligning with Python reference.

## Remaining Warnings (6)

### 1. Dead Code: `remove_duplicate_detections` (detection.rs)
**Python:** `get_detections_by_id()` at detection.py:1017
**Where Python uses it:** `create_unique_license_detections()` groups detections by identifier
**Fix:** Call when creating unique license detections (currently only used in tests)

### 2. Dead Code: `compute_detection_identifier` (detection.rs)
**Python:** `identifier_with_expression` property at detection.py:306-341
**Where Python uses it:** Sets `detection.identifier` at lines 265, 947, 2175
**Fix:** Call when creating `LicenseDetection` objects to populate `identifier` field

### 3. Dead Code: `unescape_html_entities` (models.rs)
**Python:** No equivalent - Python treats HTML entities as stopwords, doesn't unescape
**Fix:** Remove this function - it's unnecessary

### 4. Collapsible `if` (match_refine.rs:457-461)
**Current:**
```rust
if current_is_ref && other_is_text && current.matched_length < other.matched_length {
    if other.qcontains(current) {
        to_discard.insert(i);
    }
}
```
**Fix:** Collapse to single condition

### 5. Identical `if` blocks (file_text.rs:135-139)
**Analysis:** NOT a bug - UTF-32 LE and BE BOMs both need to skip 4 bytes
**Fix:** Combine conditions with `||`

### 6. Missing `is_empty` (LicenseMatch)
**Fix:** Add `is_empty()` method:
```rust
pub fn is_empty(&self) -> bool {
    self.len() == 0
}
```

## Files to Modify
- `src/license_detection/detection.rs` - add call sites for #1, #2
- `src/license_detection/models.rs` - remove #3, add #6
- `src/license_detection/match_refine.rs` - fix #4
- `src/utils/file_text.rs` - fix #5

## Verification
```bash
cargo clippy --lib 2>&1 | grep -E "^warning:" | wc -l
```
Should be 0 after fixes.

## Success Criteria
- [ ] All clippy warnings resolved
- [ ] Code still compiles
- [ ] All tests pass
