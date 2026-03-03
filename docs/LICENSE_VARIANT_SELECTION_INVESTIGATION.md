# License Variant Selection Investigation Report

**Date**: 2026-03-03
**Purpose**: Investigate why Rust selects wrong license variants when text matches multiple similar licenses
**Status**: Investigation complete - ready for fix implementation

## Executive Summary

The root cause of license variant selection issues is **not a single bug, but multiple interacting problems** in the detection pipeline:

1. **Aho-Corasick exact matches take precedence over more specific licenses** (primary issue)
2. **Candidate ranking doesn't prioritize more specific licenses** 
3. **Required phrase filtering happens after Aho matches are already selected**
4. **Match merging doesn't account for license specificity**

## Investigated Cases

### Case 1: JSON.t2 - MIT vs JSON

**File**: `testdata/license-golden/datadriven/external/glc/JSON.t2`

**Expected**: `json`
**Actual**: `mit`

**Test File Content** (key excerpt):
```
The Software should rather be used for Good, not Evil.
```

**JSON License Rule** (`json_20.RULE`):
```
The Software {{shall be used for Good, not Evil.}}
```

**Root Cause Analysis**:

1. **Aho-Corasick matches MIT first**: 
   - `mit_17.RULE` matches lines 3-11 (MIT preamble text) with 100% coverage
   - This rule has `relevance: 80` and `minimum_coverage: 95`
   - MIT preamble text is a subset of JSON license text

2. **Required phrase mismatch**:
   - JSON has required phrase: `{{shall be used for Good, not Evil.}}`
   - Test file has: `should rather be used for Good, not Evil.`
   - Word "shall" → "should rather" modification prevents exact required phrase match

3. **Why Python detects JSON correctly**:
   - Python's sequence matcher finds `json.LICENSE` and `json_20.RULE` as top candidates (score 0.9)
   - Python's required phrase checking is more lenient OR happens at a different stage
   - Python appears to use the `minimum_coverage: 70` setting to accept partial matches

4. **Rust's bug**:
   - Aho-Corasick finds MIT exact match first
   - This bypasses sequence matching candidates (json.LICENSE ranked #1)
   - Required phrase filtering happens on the MIT match (which has no required phrases)
   - Result: MIT wins because it's found earlier in the pipeline

**Candidate Ranking**:
```
NEAR-DUPE CANDIDATES: 50
  1. json.LICENSE (score: 0.9000)      ← CORRECT LICENSE ranked #1
  2. json_20.RULE (score: 0.9000)
  3. proprietary-license_605.RULE (score: 0.9000)
  4. mit.LICENSE (score: 0.9000)        ← Wrong license ranked #4
```

**Debug Output**:
```
EXACT MATCHES: 2 (raw: 4)
  Rule: mit_17.RULE (license: mit)     ← MIT found by Aho first
  Score: 8000.0%, Coverage: 100.0%
  Lines: 3-11, Tokens: 3-89
```

---

### Case 2: CC-BY-NC-4.0.t1 - CC-BY-4.0 vs CC-BY-NC-4.0

**File**: `testdata/license-golden/datadriven/external/glc/CC-BY-NC-4.0.t1`

**Expected**: `cc-by-nc-4.0`
**Actual**: `cc-by-4.0`

**Root Cause Analysis**:

1. **License relationship**: CC-BY-4.0 is a more generic version of CC-BY-NC-4.0
   - CC-BY-NC-4.0 adds "NonCommercial" restrictions
   - Large portions of text are identical

2. **Candidate ranking issue**:
   - Both licenses have high candidate scores (0.9)
   - cc-by-4.0_3.RULE ranks first in sequence matching
   - No mechanism to prefer the more specific license

3. **Missing specificity logic**:
   - When two licenses have similar scores, prefer the one with MORE restrictions/clauses
   - CC-BY-NC-4.0 has additional text ("NonCommercial") that should make it more specific

**Candidate Ranking**:
```
NEAR-DUPE CANDIDATES: 10
  1. cc-by-4.0_3.RULE (score: 0.9000)         ← Generic version ranked #1
  3. cc-by-nc-4.0_1.RULE (score: 0.9000)      ← Specific version ranked #3
```

---

### Case 3: bsd.f - bsd-simplified vs bsd-new

**File**: `testdata/license-golden/datadriven/external/licensecheck/devscripts/bsd.f`

**Expected**: `bsd-simplified` (BSD-2-Clause)
**Actual**: `bsd-new` (BSD-3-Clause)

**Test File Content**:
```fortran
c Copyright (c) 2012, Devscripts developers
c
c Redistribution and use in source and binary forms, with or without
c modification, are permitted provided that the following conditions are
c met:
c
c   - Redistributions of source code must retain the above copyright
c     notice, this list of conditions and the following disclaimer.
c
c   - Redistributions in binary form must reproduce the above copyright
c     notice, this list of conditions and the following disclaimer in the
c     documentation and/or other materials provided with the
c     distribution.
c
c THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS
c IS" ...
```

**Root Cause Analysis**:

1. **License difference**:
   - bsd-simplified (BSD-2-Clause): 2 clauses (redistribution conditions + disclaimer)
   - bsd-new (BSD-3-Clause): 3 clauses (adds "no endorsement" clause)

2. **Text match**:
   - File contains only 2-clause BSD text (no endorsement clause)
   - Should match bsd-simplified, not bsd-new

3. **Bug location**:
   - Candidates correctly rank bsd-simplified first
   - But final detection produces bsd-new
   - Issue is in match merging or detection assembly phase

**Candidate Ranking**:
```
NEAR-DUPE CANDIDATES: 44
  1. bsd-simplified.LICENSE (score: 0.7000)    ← CORRECT ranked #1
  2. bsd-simplified_169.RULE (score: 0.7000)
  3. bsd-simplified_95.RULE (score: 0.7000)
  ...
  (no bsd-new in top 10)
```

**Final Result** (INCORRECT):
```
License expression: bsd-new
SPDX expression: bsd-new
```

---

### Case 4: PHP-3.01.t2 - PHP-3.0 vs PHP-3.01

**File**: `testdata/license-golden/datadriven/external/glc/PHP-3.01.t2`

**Expected**: `php-3.01`
**Actual**: `php-3.0`

**Root Cause Analysis**:

1. **License versions**: PHP-3.01 is a minor version update to PHP-3.0
   - Very similar text
   - PHP-3.01 likely has small additions or modifications

2. **Candidate selection issue**:
   - Both licenses will have high resemblance scores
   - Generic version (php-3.0) matches before specific version (php-3.01)
   - No version-aware selection logic

---

### Case 5: lgpl-2.1-plus_19.txt - lgpl-3.0-plus vs lgpl-2.1-plus

**File**: `testdata/license-golden/datadriven/lic3/lgpl-2.1-plus_19.txt`

**Expected**: `lgpl-2.1-plus`
**Actual**: `lgpl-3.0-plus`

**Test File Content** (key excerpt):
```
dnl  The GNU MP Library is free software; you can redistribute it and/or
dnl  modify it under the terms of the GNU Lesser General Public License as
dnl  published by the Free Software Foundation; either version 2.1 of the
dnl  License, or (at your option) any later version.
```

**Root Cause Analysis**:

1. **License text explicitly states**: "either version 2.1 of the License"
   - Clear indicator this is LGPL-2.1, not LGPL-3.0

2. **Version detection issue**:
   - LGPL-3.0 and LGPL-2.1 have substantial text overlap
   - Sequence matcher may match LGPL-3.0 rules first
   - Missing logic to detect version-specific language

---

## Root Causes Summary

### Root Cause 1: Aho-Corasick Matches Bypass Better Candidates

**Location**: `src/license_detection/mod.rs` lines 180-206

**Problem**: Aho-Corasick exact matches are accepted immediately without checking if sequence matcher candidates would produce a better (more specific) match.

**Current Code**:
```rust
// Phase 1c: Aho-Corasick matching
let refined_aho = match_refine::refine_aho_matches(&self.index, aho_matches, &query);

for m in &refined_aho {
    if m.match_coverage >= 99.99 && m.end_token > m.start_token {
        matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
    }
}
all_matches.extend(refined_aho);
```

**Issue**: The Aho matches are added to `all_matches` and then proceed to sequence matching. However, when the matches are merged and refined, the Aho matches can "win" over better sequence matches because they have exact match scores (often 100% coverage).

**Python Behavior**: Python's `get_exact_matches()` (index.py) also runs Aho matching first, but the refinement and scoring process considers both Aho and sequence matches together, allowing better matches to win.

---

### Root Cause 2: Candidate Ranking Ignores License Specificity

**Location**: `src/license_detection/seq_match/candidates.rs` lines 40-66

**Problem**: The `ScoresVector` comparison uses these criteria:
1. `is_highly_resemblant`
2. `containment`
3. `resemblance`  
4. `matched_length`

None of these capture license specificity. When two licenses (e.g., CC-BY-4.0 vs CC-BY-NC-4.0) have similar scores, the wrong one can win.

**Current Comparison**:
```rust
impl Ord for ScoresVector {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.is_highly_resemblant
            .cmp(&other.is_highly_resemblant)
            .then_with(|| self.containment.partial_cmp(&other.containment).unwrap())
            .then_with(|| self.resemblance.partial_cmp(&other.resemblance).unwrap())
            .then_with(|| self.matched_length.partial_cmp(&other.matched_length).unwrap())
    }
}
```

**Python Behavior**: Python uses the same ScoresVector, BUT its `filter_dupes` function groups by `rule.length` (see match_set.py:475), which naturally prefers longer (more specific) rules within the same license expression group.

---

### Root Cause 3: filter_dupes Groups Wrongly

**Location**: `src/license_detection/seq_match/candidates.rs` lines 111-159

**Problem**: The `DupeGroupKey` includes `rule_length`:
```rust
struct DupeGroupKey {
    license_expression: String,
    is_highly_resemblant: bool,
    containment: i32,
    resemblance: i32,
    matched_length: i32,
    rule_length: usize,  // ← This groups by rule length
}
```

This means `bsd-simplified` and `bsd-new` are in different groups because they have different `license_expression` values. The filtering happens WITHIN each license expression, not ACROSS expressions.

**What's needed**: When multiple license expressions have similar scores and overlapping text, prefer the one with:
- Longer rule text (more specific)
- More restrictive clauses
- Higher version number (for versioned licenses)

---

### Root Cause 4: Required Phrase Filtering Timing

**Location**: `src/license_detection/match_refine/filter_low_quality.rs` lines 144-239

**Problem**: Required phrase filtering happens in `refine_matches()` which is called AFTER Aho matches are collected. If an Aho match doesn't have required phrases, it passes through. But if a sequence match WITH required phrases would be better, it never gets evaluated because Aho already "claimed" that region.

**Example from JSON.t2**:
- MIT match (no required phrases) claims lines 3-11
- JSON match (with required phrase "shall be used for Good, not Evil") should claim lines 3-13
- MIT wins because it was found first by Aho matcher

---

### Root Cause 5: Match Merging Cross-License Handling

**Location**: `src/license_detection/match_refine/merge.rs` lines 68-211

**Problem**: `merge_overlapping_matches()` only merges matches with THE SAME `rule_identifier`. It does not handle cases where matches from DIFFERENT licenses (different rule_identifiers) overlap.

**Code**:
```rust
for m in sorted {
    if current_group.is_empty() || current_group[0].rule_identifier == m.rule_identifier {
        current_group.push(m);
    } else {
        grouped.push(current_group);
        current_group = vec![m];
    }
}
```

This means when `bsd-simplified` and `bsd-new` matches both exist for the same region, they are NOT compared or merged. The one that happens to sort first wins.

---

## Recommended Fixes

### Fix 1: Add Cross-License Match Comparison (HIGH PRIORITY)

**Problem**: Matches from different licenses that cover the same text are not compared.

**Solution**: Add a post-merge step that compares overlapping matches from DIFFERENT licenses and prefers:
1. Matches with higher specificity (longer rule text, more clauses)
2. Matches with higher version numbers (for versioned licenses)
3. Matches with required phrases that are satisfied

**Location**: New function in `src/license_detection/match_refine/merge.rs`

**Pseudocode**:
```rust
fn resolve_cross_license_overlaps(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    // Group matches by query region overlap
    // For each group of overlapping matches from different licenses:
    //   - Compute specificity score (rule_length, clause_count, version)
    //   - Check required phrase satisfaction
    //   - Select best match
}
```

---

### Fix 2: Improve Candidate Ranking for Specificity (HIGH PRIORITY)

**Problem**: Candidates with similar scores don't account for license specificity.

**Solution**: Add a tiebreaker in candidate ranking that prefers:
1. Longer rule text (indicates more specific license)
2. Higher version number (for licenses with version in key)
3. Licenses with additional restrictions (NC, ND, SA variants)

**Location**: `src/license_detection/seq_match/candidates.rs`

**Pseudocode**:
```rust
impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score_vec_rounded.cmp(&other.score_vec_rounded)
            .then_with(|| self.score_vec_full.cmp(&other.score_vec_full))
            // NEW: Prefer longer rules (more specific)
            .then_with(|| self.rule.tokens.len().cmp(&other.rule.tokens.len()).reverse())
            // NEW: Prefer higher version numbers
            .then_with(|| compare_license_versions(&self.rule.license_expression, &other.rule.license_expression))
    }
}
```

---

### Fix 3: Delay Aho Match Acceptance (MEDIUM PRIORITY)

**Problem**: Aho matches are accepted before sequence candidates are evaluated.

**Solution**: Run sequence matching even when Aho matches exist, then compare and select the best match. This matches Python's behavior more closely.

**Location**: `src/license_detection/mod.rs` lines 180-292

**Changes**:
1. Collect Aho matches but don't immediately mark regions as matched
2. Run sequence matching on full query
3. Compare Aho matches vs sequence matches for overlapping regions
4. Select the match with better specificity/score

---

### Fix 4: Improve Required Phrase Validation (MEDIUM PRIORITY)

**Problem**: Required phrases prevent good matches when text has minor variations.

**Solution**: 
1. Allow fuzzy required phrase matching with configurable threshold
2. Consider required phrases as "preferred" rather than "required" for high-scoring matches
3. Check if the REQUIRED phrase text appears ANYWHERE in the matched region, not just at expected positions

**Location**: `src/license_detection/match_refine/filter_low_quality.rs`

**Investigation Needed**: How does Python handle the JSON.t2 case? The text has "should rather be used for Good, not Evil" but the required phrase is "shall be used for Good, not Evil". Python still detects JSON. Need to understand Python's required phrase logic.

---

### Fix 5: Add License Specificity Metadata (LOW PRIORITY)

**Problem**: No metadata to determine which license is "more specific" when comparing variants.

**Solution**: Add fields to Rule struct:
```rust
pub struct Rule {
    // ... existing fields ...
    
    /// Licenses that this license is a more specific variant of
    pub more_specific_than: Vec<String>,  // e.g., ["cc-by-4.0"] for cc-by-nc-4.0
    
    /// Version number extracted from license key
    pub version: Option<f32>,  // e.g., 3.01 for php-3.01
    
    /// Restriction flags
    pub has_nc_restriction: bool,  // NonCommercial
    pub has_nd_restriction: bool,  // NoDerivatives
    pub has_sa_restriction: bool,  // ShareAlike
}
```

**Location**: `src/license_detection/models/rule.rs`

---

## Affected Test Cases

### Fix 1 (Cross-License Comparison) Would Fix:
- `bsd.f` (bsd-simplified vs bsd-new)
- `CC-BY-NC-4.0.t1` (cc-by-4.0 vs cc-by-nc-4.0)
- `CC-BY-NC-ND-4.0.t1` (cc-by-nd-4.0 vs cc-by-nc-nd-4.0)
- `CC-BY-NC-SA-4.0.t1` (cc-by-4.0 vs cc-by-nc-sa-4.0)
- All cases where generic license variant is selected over specific variant

### Fix 2 (Specificity Ranking) Would Fix:
- `CC-BY-SA-1.0.t1` (cc-by-sa-1.0 vs cc-by-nc-sa-1.0)
- `PHP-3.01.t2` (php-3.0 vs php-3.01)
- Any version selection issues

### Fix 3 (Delay Aho Acceptance) Would Fix:
- `JSON.t2` (mit vs json)
- Cases where Aho finds subset match before sequence finds better match

### Fix 4 (Required Phrase Handling) Would Fix:
- `JSON.t2` (if required phrase is the issue)
- `gpl-3+-with-rem-comment.xml` (gpl-3.0-plus vs gpl-3.0 AND other-copyleft)

---

## Python Comparison Summary

### Python's filter_dupes (match_set.py:461-498)

```python
def filter_dupes(sortable_candidates):
    def group_key(item):
        (sv_round, _sv_full), _rid, rule, _inter = item
        return (
            rule.license_expression,
            sv_round.is_highly_resemblant,
            sv_round.containment,
            sv_round.resemblance,
            sv_round.matched_length,
            rule.length,  # ← Python includes rule.length
        )
    
    def rank_key(item):
        (_sv_round, sv_full), _rid, rule, _inter = item
        return sv_full, rule.identifier  # ← Python uses rule.identifier for tiebreaker
    
    for group, duplicates in groupby(sortable_candidates, key=group_key):
        duplicates = sorted(duplicates, reverse=True, key=rank_key)
        yield duplicates[0]
```

**Key Differences from Rust**:
1. Python groups by `rule.length`, Rust groups by `rule.tokens.len()` (same concept)
2. Python uses `rule.identifier` for tiebreaker within a group
3. Python does NOT have cross-license comparison in filter_dupes

### Python's match() Pipeline (index.py:987-1145)

```python
def match(self, ...):
    # Phase 1: Hash matching
    hash_matches = self.hash_match(query_run)
    if hash_matches:
        return hash_matches  # Early return if hash matches
    
    # Phase 2: SPDX-LID matching
    spdx_matches = self.spdx_lid_match(query)
    
    # Phase 3: Aho-Corasick matching
    aho_matches = self.aho_match(query_run)
    refined_aho = refine_matches(aho_matches, query, merge=False)
    
    # Phase 4: Sequence matching (runs even with Aho matches!)
    seq_matches = self.seq_match(query_run, ...)
    
    # Phase 5: Merge ALL matches together
    all_matches = spdx_matches + refined_aho + seq_matches
    
    # Phase 6: Single refinement pass
    refined = refine_matches(all_matches, query, merge=True, filter_false_positive=True)
    
    return refined
```

**Key Difference**: Python runs sequence matching even when Aho matches exist, then refines ALL matches together. Rust's current code adds Aho matches to `matched_qspans` which can prevent sequence matching on those regions.

---

## Next Steps

1. **Implement Fix 1** (Cross-License Match Comparison) - Highest impact
2. **Implement Fix 2** (Specificity Ranking) - Addresses variant selection
3. **Investigate Python's required phrase handling** for JSON.t2 case
4. **Add comprehensive tests** for variant selection scenarios
5. **Consider Fix 3** (Delay Aho) if Fixes 1-2 don't resolve all cases

## Test Validation

After implementing fixes, validate with these test files:

```bash
# Test cross-license comparison (Fix 1)
cargo test --release --lib license_detection::golden_test -- bsd.f

# Test specificity ranking (Fix 2)  
cargo test --release --lib license_detection::golden_test -- CC-BY-NC-4.0.t1

# Test Aho vs sequence (Fix 3)
cargo test --release --lib license_detection::golden_test -- JSON.t2

# Run all golden tests
cargo test --release --lib license_detection::golden_test
```

## References

- Python match_set.py: `/reference/scancode-toolkit/src/licensedcode/match_set.py`
- Python index.py: `/reference/scancode-toolkit/src/licensedcode/index.py`
- Python match.py: `/reference/scancode-toolkit/src/licensedcode/match.py`
- Rust candidates.rs: `/src/license_detection/seq_match/candidates.rs`
- Rust merge.rs: `/src/license_detection/match_refine/merge.rs`
- Rust mod.rs: `/src/license_detection/mod.rs`
