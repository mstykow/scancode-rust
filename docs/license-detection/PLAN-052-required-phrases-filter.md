# PLAN-052: Implement filter_matches_missing_required_phrases

## Status: NOT IMPLEMENTED

## Summary

Python has a filter that removes matches where required phrases (marked with `{{...}}` in rule text) weren't matched. Rust does not have this filter. This is a significant missing piece that affects match quality.

---

## Problem Statement

**Python** (match.py:2154-2328):

The `filter_matches_missing_required_phrases()` function removes matches that:
1. Have required phrases defined in rules that weren't matched
2. Are marked `is_continuous` but have gaps/unknowns/stopwords
3. Have required phrase spans containing unknown words
4. Have stopword count mismatches between rule and query

This filter is called **FIRST** in Python's refine pipeline.

**Rust**: Does NOT have this filter at all.

---

## Gap Analysis

| Component | Field/Method | Rust Status | Gap |
|-----------|--------------|-------------|-----|
| **Rule** | `is_continuous` | ✅ Present | None |
| **Rule** | `is_required_phrase` | ✅ Present | None |
| **Rule** | `required_phrase_spans` | ❌ Missing | Need to parse `{{...}}` |
| **Rule** | `stopwords_by_pos` | ❌ Missing | Need to add |
| **LicenseMatch** | `qspan`/`ispan` | ⚠️ Partial | Have start/end_token, not full spans |
| **LicenseMatch** | `is_continuous()` method | ❌ Missing | Need to implement |
| **Query** | `stopwords_by_pos` | ✅ Present | None |
| **Query** | `unknowns_by_pos` | ✅ Present | None |

---

## Implementation Phases

### Phase 1: Rule Struct Changes

Add fields to `Rule` struct in `models.rs`:
- `required_phrase_spans: Vec<Range<usize>>`
- `stopwords_by_pos: HashMap<usize, usize>`

### Phase 2: Required Phrase Parsing

Parse `{{...}}` markers from rule text in `rules/loader.rs`:
- Create `parse_required_phrase_spans()` function
- Integrate into rule loading

### Phase 3: LicenseMatch Methods

Add `is_continuous()` method to `LicenseMatch`:
- Check if all matched tokens are continuous without gaps
- Check for unknown tokens in range

### Phase 4: Filter Implementation

Implement `filter_matches_missing_required_phrases()` in `match_refine.rs`:
- Handle solo match exception
- Check is_continuous/is_required_phrase rules
- Validate required phrase containment
- Check unknown words and stopwords

### Phase 5: Pipeline Integration

Call filter FIRST in `refine_matches()` pipeline (before other filters).

---

## Complexity

This is a **complex** feature requiring:
- Rule parsing changes
- New struct fields
- Position tracking fixes
- Filter implementation

**Estimated Effort**: 13+ hours

---

## Known Bugs from Previous Implementation Attempts

1. **`is_continuous()` was wrong**: Just checked if positions were stored, not actual continuity
2. **Missing `rule_start_token`**: Need to store where in the rule the match starts
3. **Wrong `ispan()`**: Returned query-side positions instead of rule-side
4. **Incorrect reinstatement logic**: Python debug log was mistaken for actual reinstatement
5. **Wrong pipeline order**: Merge should happen BEFORE required phrases filter
6. **Merge doesn't update positions**: `qspan_positions`/`ispan_positions` not computed during merge

---

## Priority: MEDIUM

Important for correctness but complex to implement correctly. Lower priority than quick wins like PLAN-049 and PLAN-050.

---

## Reference

- PLAN-017: Issue 6 - Original detailed plan with code samples
- Python reference: `licensedcode/match.py:2154-2328`
