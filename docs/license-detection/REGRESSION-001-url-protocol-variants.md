# REGRESSION-001: URL Protocol Variants

**Status:** Investigating  
**Created:** 2026-03-01  
**Commit:** 3ad4daaf  
**Impact:** 3 golden test regressions (145 failures up from 142 baseline)

## Summary

The commit `3ad4daaf` introduced URL protocol variants for `ignorable_urls` in rule matching. This change generates `http://` variants for rules with `https://` in ignorable_urls, enabling matching dual-license rules when files use different URL protocols.

While this fixed the BSL-1.0_or_MIT.txt test case, it introduced 3 regressions.

## Regressed Test Files

### 1. `datadriven/lic1/gpl-2.0_82.RULE`

**Expected:**
```json
["gpl-2.0", "gpl-2.0", "gpl-2.0"]
```

**Actual:**
```json
["gpl-2.0", "gpl-2.0"]
```

**Issue:** One `gpl-2.0` match is missing. The file contains a short GPL-2.0 notice at the beginning AND the full GPL-2.0 license text. Previously 3 matches were detected, now only 2.

### 2. `datadriven/lic1/gpl-2.0_and_lppl-1.3c_and_public-domain.label`

**Expected:**
```json
["gpl-2.0", "gpl-1.0-plus", "tex-live", "gpl-1.0-plus", "public-domain", ...]
```

**Actual:**
```json
["gpl-2.0", "gpl-1.0-plus", "tex-live", "public-domain", ...]
```

**Issue:** The fourth expected match `gpl-1.0-plus` is missing. The file contains TeX Live copyright information with multiple license references.

### 3. `datadriven/lic1/gpl_and_gpl_and_gpl_and_lgpl-2.0_and_other.txt`

**Expected:**
```json
["gpl-1.0-plus", "gpl-1.0-plus", "lgpl-2.1-plus", "lgpl-2.1-plus", "gpl-1.0-plus"]
```

**Actual:**
```json
["gpl-1.0-plus", "gpl-1.0-plus", "lgpl-2.1-plus", "lgpl-2.1-plus", "unknown-license-reference", "lgpl-3.0"]
```

**Issue:** Two incorrect detections at the end. The file ends with license references that should be `gpl-1.0-plus` but are being detected as `unknown-license-reference` and `lgpl-3.0`.

## Root Cause Analysis

The URL protocol variants change adds two mechanisms:

1. **Variant hash generation** (lines 390-427): Creates alternate hashes for rules with ignorable URLs, replacing `https://` with `http://`

2. **Token set expansion** (lines 463-480): Adds `http` token to the `tids_set` for rules with `https://` ignorable URLs

The regressions appear to be caused by unintended interactions between these mechanisms and the match filtering/deduplication logic.

### Hypothesis 1: Token Set Expansion Too Broad

The token set expansion adds `http` to the token set unconditionally for ANY rule with an `https://` ignorable URL. This makes the matching more permissive than intended:

```rust
if let (Some(https_tid), Some(http_tid)) =
    (dictionary.get("https"), dictionary.get("http"))
{
    if tids_set.contains(&https_tid) {
        tids_set.insert(http_tid);  // Always adds http if https present
    }
}
```

This could cause:
- Rules to match files that happen to contain `http` tokens but not the specific ignorable URL
- Incorrect filtering of other matches due to overlap in token ranges

### Hypothesis 2: Variant Hash Collisions

The variant hash generation creates new hash entries mapping to the same rule ID. This could cause:
- Duplicate match detection when both original and variant patterns match
- Match deduplication removing legitimate matches

### Hypothesis 3: Containment Filtering Interaction

The containment filtering logic may be incorrectly eliminating matches when URL variants are involved. If a shorter match has its token set expanded to include `http`, it might incorrectly "contain" other matches.

## Recommended Fix Approach

### Option A: Scope URL Variant to Specific URLs

Instead of adding `http` token unconditionally, only add it when the file actually contains a matching URL (same domain/path, different protocol):

```rust
// Only expand token set if file contains http:// variant of ignorable URL
for url in ignorable_urls {
    if url.starts_with("https://") {
        let http_url = format!("http://{}", &url[8..]);
        if file_text.contains(&http_url) {
            // Only now add the http token
        }
    }
}
```

### Option B: Separate Variant Rules

Instead of modifying existing rule token sets, create separate "variant rules" that are matched independently. This avoids polluting the original rule's token set.

### Option C: Fix Match Deduplication

The issue may not be in the URL variant logic itself, but in how matches are deduplicated when both original and variant patterns match. Review the match deduplication logic to ensure it handles variant matches correctly.

## Next Steps

1. Add debug logging to trace which rules match for the regressed test files
2. Compare token sets before and after URL variant expansion
3. Verify match deduplication handles variant matches correctly
4. Consider implementing Option A as the most conservative fix

## Validation Update

**Date:** 2026-03-01

### Issue Status

| Test | File | Status | Details |
|------|------|--------|---------|
| Test 1 | `gpl-2.0_82.RULE` | **REGRESSION EXISTS** | Missing one `gpl-2.0` match |
| Test 2 | `gpl-2.0_and_lppl-1.3c_and_public-domain.label` | **NEEDS CLARIFICATION** | Different expression structure from expected |
| Test 3 | `gpl_and_gpl_and_gpl_and_lgpl-2.0_and_other.txt` | **REGRESSION EXISTS** | Wrong last detection (detects `unknown-license-reference` and `lgpl-3.0` instead of `gpl-1.0-plus`) |

### Python Comparison

**Critical Finding:** Python ScanCode does NOT create URL protocol variants. This is a Rust-specific optimization that does not exist in the original Python implementation.

- The `BSL-1.0_or_MIT.txt` fix works in Rust, but the approach fundamentally differs from Python's behavior
- Python handles dual-license matching without needing URL protocol variant generation
- The Rust optimization introduces regressions that Python does not have

### Recommended Action

1. **Consider removing the URL variant feature entirely** - Since Python doesn't use this approach and it introduces regressions, the feature may not be worth the complexity.

2. **Find an alternative approach for `BSL-1.0_or_MIT.txt`** - Any fix must match Python's behavior rather than implementing Rust-specific optimizations that diverge from the reference implementation.

3. **Root cause remains:** The core issue is token set expansion adding `http` to rules, which causes incorrect matching in files that contain `http` tokens unrelated to the ignorable URL being matched.

### Conclusion

The URL protocol variant feature should be reconsidered. The regressions it introduces outweigh the benefit of fixing a single test case. A solution that maintains parity with Python's behavior is required.

## Related Documents

- Original implementation: `docs/license-detection/0019-phase3-expression-combination-plan.md`
- Code location: `src/license_detection/index/builder.rs:390-480`

---

## Resolution

**Date:** 2026-03-01
**Action:** Removed the URL protocol variants feature entirely

The Rust-specific URL protocol variants feature has been removed because:
1. Python doesn't have this feature - it was a Rust-specific optimization
2. It caused 3 test regressions due to overly broad token set expansion
3. The fix for BSL-1.0_or_MIT.txt should be achieved through a different approach

**Result:**
- All 3 regressions fixed
- Test count improved: 133 → 130 failures
- BSL-1.0_or_MIT.txt still detects both `mit` and `boost-1.0` licenses (via different matching)

**Status:** ✅ RESOLVED - Feature removed, regressions fixed
