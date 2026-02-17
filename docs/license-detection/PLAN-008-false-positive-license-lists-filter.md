# PLAN-008: False Positive License Lists Filter Implementation

**Status**: Implemented but Ineffective  
**Priority**: High  
**Estimated Effort**: Medium (2-3 days)  
**Related**: Python reference at `reference/scancode-toolkit/src/licensedcode/match.py:2408-2648`

---

## Investigation Results (Feb 17, 2026)

### Summary

PLAN-007 and PLAN-008 have been correctly implemented, but they are **not the root cause** of the 117 failing golden tests. The implementations are functionally correct for their designed purposes, but most test failures are caused by **different issues**.

### What Was Verified

1. **Boolean fields ARE correctly populated**:
   - `LicenseMatch` struct has all four flag fields: `is_license_reference`, `is_license_tag`, `is_license_intro`, `is_license_clue` (`src/license_detection/models.rs:219-229`)
   - All matchers propagate these fields from rules:
     - `aho_match.rs:172-175`
     - `hash_match.rs:116-119`
     - `seq_match.rs:505-508`
     - `spdx_lid.rs:288-291`
     - `unknown_match.rs:306-309`
   - `RuleFrontmatter` parses these fields from YAML (`src/license_detection/rules/loader.rs:162-166`)
   - Rule files have flags set (e.g., `spdx_license_id_borceux_for_borceux.RULE` has `is_license_reference: yes`)

2. **The filter IS being called**:
   - `filter_false_positive_license_lists_matches()` is called in `refine_matches()` at `match_refine.rs:671`
   - The function implementation is correct (`match_refine.rs:541-621`)
   - Unit tests for `is_candidate_false_positive()` pass

3. **PLAN-007's boolean flag fix is correct**:
   - `is_license_intro_match()` at `detection.rs:229-233` uses `match_item.is_license_intro` and `match_item.is_license_clue`
   - `is_license_clue_match()` at `detection.rs:242-244` uses `match_item.is_license_clue`

### Why PLAN-007/PLAN-008 Don't Fix Most Failures

**PLAN-008 targets the WRONG problem**:

The filter is designed for files with 15+ tag/reference matches forming a "license identifier list". But the test failures are NOT license list false positives:

| Test File | Expected | Actual | Root Cause |
|-----------|----------|--------|------------|
| `gpl-2.0-plus_11.txt` | `["gpl-2.0-plus"]` | `["gpl-2.0-plus", "borceux"]` | Single spurious match, not a list |
| `gpl-2.0-plus_17.txt` | `["gpl-2.0-plus"]` | `["gpl-2.0-plus", "allegro-4 AND bsd-new AND bsd-new"]` | Spurious match from short rule |
| `gpl_18.txt` | `["gpl-1.0-plus"]` | `["gpl-1.0-plus", "borceux"]` | Single spurious match |

The `filter_false_positive_license_lists_matches()` only activates when:

- There are 15+ matches total (line 546-548 in match_refine.rs)
- Or for long lists (150+ matches) with 95% candidates

But the failures have 1-2 extra matches, not 15+ candidate matches.

### Root Causes of Test Failures

1. **Short SPDX license ID rules match random words**:
   - Rules like `spdx_license_id_borceux_for_borceux.RULE` have `relevance: 50` and match bare license identifiers
   - These match random words in code that happen to be license identifiers
   - The PLAN-008 filter doesn't catch these because there's only 1 spurious match, not 15+

2. **Expression combination issues** (PLAN-010, PLAN-012):
   - Parentheses handling differs from Python
   - Example: `epl-2.0 OR apache-2.0 OR ((gpl-2.0 WITH classpath-exception-2.0) AND (gpl-2.0 WITH openjdk-exception))` vs expected without extra parens

3. **Match grouping/deduplication issues** (PLAN-011, PLAN-013):
   - Multiple matches being incorrectly combined
   - Detection grouping threshold differences

4. **Missing detections**:
   - Some tests expect matches that aren't being detected at all (e.g., `ecos-2.0_spdx.c`, `exif_not_lgpl3.txt`)

### Recommended Next Steps

1. **PLAN-008 is complete** - The filter is implemented correctly for its purpose
2. **Focus on other PLANs** for the remaining failures:
   - PLAN-010: Expression simplification
   - PLAN-011: Detection deduplication
   - PLAN-012: Expression parentheses
   - PLAN-013: Match grouping thresholds
   - PLAN-014: License containment check
3. **Consider a new plan** for filtering single spurious tag/reference matches with low relevance (e.g., filter matches where `is_license_reference=true AND matched_length<=3 AND rule_relevance<60` when not in a list)

---

## Deep Dive Analysis (Feb 17, 2026)

### Test Case Analysis: `gpl-2.0-plus_11.txt`

**Expected**: `["gpl-2.0-plus"]`

**Actual**: `["gpl-2.0-plus", "borceux"]`

**File Content**: Linux kernel header file with GPL-2.0+ license notice at lines 7-19.

**Spurious Match Details**:
```json
{
  "license_expression": "borceux",
  "start_line": 188,
  "end_line": 188,
  "matcher": "2-aho",
  "matched_length": 1,
  "match_coverage": 100.0,
  "rule_relevance": 50,
  "rule_identifier": "#34598"
}
```

**Why the borceux rule matches**:
- Rule `spdx_license_id_borceux_for_borceux.RULE` has:
  - `is_license_reference: yes`
  - `relevance: 50`
  - Text: just the word "borceux"
- The rule matches a word in the source code that happens to look like "borceux" (possibly "printk" or similar being tokenized)

**Why it's not filtered**:

1. **`filter_false_positive_license_lists_matches` threshold too high**:
   - Filter requires 15+ matches to activate (`MIN_SHORT_FP_LIST_LENGTH = 15`)
   - This file has only 2 matches total
   - The filter early-exits at line 541-543: `if len_matches < MIN_SHORT_FP_LIST_LENGTH { return (matches, vec![]); }`

2. **`is_false_positive()` in detection.rs missing `is_license_reference` check**:
   - Python's `is_false_positive()` checks `is_license_tag` at line 1236:
     ```python
     if matches_is_license_tag_flags and all_match_rule_length_one:
         return True
     ```
   - But it does NOT check `is_license_reference`
   - Rust's `is_false_positive()` only checks `is_license_tag` at lines 415-418:
     ```rust
     if all_is_license_tag && all_rule_length_one {
         return true;
     }
     ```
   - The borceux match has `is_license_reference=true` (not `is_license_tag`)

3. **The borceux match characteristics**:
   - `is_license_reference: true` ✓
   - `rule_relevance: 50` (< 60, low relevance) ✓
   - `matched_length: 1` (rule length 1) ✓
   - `match_coverage: 100.0` ✓
   - But NOT: `is_license_tag`, `is_bare_rule`, or `is_gpl`
   - So none of the existing false positive checks catch it!

### Threshold Analysis

The `MIN_SHORT_FP_LIST_LENGTH = 15` threshold is designed for files like:
- SPDX license list JSON files
- Package manager license selection code
- License detection library test data

But the failing tests have small numbers of spurious matches (1-2), not 15+ candidate matches.

### The Missing Check

**Both Python and Rust are missing a check for `is_license_reference`** in `is_false_positive()`:

```python
# Python detection.py:1236 - only checks is_license_tag
if matches_is_license_tag_flags and all_match_rule_length_one:
    return True
```

```rust
// Rust detection.rs:415-418 - only checks is_license_tag
if all_is_license_tag && all_rule_length_one {
    return true;
}
```

**However**, Python must be filtering these somewhere else (possibly in match refinement or a different detection path), as the golden tests show Python produces only `["gpl-2.0-plus"]`.

---

## 1. Problem Statement

Approximately 15 golden tests are currently failing because spurious license matches from license tag/reference/intro/clue rules are not being filtered out. These matches occur when the scanner processes files containing lists of license identifiers (e.g., SPDX license lists in tools like `spdx-license-list`, package manager code, or license detection libraries).

**Example scenario**: A file containing a list of SPDX license identifiers like:

```json
["MIT", "Apache-2.0", "GPL-3.0", "BSD-3-Clause", ...]
```

Each identifier matches a license tag or reference rule, producing dozens of false positive matches. The Rust implementation currently lacks the `filter_false_positive_license_lists_matches` function that the Python implementation uses to detect and filter these patterns.

**Why this matters**:

- Users scanning license-related tools or data files get incorrect results
- The noise-to-signal ratio is unacceptably high for certain file types
- Feature parity with Python ScanCode Toolkit is incomplete

## 2. Python Reference Analysis

### 2.1 Main Function: `filter_false_positive_license_lists_matches`

Location: `reference/scancode-toolkit/src/licensedcode/match.py:2408-2539`

```python
# Constants (lines 2400-2405)
MIN_SHORT_FP_LIST_LENGTH = 15
MIN_UNIQUE_LICENSES_PROPORTION = 1 / 3
MIN_LONG_FP_LIST_LENGTH = 150

def filter_false_positive_license_lists_matches(
    matches,
    min_matches=MIN_SHORT_FP_LIST_LENGTH,
    min_matches_long=MIN_LONG_FP_LIST_LENGTH,
    min_unique_licenses_proportion=MIN_UNIQUE_LICENSES_PROPORTION,
    reason=DiscardReason.LICENSE_LIST,
    trace=TRACE_FILTER_LICENSE_LIST,
):
    """
    Return a filtered list of kept LicenseMatch matches and a list of
    discardable matches given a `matches` list of LicenseMatch by checking false
    positive status for matches to lists of licenses ids such as lists of SPDX
    license ids found in license-related tools code or data files.
    """
```

**Key logic**:

1. **Early exit** if fewer than `min_matches` (15) matches exist
2. **Fast path for long lists** (>150 matches): If 95% are candidates, discard all
3. **Detailed procedure** for medium lists (15-150 matches):
   - Group candidate matches by proximity (within 10 lines)
   - For each group, determine if it's a false positive list using `is_list_of_false_positives()`
   - Keep or discard the group based on the result

### 2.2 Candidate Detection: `is_candidate_false_positive`

Location: `reference/scancode-toolkit/src/licensedcode/match.py:2651-2688`

```python
def is_candidate_false_positive(
    match,
    max_length=20,
    trace=TRACE_FILTER_LICENSE_LIST_DETAILED,
):
    """
    Return True if the ``match`` LicenseMatch is a candidate false positive
    license list match.
    """
    is_candidate = (
        # only tags, refs, or clues
        (
            match.rule.is_license_reference
            or match.rule.is_license_tag
            or match.rule.is_license_intro
            or match.rule.is_license_clue
        )
        # but not tags that are SPDX license identifiers
        and not match.matcher == '1-spdx-id'
        # exact matches only
        and match.coverage() == 100

        # not too long
        and match.len() <= max_length
    )
    return is_candidate
```

**Conditions for being a candidate false positive**:

1. Rule is one of: `is_license_reference`, `is_license_tag`, `is_license_intro`, or `is_license_clue`
2. Matcher is NOT `'1-spdx-id'` (explicit SPDX ID matches are legitimate)
3. Coverage is exactly 100% (exact matches only)
4. Match length (in tokens) is <= 20

### 2.3 List Detection: `is_list_of_false_positives`

Location: `reference/scancode-toolkit/src/licensedcode/match.py:2556-2648`

```python
def is_list_of_false_positives(
    matches,
    min_matches=MIN_SHORT_FP_LIST_LENGTH,
    min_unique_licenses=MIN_UNIQUE_LICENSES,
    min_unique_licenses_proportion=MIN_UNIQUE_LICENSES_PROPORTION,
    min_candidate_proportion=0,
    trace=TRACE_FILTER_LICENSE_LIST,
):
    """
    Return True if all LicenseMatch in the ``matches`` list form a proper false
    positive license list sequence.
    """
    len_matches = len(matches)

    is_long_enough_sequence = len_matches >= min_matches

    len_unique_licenses = count_unique_licenses(matches)
    has_enough_licenses = (
        len_unique_licenses / len_matches > min_unique_licenses_proportion
    )

    if not has_enough_licenses:
        has_enough_licenses = len_unique_licenses >= min_unique_licenses

    has_enough_candidates = True
    if min_candidate_proportion:
        candidates_count = len([
            m for m in matches
            if is_candidate_false_positive(m)
        ])
        has_enough_candidates = (
            (candidates_count / len_matches)
            > min_candidate_proportion
        )

    is_fp = (
        is_long_enough_sequence
        and has_enough_licenses
        and has_enough_candidates
    )

    return is_fp
```

**Conditions for a list being false positives**:

1. At least `min_matches` (default 15) matches
2. Either:
   - Proportion of unique license expressions > 1/3, OR
   - At least `min_unique_licenses` unique license expressions
3. (Optional) Proportion of candidate matches exceeds threshold

### 2.4 Distance Calculation: `qdistance_to`

Location: `reference/scancode-toolkit/src/licensedcode/match.py:450-456`

```python
def qdistance_to(self, other):
    """
    Return the absolute qspan distance to other match.
    Overlapping matches have a zero distance.
    Non-overlapping touching matches have a distance of one.
    """
    return self.qspan.distance_to(other.qspan)
```

The `distance_to` method on spans (from `spans.py:402-435`):

- Returns 0 if spans overlap
- Returns 1 if spans touch (adjacent)
- Otherwise returns the gap between them

### 2.5 Integration Point in `refine_matches`

Location: `reference/scancode-toolkit/src/licensedcode/match.py:2809-2817`

```python
if filter_false_positive:
    matches, discarded = filter_false_positive_matches(matches)
    all_discarded_extend(discarded)
    _log(matches, discarded, 'TRUE POSITIVE')

    # license listings are false positive-like
    matches, discarded = filter_false_positive_license_lists_matches(matches)
    all_discarded_extend(discarded)
    _log(matches, discarded, 'NOT A LICENSE LIST')
```

Called **after** `filter_false_positive_matches` but **before** score filtering and final merge.

## 3. Rust Code Analysis

### 3.1 Current Match Refinement Pipeline

Location: `src/license_detection/match_refine.rs:482-510`

```rust
pub fn refine_matches(
    index: &LicenseIndex,
    matches: Vec<LicenseMatch>,
    _query: &Query,
) -> Vec<LicenseMatch> {
    if matches.is_empty() {
        return Vec::new();
    }

    let filtered = filter_short_gpl_matches(&matches);
    let merged = merge_overlapping_matches(&filtered);
    let non_contained = filter_contained_matches(&merged);
    let (kept, discarded) = filter_overlapping_matches(non_contained, index);
    let (restored, _) = restore_non_overlapping(&kept, discarded);

    let mut final_matches = kept;
    final_matches.extend(restored);

    let non_fp = filter_false_positive_matches(index, &final_matches);

    let mut scored = non_fp;
    update_match_scores(&mut scored);

    scored
}
```

**Current filters present**:

- `filter_short_gpl_matches` - Filters GPL matches with very short matched_length
- `filter_false_positive_matches` - Filters matches by rule ID in false_positive_rids set
- `filter_contained_matches` - Filters matches contained within other matches
- `filter_overlapping_matches` - Filters overlapping matches

**Missing**: `filter_false_positive_license_lists_matches`

### 3.2 LicenseMatch Structure

Location: `src/license_detection/models.rs:174-224`

```rust
pub struct LicenseMatch {
    pub license_expression: String,
    pub license_expression_spdx: String,
    pub from_file: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub matcher: String,
    pub score: f32,
    pub matched_length: usize,  // Token count (equivalent to Python's match.len())
    pub match_coverage: f32,    // Percentage 0-100 (equivalent to Python's coverage())
    pub rule_relevance: u8,
    pub rule_identifier: String,
    pub rule_url: String,
    pub matched_text: Option<String>,
    pub referenced_filenames: Option<Vec<String>>,
    pub is_license_intro: bool,
    pub is_license_clue: bool,
}
```

**Missing fields needed for filtering**:

- `is_license_reference: bool` - Not present on LicenseMatch
- `is_license_tag: bool` - Not present on LicenseMatch

**Note**: These fields exist on the `Rule` struct but are NOT propagated to `LicenseMatch`. The current implementation only propagates `is_license_intro` and `is_license_clue`.

### 3.3 Rule Structure

Location: `src/license_detection/models.rs:58-171`

```rust
pub struct Rule {
    // ... other fields ...
    pub is_license_text: bool,
    pub is_license_notice: bool,
    pub is_license_reference: bool,  // ✓ Present
    pub is_license_tag: bool,        // ✓ Present
    pub is_license_intro: bool,
    pub is_license_clue: bool,
    pub is_false_positive: bool,
    // ...
}
```

The `Rule` struct has all needed flags. They just need to be propagated to `LicenseMatch`.

### 3.4 Where Matches Are Created

Matches are created in multiple matchers:

1. **aho_match.rs:157-174** - Aho-Corasick matches
2. **hash_match.rs:141-180** - Hash-based exact matches
3. **seq_match.rs:505-506** - Sequence matches (already propagates intro/clue)
4. **unknown_match.rs:306-307** - Unknown matches

## 4. Proposed Changes

### 4.1 Add Missing Fields to LicenseMatch

**File**: `src/license_detection/models.rs`

Add two new fields to `LicenseMatch`:

```rust
pub struct LicenseMatch {
    // ... existing fields ...
    pub is_license_intro: bool,
    pub is_license_clue: bool,
    
    // ADD THESE TWO FIELDS:
    pub is_license_reference: bool,
    pub is_license_tag: bool,
}
```

### 4.2 Update All Match Creation Sites

Update the following files to propagate the new fields:

**File**: `src/license_detection/aho_match.rs` (line ~172)

```rust
let license_match = LicenseMatch {
    // ... existing fields ...
    is_license_intro: rule.is_license_intro,
    is_license_clue: rule.is_license_clue,
    is_license_reference: rule.is_license_reference,  // ADD
    is_license_tag: rule.is_license_tag,              // ADD
};
```

**File**: `src/license_detection/hash_match.rs` (lines ~141-142, ~179-180)

```rust
// Add is_license_reference and is_license_tag to both match creation sites
```

**File**: `src/license_detection/seq_match.rs` (line ~505)

```rust
is_license_intro: candidate.rule.is_license_intro,
is_license_clue: candidate.rule.is_license_clue,
is_license_reference: candidate.rule.is_license_reference,  // ADD
is_license_tag: candidate.rule.is_license_tag,              // ADD
```

**File**: `src/license_detection/unknown_match.rs` (line ~306)

```rust
is_license_intro: false,
is_license_clue: false,
is_license_reference: false,  // ADD
is_license_tag: false,        // ADD
```

### 4.3 Implement `filter_false_positive_license_lists_matches`

**File**: `src/license_detection/match_refine.rs`

Add the following constants and functions:

```rust
// Constants for license list false positive detection
const MIN_SHORT_FP_LIST_LENGTH: usize = 15;
const MIN_LONG_FP_LIST_LENGTH: usize = 150;
const MIN_UNIQUE_LICENSES_PROPORTION: f64 = 1.0 / 3.0;
const MAX_CANDIDATE_LENGTH: usize = 20;
const MAX_DISTANCE_BETWEEN_CANDIDATES: usize = 10;

/// Check if a match is a candidate false positive for license list filtering.
///
/// A candidate is a match from a tag/reference/intro/clue rule that:
/// - Is NOT an explicit SPDX ID match (matcher != "1-spdx-id")
/// - Has 100% coverage
/// - Has matched_length <= MAX_CANDIDATE_LENGTH
fn is_candidate_false_positive(m: &LicenseMatch) -> bool {
    let is_tag_or_ref = m.is_license_reference
        || m.is_license_tag
        || m.is_license_intro
        || m.is_license_clue;
    
    let is_not_spdx_id = m.matcher != "1-spdx-id";
    let is_exact_match = (m.match_coverage - 100.0).abs() < f32::EPSILON;
    let is_short = m.matched_length <= MAX_CANDIDATE_LENGTH;
    
    is_tag_or_ref && is_not_spdx_id && is_exact_match && is_short
}

/// Count unique license expressions in a slice of matches.
fn count_unique_licenses(matches: &[LicenseMatch]) -> usize {
    let mut seen = std::collections::HashSet::new();
    for m in matches {
        seen.insert(&m.license_expression);
    }
    seen.len()
}

/// Check if a list of matches qualifies as false positive license list.
fn is_list_of_false_positives(
    matches: &[LicenseMatch],
    min_matches: usize,
    min_unique_licenses_proportion: f64,
    min_candidate_proportion: f64,
) -> bool {
    if matches.len() < min_matches {
        return false;
    }
    
    let len_matches = matches.len();
    let len_unique_licenses = count_unique_licenses(matches);
    
    // Check unique licenses proportion
    let unique_proportion = len_unique_licenses as f64 / len_matches as f64;
    let has_enough_licenses = unique_proportion > min_unique_licenses_proportion
        || len_unique_licenses >= min_matches / 3;
    
    // Check candidate proportion if threshold > 0
    let has_enough_candidates = if min_candidate_proportion > 0.0 {
        let candidates_count = matches.iter()
            .filter(|m| is_candidate_false_positive(m))
            .count();
        (candidates_count as f64 / len_matches as f64) > min_candidate_proportion
    } else {
        true
    };
    
    has_enough_licenses && has_enough_candidates
}

/// Calculate distance between two matches (in lines).
///
/// Returns 0 if overlapping, 1 if touching, otherwise the gap.
fn match_distance(a: &LicenseMatch, b: &LicenseMatch) -> usize {
    // Check overlap
    if a.start_line <= b.end_line && b.start_line <= a.end_line {
        return 0;
    }
    
    // Check touching
    if a.end_line + 1 == b.start_line || b.end_line + 1 == a.start_line {
        return 1;
    }
    
    // Calculate gap
    if a.end_line < b.start_line {
        b.start_line - a.end_line
    } else {
        a.start_line - b.end_line
    }
}

/// Filter false positive license list matches.
///
/// This function detects and removes matches that are likely from license
/// identifier lists (e.g., SPDX license lists in tools or data files).
///
/// Returns (kept_matches, discarded_matches).
pub fn filter_false_positive_license_lists_matches(
    matches: Vec<LicenseMatch>,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    let len_matches = matches.len();
    
    // Early exit if not enough matches
    if len_matches < MIN_SHORT_FP_LIST_LENGTH {
        return (matches, vec![]);
    }
    
    // Fast path for long lists: if 95% are candidates, discard all
    if len_matches > MIN_LONG_FP_LIST_LENGTH {
        if is_list_of_false_positives(
            &matches,
            MIN_LONG_FP_LIST_LENGTH,
            MIN_UNIQUE_LICENSES_PROPORTION,
            0.95,
        ) {
            return (vec![], matches);
        }
    }
    
    // Detailed procedure: identify sub-sequences of false positives
    let mut kept = Vec::new();
    let mut discarded = Vec::new();
    let mut candidates: Vec<&LicenseMatch> = Vec::new();
    
    for match_item in &matches {
        let is_candidate = is_candidate_false_positive(match_item);
        
        if is_candidate {
            // Check if close enough to existing candidates
            let is_close_enough = candidates.last()
                .map(|last| match_distance(last, match_item) <= MAX_DISTANCE_BETWEEN_CANDIDATES)
                .unwrap_or(true);
            
            if is_close_enough {
                candidates.push(match_item);
            } else {
                // Process accumulated candidates
                let owned: Vec<LicenseMatch> = candidates.iter().map(|m| (*m).clone()).collect();
                if is_list_of_false_positives(
                    &owned,
                    MIN_SHORT_FP_LIST_LENGTH,
                    MIN_UNIQUE_LICENSES_PROPORTION,
                    0.0,
                ) {
                    discarded.extend(owned);
                } else {
                    kept.extend(owned);
                }
                candidates.clear();
                candidates.push(match_item);
            }
        } else {
            // Not a candidate - process accumulated and keep current
            let owned: Vec<LicenseMatch> = candidates.iter().map(|m| (*m).clone()).collect();
            if is_list_of_false_positives(
                &owned,
                MIN_SHORT_FP_LIST_LENGTH,
                MIN_UNIQUE_LICENSES_PROPORTION,
                0.0,
            ) {
                discarded.extend(owned);
            } else {
                kept.extend(owned);
            }
            candidates.clear();
            kept.push(match_item.clone());
        }
    }
    
    // Process any remaining candidates
    let owned: Vec<LicenseMatch> = candidates.iter().map(|m| (*m).clone()).collect();
    if is_list_of_false_positives(
        &owned,
        MIN_SHORT_FP_LIST_LENGTH,
        MIN_UNIQUE_LICENSES_PROPORTION,
        0.0,
    ) {
        discarded.extend(owned);
    } else {
        kept.extend(owned);
    }
    
    (kept, discarded)
}
```

### 4.4 Update `refine_matches` to Use New Filter

**File**: `src/license_detection/match_refine.rs`

Modify the `refine_matches` function:

```rust
pub fn refine_matches(
    index: &LicenseIndex,
    matches: Vec<LicenseMatch>,
    _query: &Query,
) -> Vec<LicenseMatch> {
    if matches.is_empty() {
        return Vec::new();
    }

    let filtered = filter_short_gpl_matches(&matches);
    let merged = merge_overlapping_matches(&filtered);
    let non_contained = filter_contained_matches(&merged);
    let (kept, discarded) = filter_overlapping_matches(non_contained, index);
    let (restored, _) = restore_non_overlapping(&kept, discarded);

    let mut final_matches = kept;
    final_matches.extend(restored);

    let non_fp = filter_false_positive_matches(index, &final_matches);
    
    // ADD THIS: Filter false positive license list matches
    let (kept, _discarded) = filter_false_positive_license_lists_matches(non_fp);

    let mut scored = kept;
    update_match_scores(&mut scored);

    scored
}
```

### 4.5 Update Test Utilities

**File**: `src/license_detection/test_utils.rs`

Update test match creation to include new fields:

```rust
// In create_mock_rule() and any test match helpers
is_license_reference: false,
is_license_tag: false,
```

### 4.6 Update Existing Tests

Update all test match creations that manually construct `LicenseMatch` to include the new fields. This affects tests in:

- `src/license_detection/match_refine.rs` (test module)
- `src/license_detection/detection.rs` (test module)
- `src/license_detection/seq_match.rs` (test module)

## 5. Testing Strategy

### 5.1 Unit Tests

Add unit tests to `src/license_detection/match_refine.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_match_with_flags(
        rule_identifier: &str,
        start_line: usize,
        end_line: usize,
        is_license_reference: bool,
        is_license_tag: bool,
        is_license_intro: bool,
        is_license_clue: bool,
        matcher: &str,
        match_coverage: f32,
        matched_length: usize,
        license_expression: &str,
    ) -> LicenseMatch {
        LicenseMatch {
            license_expression: license_expression.to_string(),
            license_expression_spdx: license_expression.to_string(),
            from_file: None,
            start_line,
            end_line,
            matcher: matcher.to_string(),
            score: 1.0,
            matched_length,
            match_coverage,
            rule_relevance: 100,
            rule_identifier: rule_identifier.to_string(),
            rule_url: String::new(),
            matched_text: None,
            referenced_filenames: None,
            is_license_intro,
            is_license_clue,
            is_license_reference,
            is_license_tag,
        }
    }

    #[test]
    fn test_is_candidate_false_positive_tag_match() {
        let m = create_test_match_with_flags(
            "#1", 1, 1, 
            false, true, false, false,  // is_license_tag = true
            "2-aho", 100.0, 5, "mit"
        );
        assert!(is_candidate_false_positive(&m));
    }

    #[test]
    fn test_is_candidate_false_positive_reference_match() {
        let m = create_test_match_with_flags(
            "#2", 1, 1,
            true, false, false, false,  // is_license_reference = true
            "2-aho", 100.0, 3, "apache-2.0"
        );
        assert!(is_candidate_false_positive(&m));
    }

    #[test]
    fn test_is_candidate_false_positive_spdx_id_excluded() {
        let m = create_test_match_with_flags(
            "#3", 1, 1,
            true, false, false, false,
            "1-spdx-id",  // SPDX ID matcher - should be excluded
            100.0, 3, "mit"
        );
        assert!(!is_candidate_false_positive(&m));
    }

    #[test]
    fn test_is_candidate_false_partial_coverage_excluded() {
        let m = create_test_match_with_flags(
            "#4", 1, 1,
            true, false, false, false,
            "2-aho",
            80.0,  // Not 100% coverage
            5, "mit"
        );
        assert!(!is_candidate_false_positive(&m));
    }

    #[test]
    fn test_is_candidate_false_long_match_excluded() {
        let m = create_test_match_with_flags(
            "#5", 1, 1,
            true, false, false, false,
            "2-aho",
            100.0,
            25,  // > MAX_CANDIDATE_LENGTH (20)
            "mit"
        );
        assert!(!is_candidate_false_positive(&m));
    }

    #[test]
    fn test_filter_short_list_not_filtered() {
        // Less than MIN_SHORT_FP_LIST_LENGTH matches
        let matches: Vec<LicenseMatch> = (0..10)
            .map(|i| create_test_match_with_flags(
                &format!("#{}", i), i + 1, i + 1,
                true, false, false, false,
                "2-aho", 100.0, 3, &format!("license-{}", i)
            ))
            .collect();
        
        let (kept, discarded) = filter_false_positive_license_lists_matches(matches);
        assert_eq!(kept.len(), 10);
        assert_eq!(discarded.len(), 0);
    }

    #[test]
    fn test_filter_long_list_all_candidates() {
        // MIN_LONG_FP_LIST_LENGTH + matches, all unique, all candidates
        let matches: Vec<LicenseMatch> = (0..160)
            .map(|i| create_test_match_with_flags(
                &format!("#{}", i), i + 1, i + 1,
                true, false, false, false,
                "2-aho", 100.0, 3, &format!("license-{}", i)
            ))
            .collect();
        
        let (kept, discarded) = filter_false_positive_license_lists_matches(matches);
        assert_eq!(kept.len(), 0);
        assert_eq!(discarded.len(), 160);
    }

    #[test]
    fn test_filter_mixed_list_keeps_non_candidates() {
        // Mix of candidates and non-candidates
        let mut matches = Vec::new();
        
        // Add 15 candidate matches (a license list)
        for i in 0..15 {
            matches.push(create_test_match_with_flags(
                &format!("#{}", i), i + 1, i + 1,
                true, false, false, false,
                "2-aho", 100.0, 3, &format!("license-{}", i)
            ));
        }
        
        // Add 5 non-candidate matches (real license text)
        for i in 0..5 {
            matches.push(create_test_match_with_flags(
                &format!("#{}", 100 + i), 100 + i, 100 + i + 20,
                false, false, false, false,  // Not a tag/ref/intro/clue
                "2-aho", 100.0, 100, "gpl-3.0"
            ));
        }
        
        let (kept, discarded) = filter_false_positive_license_lists_matches(matches);
        
        // The 15 candidates should be discarded
        // The 5 non-candidates should be kept
        assert_eq!(kept.len(), 5);
        assert_eq!(discarded.len(), 15);
    }

    #[test]
    fn test_filter_candidates_with_real_license() {
        // Candidates scattered with a real license in between
        let mut matches = Vec::new();
        
        // First group of candidates (should be discarded)
        for i in 0..15 {
            matches.push(create_test_match_with_flags(
                &format!("#{}", i), i + 1, i + 1,
                true, false, false, false,
                "2-aho", 100.0, 3, &format!("license-{}", i)
            ));
        }
        
        // Real license match (should be kept)
        matches.push(create_test_match_with_flags(
            "#real", 100, 150,
            false, false, false, false,
            "2-aho", 100.0, 200, "mit"
        ));
        
        // Second group of candidates (should be discarded)
        for i in 0..15 {
            matches.push(create_test_match_with_flags(
                &format!("#{}", 200 + i), 200 + i, 200 + i,
                true, false, false, false,
                "2-aho", 100.0, 3, &format!("license-{}", 200 + i)
            ));
        }
        
        let (kept, discarded) = filter_false_positive_license_lists_matches(matches);
        
        assert_eq!(kept.len(), 1);  // Only the real license
        assert_eq!(discarded.len(), 30);  // Both candidate groups
    }

    #[test]
    fn test_match_distance_overlapping() {
        let a = create_test_match_with_flags("#1", 1, 10, false, false, false, false, "2-aho", 100.0, 10, "mit");
        let b = create_test_match_with_flags("#2", 5, 15, false, false, false, false, "2-aho", 100.0, 10, "mit");
        assert_eq!(match_distance(&a, &b), 0);
    }

    #[test]
    fn test_match_distance_touching() {
        let a = create_test_match_with_flags("#1", 1, 10, false, false, false, false, "2-aho", 100.0, 10, "mit");
        let b = create_test_match_with_flags("#2", 11, 20, false, false, false, false, "2-aho", 100.0, 10, "mit");
        assert_eq!(match_distance(&a, &b), 1);
    }

    #[test]
    fn test_match_distance_gap() {
        let a = create_test_match_with_flags("#1", 1, 10, false, false, false, false, "2-aho", 100.0, 10, "mit");
        let b = create_test_match_with_flags("#2", 15, 25, false, false, false, false, "2-aho", 100.0, 10, "mit");
        assert_eq!(match_distance(&a, &b), 4);
    }

    #[test]
    fn test_count_unique_licenses() {
        let matches = vec![
            create_test_match_with_flags("#1", 1, 1, false, false, false, false, "2-aho", 100.0, 5, "mit"),
            create_test_match_with_flags("#2", 2, 2, false, false, false, false, "2-aho", 100.0, 5, "mit"),
            create_test_match_with_flags("#3", 3, 3, false, false, false, false, "2-aho", 100.0, 5, "apache-2.0"),
        ];
        assert_eq!(count_unique_licenses(&matches), 2);
    }
}
```

### 5.2 Golden Test Verification

Run the existing golden tests to verify the fix:

```bash
cargo test --test license_detection_golden_test
```

The tests that were previously failing due to spurious license list matches should now pass.

### 5.3 Integration Testing

Create a test file with a list of license identifiers and verify it produces no false positives:

```bash
# Create test file
cat > /tmp/license_list.json << 'EOF'
[
  "MIT",
  "Apache-2.0",
  "GPL-3.0",
  "BSD-3-Clause",
  "ISC",
  "MPL-2.0",
  "LGPL-2.1",
  "EPL-1.0",
  "CDDL-1.0",
  "Unlicense",
  "0BSD",
  "AFL-3.0",
  "AGPL-3.0",
  "Apache-1.0",
  "Artistic-2.0"
]
EOF

# Run scanner
cargo run -- /tmp/license_list.json -o /tmp/output.json

# Verify no (or minimal) license matches
```

## 6. Implementation Checklist

- [ ] Add `is_license_reference` and `is_license_tag` fields to `LicenseMatch`
- [ ] Update `aho_match.rs` to propagate new fields
- [ ] Update `hash_match.rs` to propagate new fields
- [ ] Update `seq_match.rs` to propagate new fields
- [ ] Update `unknown_match.rs` with new fields (set to false)
- [ ] Update `test_utils.rs` with new fields
- [ ] Implement `is_candidate_false_positive()` function
- [ ] Implement `count_unique_licenses()` function
- [ ] Implement `is_list_of_false_positives()` function
- [ ] Implement `match_distance()` function
- [ ] Implement `filter_false_positive_license_lists_matches()` function
- [ ] Update `refine_matches()` to call new filter
- [ ] Add comprehensive unit tests
- [ ] Update existing tests that construct `LicenseMatch`
- [ ] Run golden tests and verify fix
- [ ] Run `cargo clippy` and fix warnings
- [ ] Run `cargo fmt`

## 7. Files Modified

1. `src/license_detection/models.rs` - Add fields to `LicenseMatch`
2. `src/license_detection/match_refine.rs` - Main implementation
3. `src/license_detection/aho_match.rs` - Propagate fields
4. `src/license_detection/hash_match.rs` - Propagate fields
5. `src/license_detection/seq_match.rs` - Propagate fields
6. `src/license_detection/unknown_match.rs` - Add fields
7. `src/license_detection/test_utils.rs` - Update test helpers
8. `src/license_detection/detection.rs` - Update tests

## 8. References

- Python implementation: `reference/scancode-toolkit/src/licensedcode/match.py:2408-2648`
- Python span distance: `reference/scancode-toolkit/src/licensedcode/spans.py:402-435`
- Python refine_matches: `reference/scancode-toolkit/src/licensedcode/match.py:2691-2833`

---

## Remaining TODOs

### TODO 1: Add `is_license_reference` check to `is_false_positive()` in detection.rs

**Location**: `src/license_detection/detection.rs:359-421`

**Problem**: The `is_false_positive()` function only checks `is_license_tag` for single-token rule filtering, but not `is_license_reference`. The Python reference also only checks `is_license_tag`, so this may be a missing filter in both implementations.

**Proposed Fix**: Add check for `is_license_reference` matches with low relevance and short rule length:

```rust
// Check 5: License reference matches with length == 1 and low relevance
let all_is_license_reference = matches.iter().all(|m| m.is_license_reference);
if all_is_license_reference && all_rule_length_one && all_low_relevance {
    return true;
}
```

**Affected tests**: `gpl-2.0-plus_11.txt`, `gpl_18.txt`, `gpl_26.txt`, `gpl_35.txt`, `gpl_36.txt`, `gpl_40.txt`, `gpl_48.txt`, `gpl_57.txt`, `fsf-unlimited-no-warranty_with_line_numbers.pl`, `complex.el`

### TODO 2: Investigate why Python produces correct output

**Problem**: The Python reference also only checks `is_license_tag` in `is_false_positive()`, yet Python produces the correct output for these tests. There may be additional filtering happening in:
- `filter_false_positive_license_lists_matches()` being called with different parameters
- Detection grouping removing isolated low-relevance matches
- A different code path entirely

**Action**: Trace through Python execution on `gpl-2.0-plus_11.txt` to find where the borceux match is filtered.

### TODO 3: Lower `MIN_SHORT_FP_LIST_LENGTH` threshold (optional)

**Current value**: 15

**Problem**: Tests with 1-2 spurious matches are not filtered because threshold is too high.

**Risk**: Lowering threshold may cause false negatives on legitimate license lists.

**Recommendation**: Only do this after investigating TODO 2. The correct solution may be adding `is_license_reference` check (TODO 1) rather than changing threshold.

### TODO 4: Add unit test for single spurious `is_license_reference` match

Add test to `detection.rs`:

```rust
#[test]
fn test_is_false_positive_single_license_reference() {
    let matches = vec![LicenseMatch {
        license_expression: "borceux".to_string(),
        license_expression_spdx: "Borceux".to_string(),
        from_file: None,
        start_line: 188,
        end_line: 188,
        matcher: "2-aho".to_string(),
        score: 50.0,
        matched_length: 1,
        rule_length: 1,
        match_coverage: 100.0,
        rule_relevance: 50,
        rule_identifier: "spdx_license_id_borceux_for_borceux.RULE".to_string(),
        rule_url: String::new(),
        matched_text: None,
        referenced_filenames: None,
        is_license_intro: false,
        is_license_clue: false,
        is_license_reference: true,
        is_license_tag: false,
    }];
    // After TODO 1 fix, this should return true
    assert!(is_false_positive(&matches));
}
```
