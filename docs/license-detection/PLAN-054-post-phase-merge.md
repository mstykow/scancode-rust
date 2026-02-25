# PLAN-054: Add Post-Phase Merge Calls

## Status: NOT IMPLEMENTED

## Summary

Python calls `merge_matches()` after each matching phase (hash, SPDX, Aho, sequence). Rust only merges at the end of `refine_matches()`. This can cause duplicate detections.

---

## Problem Statement

**Python** (index.py:1040-1050):

```python
# After each matching phase:
matches = match.merge_matches(matches)
```

**Rust**: Only merges once at end of `refine_matches()`.

---

## Impact

- ~200 external tests affected
- Duplicate detections in output
- Different match counts than Python

---

## Implementation

**Location**: `src/license_detection/mod.rs:117-271`

Add merge calls after each matching phase:
1. After hash match phase
2. After SPDX match phase  
3. After Aho-Corasick match phase
4. After sequence match phase

Also add hash match early return when matches found (Python optimization).

---

## Priority: MEDIUM

Affects external tests significantly but may be complex to implement correctly.

---

## Reference

- PLAN-029 section 2.4
- PLAN-037 (referenced but not created)
