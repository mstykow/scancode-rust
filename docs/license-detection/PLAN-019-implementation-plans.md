# PLAN-019: Implementation Plans for License Detection Alignment

This document contains detailed implementation plans for aligning Rust's license detection with Python's behavior.

---

## Part 1: `is_license_text` Field and Filtering Logic

### Status: ✅ COMPLETE

- Field added to `LicenseMatch`
- All creation sites updated
- Subtraction logic implemented after each matcher (timing corrected)

---

## Part 3: Filter Pipeline Alignment

### Status: ✅ MOSTLY COMPLETE

| Step | Function Call | Status |
|------|---------------|--------|
| 1 | `merge_matches()` | ✓ |
| 2 | `filter_matches_missing_required_phrases()` | ✓ |
| 3 | `filter_spurious_matches()` | ✓ |
| 4 | `filter_below_rule_minimum_coverage()` | ✓ |
| 5 | `filter_matches_to_spurious_single_token()` | ✓ |
| 6 | `filter_too_short_matches()` | ✓ |
| 7 | `filter_short_matches_scattered_on_too_many_lines()` | ✓ |
| 8 | `filter_invalid_matches_to_single_word_gibberish()` | ✓ |
| 9 | `merge_matches()` | ✓ |
| 10 | `filter_contained_matches()` | ✓ |
| 11 | `filter_overlapping_matches()` | ✓ |
| 12 | `restore_non_overlapping()` (first) | ✓ |
| 13 | `restore_non_overlapping()` (second) | ✓ |
| 14 | `filter_contained_matches()` (second) | ✓ |
| 15 | `filter_false_positive_matches()` | ✓ |
| 16 | `filter_false_positive_license_lists_matches()` | ✓ |
| 17 | `filter_matches_below_minimum_score()` | ❌ Not implemented |
| 18 | `merge_matches()` (final) | ✓ |

### Remaining Items

1. **`filter_matches_below_minimum_score()`** - Medium priority, only used when `min_score > 0`

2. **`licensing_contains()`** - Still uses length approximation instead of proper license expression containment

---

## Verification Results

All implemented changes verified correct:

- `is_license_text` subtraction timing: ✅ Matches Python
- Double `restore_non_overlapping()` calls: ✅ Matches Python
- Second `filter_contained_matches()` call: ✅ Matches Python
- Final `merge_matches()` call: ✅ Matches Python

---

## Golden Test Results

| Suite | Baseline | Current | Delta |
|-------|----------|---------|-------|
| lic1 | 228 | 224 | -4 |
| lic2 | 776 | 775 | -1 |
| lic3 | 251 | 250 | -1 |
| lic4 | 281 | 286 | +5 |
| external | 1882 | 2018 | +136 |
| unknown | 2 | 3 | +1 |

**Total: +136 tests vs baseline**
