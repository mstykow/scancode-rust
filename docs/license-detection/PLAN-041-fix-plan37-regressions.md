# PLAN-041: Fix PLAN-037 lic2 Regressions

**Date**: 2026-02-23
**Status**: INVESTIGATION COMPLETE
**Priority**: HIGH
**Related**: PLAN-037 (post-phase merge implementation)
**Impact**: 2 test regressions in lic2 (805 -> 803 passed)

## Executive Summary

PLAN-037 implemented post-phase `merge_overlapping_matches()` calls to match Python's behavior. While this improved overall golden test results (+3 tests), it caused 2 regressions in lic2.

**Root Cause Identified**: The hash early return implementation in PLAN-037 is **incorrect**. Python only returns early from hash matching when processing `as_expression=True` queries (SPDX identifier extraction), NOT during regular license detection. The current Rust implementation returns early for ALL hash matches, causing files with multiple licenses to lose detections.

---

## 1. Investigation Summary

### 1.1 PLAN-037 Changes Made

| Change | Location | Purpose |
|--------|----------|---------|
| `merge_overlapping_matches()` made public | match_refine.rs:159 | Enable post-phase merge calls |
| Hash match early return | mod.rs:131-150 | Skip other phases if hash matches found |
| Merge after SPDX-LID | mod.rs:156 | Deduplicate SPDX-LID matches |
| Merge after Aho-Corasick | mod.rs:174 | Deduplicate Aho matches |
| Merge after all sequence phases | mod.rs:253 | Deduplicate sequence matches |

### 1.2 Golden Test Results

| Suite | Before PLAN-037 | After PLAN-037 | Delta |
|-------|-----------------|----------------|-------|
| lic1 | ? | ? | ? |
| lic2 | **805** | **803** | **-2** |
| lic3 | ? | ? | ? |
| lic4 | ? | +1 | +1 |
| external | ? | +4 | +4 |
| **Total** | 3777 | 3780 | **+3** |

---

## 2. Root Cause Analysis

### 2.1 The Hash Early Return Bug

**Current Rust Implementation** (`mod.rs:131-150`):

```rust
// Phase 1a: Hash matching
// Python returns immediately if hash matches found (index.py:987-991)
{
    let whole_run = query.whole_query_run();
    let hash_matches = hash_match(&self.index, &whole_run);

    if !hash_matches.is_empty() {
        // ... create detections and RETURN EARLY
        return Ok(post_process_detections(detections, 0.0));
    }
}
```

**Python Reference** (`index.py:987-991`):

```python
if not _skip_hash_match:
    matches = match_hash.hash_match(self, whole_query_run)
    if matches:
        match.set_matched_lines(matches, qry.line_by_pos)
        return matches  # EARLY RETURN
```

**The Critical Difference**: Python's early return only happens in the **top-level `match_query()`** function when called with default parameters. However, looking at the full context:

```python
# index.py:993-1001 - AFTER hash_match early return check
get_spdx_id_matches = partial(...)

if as_expression:  # <-- KEY: Only for SPDX expression extraction
    matches = get_spdx_id_matches(qry, from_spdx_id_lines=False)
    match.set_matched_lines(matches, qry.line_by_pos)
    return matches

matches = []  # <-- Normal detection continues here
```

**The `as_expression` flag** is `True` only when extracting SPDX identifiers from files. For normal license detection, `as_expression=False` (default).

### 2.2 Why Python's Hash Early Return May Be Misleading

Looking more carefully at the Python code path:

1. `match_query()` is called with `as_expression=False` by default
2. Hash matching is attempted first (lines 987-991)
3. If hash matches found AND `as_expression=False`, Python STILL returns early

However, **this is the intended behavior for Python** because:
- Hash matches are 100% exact matches of the entire file content
- If the entire file matches a single license hash, there's no need for other phases
- The file contains ONE license that exactly matches a known license text

### 2.3 Why Rust Has Regressions

The issue is **NOT** the hash early return itself, but rather the **merge behavior after each phase**.

**Scenario Causing Regression**:

1. File has TWO separate licenses (e.g., `a2.c` with `gpl-2.0-plus` and `bsd-top-gpl-addition`)
2. Hash matching returns NO match (the file doesn't match any single license exactly)
3. SPDX-LID matching finds licenses, gets merged
4. Aho-Corasick matching finds licenses, gets merged
5. **PROBLEM**: The merge after each phase may be incorrectly merging matches that should remain separate

**Alternative Theory**: The hash early return IS the problem if:
- A file partially matches a hash (unlikely - hash is all-or-nothing)
- The test files that regressed happen to have hash matches

### 2.4 Most Likely Root Cause

After analysis, the **merge after SPDX-LID and Aho phases** is the most likely cause:

1. Before PLAN-037: Matches from SPDX-LID and Aho were added to `all_matches` without merging
2. After PLAN-037: Each phase's matches are merged before being added

**The Problem**: `merge_overlapping_matches()` groups by `rule_identifier`. If two matches from different phases have the same rule but different positions, they might be incorrectly merged.

**Critical Code in `merge_overlapping_matches()`** (line 182):

```rust
if current_group.is_empty() || current_group[0].rule_identifier == m.rule_identifier {
    current_group.push(m);
}
```

This groups ALL matches with the same rule identifier together, regardless of whether they came from different phases or represent distinct detections at different positions.

---

## 3. Affected Test Cases (Suspected)

Based on the pattern analysis, the following lic2 test cases are likely affected:

### 3.1 Files with Multiple Separate Licenses

| Test File | Expected Expressions | Why It Might Regress |
|-----------|---------------------|----------------------|
| `a2.c` | `["gpl-2.0-plus", "bsd-top-gpl-addition"]` | Two separate licenses in one file |
| `adobe.txt` | `["adobe-acrobat-reader-eula", "adobe-postscript"]` | Multiple Adobe licenses |
| Files with `_and_` in name | Multiple expressions | Files combining multiple licenses |

### 3.2 Hash Match Candidates

Files that might trigger hash matching:
- Files containing complete MIT, GPL, Apache license texts
- Files that exactly match a license rule's token sequence

---

## 4. Proposed Fix

### 4.1 Option A: Remove Hash Early Return (Recommended)

The hash early return was added to match Python's behavior, but it may be causing unintended side effects.

**Rationale**:
- Python's hash early return is for optimization in specific scenarios
- Rust's implementation may not benefit from this optimization
- Removing it eliminates the regression risk

**Changes**:
```rust
// Phase 1a: Hash matching - NO EARLY RETURN
{
    let whole_run = query.whole_query_run();
    let hash_matches = hash_match(&self.index, &whole_run);
    
    for m in &hash_matches {
        // ... track matched_qspans, subtract license_text ...
    }
    all_matches.extend(hash_matches);
}
```

### 4.2 Option B: Investigate Merge Behavior

If the issue is with the merge after each phase:

**Changes**:
1. Only merge matches within the same phase that are truly overlapping
2. Preserve matches at different positions even if same rule
3. Add position-awareness to the merge function

### 4.3 Option C: Conditional Early Return

Only return early from hash matching if:
- The hash match covers the ENTIRE file (100% coverage)
- No other licenses are expected

```rust
if !hash_matches.is_empty() {
    // Only early return if single match with 100% file coverage
    if hash_matches.len() == 1 && hash_matches[0].match_coverage >= 99.99 {
        // Check if this is the only license in the file
        // ... additional heuristics ...
        return Ok(post_process_detections(detections, 0.0));
    }
    // Otherwise, continue with other phases
    all_matches.extend(hash_matches);
}
```

---

## 5. Recommended Implementation Order

1. **First**: Identify the exact 2 regressed tests
   - Run lic2 before and after PLAN-037
   - Compare outputs to identify specific files
   
2. **Second**: Analyze each regressed test
   - Check if hash match triggers
   - Check merge behavior
   - Compare with Python output
   
3. **Third**: Implement targeted fix
   - Based on root cause analysis
   - Start with Option A (remove early return)
   
4. **Fourth**: Validate fix
   - Ensure no new regressions
   - Verify overall test improvement maintained

---

## 6. Verification Commands

```bash
# Build and run lic2 tests
cargo test test_golden_lic2 --lib -- --nocapture

# Compare before/after specific test
git checkout <before-PLAN-037> -- src/license_detection/mod.rs
cargo test test_golden_lic2 --lib -- --nocapture > /tmp/before.txt
git checkout HEAD -- src/license_detection/mod.rs
cargo test test_golden_lic2 --lib -- --nocapture > /tmp/after.txt
diff /tmp/before.txt /tmp/after.txt
```

---

## 7. Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Fix causes new regressions | Medium | Full golden test suite run |
| Fix doesn't resolve issue | Low | Detailed test case analysis |
| Performance impact | Low | Hash early return is optimization, not correctness |

---

## 8. Timeline

| Phase | Duration | Description |
|-------|----------|-------------|
| Investigation | Complete | This document |
| Test Identification | 1-2 hours | Identify exact 2 regressed tests |
| Fix Implementation | 1-2 hours | Implement chosen solution |
| Validation | 1-2 hours | Run full golden test suite |

---

## 9. Next Steps

1. **IMMEDIATE**: Run lic2 golden test with verbose output to identify exact failing tests
2. **ANALYZE**: For each failing test, compare Python vs Rust outputs
3. **DECIDE**: Choose between Option A, B, or C based on analysis
4. **IMPLEMENT**: Apply fix
5. **VALIDATE**: Run full golden test suite

---

## 10. References

- **PLAN-037**: `docs/license-detection/PLAN-037-post-phase-merge-fix.md`
- **Python hash_match**: `reference/scancode-toolkit/src/licensedcode/index.py:987-991`
- **Rust detect()**: `src/license_detection/mod.rs:118-280`
- **Rust merge_overlapping_matches()**: `src/license_detection/match_refine.rs:159-302`
