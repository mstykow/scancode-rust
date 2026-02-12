# Matcher Order Analysis: No Missing "Tag" Matcher

**Date**: 2026-02-12
**Author**: Research findings for TODO-4.3

## Summary

**Conclusion**: There is NO missing "tag" matcher. The gap in matcher order numbering is due to a naming confusion between Python's `MATCH_UNKNOWN_ORDER = 6` and `MATCHER_UNDETECTED_ORDER = 4`.

## Detailed Analysis

### Python Matcher Orders

| Matcher | Constant | Order | String |
|---------|----------|-------|--------|
| Hash | `MATCH_HASH_ORDER` | 0 | `"1-hash"` |
| AHO Exact | `MATCH_AHO_EXACT_ORDER` | 1 | `"2-aho"` |
| SPDX-LID | `MATCH_SPDX_ID_ORDER` | 2 | `"1-spdx-id"` |
| Sequence | `MATCH_SEQ_ORDER` | 3 | `"3-seq"` |
| Undetected | `MATCHER_UNDETECTED_ORDER` | 4 | `"5-undetected"` |
| AHO Frag | `MATCH_AHO_FRAG_ORDER` | 5 | `"5-aho-frag"` |
| Unknown | `MATCH_UNKNOWN_ORDER` | 6 | `"6-unknown"` |

**Key Finding**: Python has TWO different constants for similar concepts:

- `MATCHER_UNDETECTED_ORDER = 4` with matcher `"5-undetected"` - Used in `detection.py` for creating matches from query strings (utility function `get_undetected_matches()`)
- `MATCH_UNKNOWN_ORDER = 6` with matcher `"6-unknown"` - Used in `match_unknown.py` for ngram-based unknown license detection

### What `is_license_tag` Actually Is

The `is_license_tag` flag on rules is NOT a separate matcher. It's a rule classification used for:

1. **Scattered match tolerance** (match.py:1959-1960): License tag matches can span extra lines
2. **False positive detection** (detection.py:1218-1237): Tags matching single rules may be filtered
3. **Candidate filtering** (match.py:2660-2688): Tags are considered in false positive candidate detection

A "license tag" is a short structured license identifier like:

- SPDX identifiers in code comments
- License fields in package manifests (e.g., `"license": "MIT"` in package.json)
- Structured license name references

These are detected by EXISTING matchers (primarily AHO exact matching), not a separate "tag matcher."

### Rust Implementation Status

The Rust implementation correctly handles `is_license_tag`:

1. **Rule model** (`models.rs:52`): Has `is_license_tag: bool` field
2. **Rule loader** (`rules/loader.rs:159, 298`): Parses `is_license_tag` from .RULE files
3. **Index builder** (`index/builder.rs:102`): Uses `is_license_tag` for `is_approx_matchable` classification

### Current Rust Matcher Orders

| Matcher | Constant | Order | String |
|---------|----------|-------|--------|
| Hash | `MATCH_HASH_ORDER` | 0 | `"1-hash"` |
| AHO | `MATCH_AHO_ORDER` | 2 | (combined exact) |
| SPDX-LID | `MATCH_SPDX_ID_ORDER` | 1 | (via SPDX detection) |
| Sequence | `MATCH_SEQ_ORDER` | 3 | `"3-seq"` |
| Unknown | `MATCH_UNKNOWN_ORDER` | 5 | `"5-undetected"` |

### Discrepancy Explanation

The Rust implementation uses:

- `MATCH_UNKNOWN: &str = "5-undetected"` (not `"6-unknown"`)
- `MATCH_UNKNOWN_ORDER: u8 = 5` (not 6)

This is a **minor naming inconsistency** but functionally correct:

- Rust combines the "undetected" (order 4) and "unknown" (order 6) concepts into a single matcher
- The matcher name `"5-undetected"` is closer to Python's `MATCHER_UNDETECTED`
- The order value (5) falls between Python's undetected (4) and unknown (6)

## Recommendation

**No new matcher implementation needed.** The current Rust implementation:

1. ✅ Supports `is_license_tag` rule flag correctly
2. ✅ Has a working unknown/undetected matcher
3. ✅ Handles tag-style license detection via existing AHO matching

### Optional Minor Improvements

1. **Consider renaming** `MATCH_UNKNOWN` to `MATCH_UNDETECTED` and using `"6-unknown"` for clarity
2. **Consider splitting** into two matchers if exact Python parity is required:
   - `MATCH_UNDETECTED` (order 4) for utility detection
   - `MATCH_UNKNOWN` (order 6) for ngram-based unknown detection

These are cosmetic improvements, not feature gaps.

## References

- Python `match.py`: Lines 1959-1960, 2660-2688
- Python `detection.py`: Lines 77-78, 1218-1237, 1614-1661
- Python `match_unknown.py`: Lines 46-47
- Python `match_aho.py`: Lines 78-81
- Python `models.py`: Lines 1398-1408
- Rust `unknown_match.rs`: Lines 62-67
- Rust `models.rs`: Line 52
