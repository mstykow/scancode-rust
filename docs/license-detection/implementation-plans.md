# License Detection Implementation Plans

This document outlines the implementation plans to close the gaps between the Rust and Python license detection implementations.

---

## Overview

| Plan ID | Title | Priority | Effort | Status |
|---------|-------|----------|--------|--------|
| PLAN-001 | SPDX `+` Suffix Support | High | Medium | ✅ Done |
| PLAN-002 | License Intro Filtering | High | Medium | ✅ Done |
| PLAN-003 | Deprecated Rule Filtering | Medium | Low | ✅ Done |
| PLAN-004 | Overlapping Match Filtering | Medium | High | 📋 Planned |

---

## PLAN-001: SPDX `+` Suffix Support ✅

**Status:** Completed in commit `9b47b558`

### Problem

SPDX identifiers with `+` suffix (e.g., `GPL-2.0+`) are not detected. The SPDX key `GPL-2.0+` should map to the ScanCode key `gpl-2.0-plus`.

### Root Cause

The Rust `find_best_matching_rule()` function only matches against `rule.license_expression`, not against SPDX-specific keys. Python has a separate lookup table for SPDX key mappings.

### Python Implementation

**License data structure** (`gpl-2.0-plus.LICENSE`):

The Rust `find_best_matching_rule()` function only matches against `rule.license_expression`, not against SPDX-specific keys. Python has a separate lookup table for SPDX key mappings.

```yaml

key: gpl-2.0-plus
spdx_license_key: GPL-2.0-or-later
other_spdx_license_keys:

- GPL-2.0+
- GPL 2.0+

```

**SPDX key mapping** (`cache.py:build_spdx_symbols()`):

```python
licenses_by_spdx_key = get_licenses_by_spdx_key(
    licenses=licenses_db.values(),
    include_other_spdx_license_keys=True,
)
```

### Required Changes

#### 1. Add SPDX fields to Rule struct

**File:** `src/license_detection/models.rs`

```rust
pub struct Rule {
    // ... existing fields ...
    pub spdx_license_key: Option<String>,
    pub other_spdx_license_keys: Vec<String>,
}
```

#### 2. Update loader to parse SPDX fields

**File:** `src/license_detection/rules/loader.rs`

Parse `spdx_license_key` and `other_spdx_license_keys` from .LICENSE files.

#### 3. Build SPDX-to-RID lookup table in index

**File:** `src/license_detection/index/builder.rs`

```rust
pub struct LicenseIndex {
    // ... existing fields ...
    pub rid_by_spdx_key: HashMap<String, usize>,
}
```

During index building:

```rust
for (rid, rule) in rules_by_rid.iter().enumerate() {
    if let Some(ref spdx_key) = rule.spdx_license_key {
        rid_by_spdx_key.insert(spdx_key.to_lowercase(), rid);
    }
    for alias in &rule.other_spdx_license_keys {
        rid_by_spdx_key.insert(alias.to_lowercase(), rid);
    }
}
```

#### 4. Update `find_best_matching_rule()` to use SPDX lookup

**File:** `src/license_detection/spdx_lid.rs`

```rust
fn find_best_matching_rule(index: &LicenseIndex, spdx_key: &str) -> Option<usize> {
    // First try direct SPDX key lookup
    let normalized = normalize_spdx_key(spdx_key);
    if let Some(&rid) = index.rid_by_spdx_key.get(&normalized) {
        return Some(rid);
    }
    
    // Fallback to license expression matching (current behavior)
    for (rid, rule) in index.rules_by_rid.iter().enumerate() {
        // ... existing logic ...
    }
}
```

### Test Verification

```bash
cargo test test_spdx_with_plus -- --nocapture
```

### Files Changed

| File | Change |
|------|--------|
| `src/license_detection/models.rs` | Add `spdx_license_key`, `other_spdx_license_keys` fields |
| `src/license_detection/rules/loader.rs` | Parse SPDX fields from .LICENSE files |
| `src/license_detection/index/builder.rs` | Build `rid_by_spdx_key` lookup table |
| `src/license_detection/index/mod.rs` | Add `rid_by_spdx_key` to `LicenseIndex` |
| `src/license_detection/spdx_lid.rs` | Update `find_best_matching_rule()` |

---

## PLAN-002: License Intro Filtering ✅

**Status:** Completed in commit `f93270b6`

### Problem

License expressions incorrectly include "unknown" from license intro matches. Python filters these out before building expressions.

### Root Cause

Rust builds expressions from ALL matches without filtering intros. Python has:

1. `is_license_intro()` - identifies intro matches
2. `filter_license_intros()` - removes them from expression building
3. Category analysis that determines when to apply filtering

### Python Implementation

**`is_license_intro()`** (`detection.py:1349-1365`):

```python
def is_license_intro(license_match):
    return (
        (
            license_match.rule.is_license_intro or 
            license_match.rule.is_license_clue or
            license_match.rule.license_expression == 'free-unknown'
        )
        and (
            license_match.matcher == MATCH_AHO_EXACT  # '2-aho'
            or license_match.coverage() == 100
        )
    )
```

**`filter_license_intros()`** (`detection.py:1336-1347`):

```python
def filter_license_intros(license_match_objects):
    filtered_matches = [m for m in license_match_objects if not is_license_intro(m)]
    if not filtered_matches:
        return license_match_objects
    return filtered_matches
```

**Called when:** `analysis == UNKNOWN_INTRO_BEFORE_DETECTION`

### Required Changes

#### 1. Add fields to LicenseMatch struct

**File:** `src/license_detection/models.rs`

```rust
pub struct LicenseMatch {
    // ... existing fields ...
    pub is_license_intro: bool,
    pub is_license_clue: bool,
}
```

#### 2. Propagate flags when creating matches

**Files:** `src/license_detection/hash_match.rs`, `src/license_detection/aho_match.rs`, `src/license_detection/seq_match.rs`

When creating `LicenseMatch`, copy flags from the rule:

```rust
LicenseMatch {
    // ... existing fields ...
    is_license_intro: rule.is_license_intro,
    is_license_clue: rule.is_license_clue,
}
```

#### 3. Implement correct `is_license_intro()` function

**File:** `src/license_detection/detection.rs`

```rust
fn is_license_intro(match_item: &LicenseMatch) -> bool {
    (match_item.is_license_intro || 
     match_item.is_license_clue || 
     match_item.license_expression == "free-unknown")
    && (match_item.matcher == "2-aho" || match_item.match_coverage >= 99.99)
}
```

#### 4. Implement `filter_license_intros()` function

**File:** `src/license_detection/detection.rs`

```rust
fn filter_license_intros(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    let filtered: Vec<_> = matches.iter()
        .filter(|m| !is_license_intro(m))
        .cloned()
        .collect();
    
    if filtered.is_empty() {
        matches.to_vec()
    } else {
        filtered
    }
}
```

#### 5. Update `create_detection_from_group()` to filter intros

**File:** `src/license_detection/detection.rs`

Before building expression, check category and filter:

```rust
fn create_detection_from_group(group: &[LicenseMatch]) -> LicenseDetection {
    // Determine category first
    let category = analyze_detection(group);
    
    // Filter matches based on category
    let matches_for_expression = if category == DetectionCategory::UnknownIntroBeforeDetection {
        filter_license_intros(group)
    } else {
        group.to_vec()
    };
    
    // Then build expression
    let expression = determine_license_expression(&matches_for_expression);
    // ...
}
```

#### 6. Implement `analyze_detection()` for category determination

**File:** `src/license_detection/detection.rs`

Implement logic to detect `UNKNOWN_INTRO_BEFORE_DETECTION` category:

- An unknown intro match followed by a proper license match
- The intro must immediately precede the detection

### Test Verification

Golden test `double_isc.txt` should produce `["isc", "isc", "sudo"]` instead of `["isc", "isc AND unknown"]`.

### Files Changed

| File | Change |
|------|--------|
| `src/license_detection/models.rs` | Add `is_license_intro`, `is_license_clue` to `LicenseMatch` |
| `src/license_detection/hash_match.rs` | Propagate flags when creating matches |
| `src/license_detection/aho_match.rs` | Propagate flags when creating matches |
| `src/license_detection/seq_match.rs` | Propagate flags when creating matches |
| `src/license_detection/detection.rs` | Implement `is_license_intro()`, `filter_license_intros()`, `analyze_detection()` |

---

## PLAN-003: Deprecated Rule Filtering ✅

**Status:** Completed in commit `3b5ea424`

### Problem

Deprecated rules are being used for detection when they should be skipped by default.

### Root Cause

Rust loads all rules including deprecated ones. Python filters deprecated rules by default (`with_deprecated=False`).

### Python Implementation

**License loading** (`models.py:844-845`):

```python
if not with_deprecated and lic.is_deprecated:
    continue
```

**Rule loading** (`models.py:1245-1246`):

```python
if not with_deprecated and rule.is_deprecated:
    continue
```

**Validation** (`models.py:1103-1104`):

```python
# always skip deprecated rules
rules = [r for r in rules if not r.is_deprecated]
```

### Required Changes

#### 1. Add `with_deprecated` parameter to loader functions

**File:** `src/license_detection/rules/loader.rs`

```rust
pub fn load_licenses_from_directory(
    path: &Path,
    with_deprecated: bool,  // Default: false
) -> Result<Vec<License>> {
    // ...
    if !with_deprecated && license.is_deprecated {
        continue;
    }
    // ...
}

pub fn load_rules_from_directory(
    path: &Path,
    with_deprecated: bool,  // Default: false
) -> Result<Vec<Rule>> {
    // ...
    if !with_deprecated && rule.is_deprecated {
        continue;
    }
    // ...
}
```

#### 2. Update callers to pass `with_deprecated = false`

**File:** `src/license_detection/mod.rs`

```rust
let rules = load_rules_from_directory(&rules_dir, false)?;
let licenses = load_licenses_from_directory(&licenses_dir, false)?;
```

#### 3. Update golden tests that expect deprecated expressions

Some golden tests expect deprecated license expressions. Options:

1. Update test expected values to use replacement expressions
2. Add test for deprecated rule handling with `with_deprecated = true`

**Files to check:** `testdata/license-golden/datadriven/lic1/freebsd-doc_*.txt.EXPECTED`

### Test Verification

```bash
cargo test --release license_detection::golden_test -- --nocapture
```

Should see improved pass rate. The test `camellia_bsd.c` should produce `bsd-2-clause-first-lines` instead of deprecated `freebsd-doc`.

### Files Changed

| File | Change |
|------|--------|
| `src/license_detection/rules/loader.rs` | Add `with_deprecated` parameter, filter deprecated |
| `src/license_detection/mod.rs` | Pass `with_deprecated = false` |
| `testdata/license-golden/datadriven/lic1/freebsd-doc_*.txt.EXPECTED` | May need updates |

---

## PLAN-004: Overlapping Match Filtering 📋

### Problem

Complex overlap scenarios between matches are not handled correctly, leading to incorrect expression combinations.

### Root Cause

Rust has basic `filter_contained_matches()` but lacks the sophisticated `filter_overlapping_matches()` logic from Python.

### Python Reference

**Location:** `reference/scancode-toolkit/src/licensedcode/match.py`

- `filter_overlapping_matches()` - lines 1187-1523
- `restore_non_overlapping()` - lines 1526-1548

### Python Implementation Details

#### Overlap Thresholds

```python
OVERLAP_SMALL = 0.10       # 10%
OVERLAP_MEDIUM = 0.40      # 40%
OVERLAP_LARGE = 0.70       # 70%
OVERLAP_EXTRA_LARGE = 0.90 # 90%
```

#### Sorting Criteria

```python
sorter = lambda m: (m.qspan.start, -m.hilen(), -m.len(), m.matcher_order)
```

#### Filtering Logic by Overlap Level

| Level | Condition | Action |
|-------|-----------|--------|
| EXTRA_LARGE (≥90%) | `overlap/next.len ≥ 0.9` AND `current.len ≥ next.len` | Discard next |
| EXTRA_LARGE (≥90%) | `overlap/current.len ≥ 0.9` AND `current.len ≤ next.len` | Discard current |
| LARGE (≥70%) | Same as above + `hilen` comparison | Discard shorter with fewer high-tokens |
| MEDIUM (≥40%) | + `licensing_contains()` check | Discard if licensing contained + shorter |
| SMALL (≥10%) | + `surround()` + `licensing_contains()` | Discard if surrounded and contained |

#### Special Cases

1. **Skip overlapping false positives** (lines 1276-1286): Adjacent FP matches not treated as overlapping
2. **Sandwich detection** (lines 1486-1507): Discard current if 90% contained in previous+next
3. **Trailing "license foo"** (lines 1387-1402): Special handling for 2-token license patterns

#### Helper Methods Required

| Method | Python Lines | Purpose |
|--------|--------------|---------|
| `overlap()` | spans.py:312-330 | Count positions in intersection |
| `hilen()` | match.py:1220 | Count high-value (legalese) tokens |
| `licensing_contains()` | match.py:1374,1404,etc. | Check license text containment |
| `surround()` | match.py:1452,1467 | Check if one match surrounds another |
| `merge_matches()` | match.py:1540 | Merge discarded before restoration |

### Required Changes

#### Phase 1: Span Operations

**File:** `src/license_detection/spans.rs`

```rust
impl Span {
    pub fn overlap(&self, other: &Span) -> usize {
        self.intersection(other).len()
    }
    
    pub fn overlap_ratio(&self, other: &Span) -> f64 {
        let overlap = self.overlap(other);
        overlap as f64 / self.len().max(other.len()) as f64
    }
    
    pub fn union_span(&self, other: &Span) -> Span {
        // Union of two spans for all_matched_qspans calculation
    }
}
```

#### Phase 2: LicenseMatch Methods

**File:** `src/license_detection/models.rs`

```rust
impl LicenseMatch {
    pub fn hilen(&self) -> usize {
        // Count of high-value (legalese) tokens in matched range
        // Requires access to index.high_postings_by_rid
    }
    
    pub fn licensing_contains(&self, other: &LicenseMatch) -> bool {
        // Check if self's license text contains other's license text
        // Based on rule text containment
    }
    
    pub fn surround(&self, other: &LicenseMatch) -> bool {
        // Check if self surrounds other (starts before and ends after)
    }
    
    pub fn matcher_order(&self) -> u8 {
        // Matcher precedence: 1=hash, 2=aho, 3=spdx, 4=seq, 5=unknown
        match self.matcher.as_str() {
            "1-hash" => 1,
            "2-aho" => 2,
            "3-spdx" => 3,
            "4-seq" => 4,
            "5-unknown" => 5,
            _ => 9,
        }
    }
}
```

#### Phase 3: Filter Overlapping Matches

**File:** `src/license_detection/match_refine.rs`

```rust
const OVERLAP_SMALL: f64 = 0.10;
const OVERLAP_MEDIUM: f64 = 0.40;
const OVERLAP_LARGE: f64 = 0.70;
const OVERLAP_EXTRA_LARGE: f64 = 0.90;

/// Return (kept_matches, discarded_matches)
/// Based on Python: filter_overlapping_matches() at match.py:1187-1523
pub fn filter_overlapping_matches(
    matches: Vec<LicenseMatch>,
    index: &LicenseIndex,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    if matches.len() < 2 {
        return (matches, vec![]);
    }
    
    let mut matches = matches;
    let mut discarded: Vec<LicenseMatch> = vec![];
    
    // Sort by (start, -hilen, -len, matcher_order)
    matches.sort_by(|a, b| {
        a.start_line
            .cmp(&b.start_line)
            .then_with(|| b.hilen().cmp(&a.hilen()))
            .then_with(|| b.matched_length.cmp(&a.matched_length))
            .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
    });
    
    let mut i = 0;
    while i < matches.len() - 1 {
        let mut j = i + 1;
        while j < matches.len() {
            // Check for early break (no overlap possible)
            if matches[j].start_line > matches[i].end_line {
                break;
            }
            
            // Skip overlapping false positives
            let both_fp = is_false_positive(&matches[i], index) 
                && is_false_positive(&matches[j], index);
            if both_fp {
                j += 1;
                continue;
            }
            
            // Calculate overlap ratios
            let overlap = calculate_overlap(&matches[i], &matches[j]);
            if overlap == 0 {
                j += 1;
                continue;
            }
            
            let ratio_to_next = overlap as f64 / matches[j].matched_length as f64;
            let ratio_to_current = overlap as f64 / matches[i].matched_length as f64;
            
            // Apply overlap thresholds (EXTRA_LARGE, LARGE, MEDIUM, SMALL)
            // ... detailed logic from Python ...
            
            // Check for sandwich (current 90% in previous+next)
            // ... sandwich detection logic ...
            
            j += 1;
        }
        i += 1;
    }
    
    (matches, discarded)
}
```

#### Phase 4: Restore Non-Overlapping

**File:** `src/license_detection/match_refine.rs`

```rust
/// Reintegrate discarded matches that don't overlap with kept matches.
/// Based on Python: restore_non_overlapping() at match.py:1526-1548
pub fn restore_non_overlapping(
    kept: &[LicenseMatch],
    discarded: Vec<LicenseMatch>,
) -> (Vec<LicenseMatch>, Vec<LicenseMatch>) {
    // Build union of all matched spans
    let all_matched = kept.iter()
        .fold(Span::new(), |acc, m| acc.union_span(&m.span()));
    
    let mut to_keep = vec![];
    let mut still_discarded = vec![];
    
    // Merge discarded matches first
    let merged_discarded = merge_overlapping_matches(&discarded);
    
    for disc in merged_discarded {
        if !disc.span().intersects(&all_matched) {
            to_keep.push(disc);
        } else {
            still_discarded.push(disc);
        }
    }
    
    (to_keep, still_discarded)
}
```

#### Phase 5: Update Refine Pipeline

**File:** `src/license_detection/match_refine.rs`

```rust
pub fn refine_matches(
    index: &LicenseIndex,
    matches: Vec<LicenseMatch>,
    query: &Query,
) -> Vec<LicenseMatch> {
    // 1. Filter short GPL false positives
    let filtered = filter_short_gpl_matches(&matches);
    
    // 2. Merge overlapping/adjacent matches (existing)
    let merged = merge_overlapping_matches(&filtered);
    
    // 3. Filter contained matches (existing)
    let non_contained = filter_contained_matches(&merged);
    
    // 4. Filter overlapping matches (NEW)
    let (kept, discarded) = filter_overlapping_matches(non_contained, index);
    
    // 5. Restore non-overlapping discarded (NEW)
    let (restored, _) = restore_non_overlapping(&kept, discarded);
    
    // 6. Combine kept + restored
    let mut final_matches = kept;
    final_matches.extend(restored);
    
    // 7. Filter false positives (existing)
    let non_fp = filter_false_positive_matches(index, &final_matches);
    
    // 8. Update scores
    let mut scored = non_fp;
    update_match_scores(&mut scored);
    
    scored
}
```

### Test Verification

```bash
cargo test --release license_detection::match_refine::tests -- --nocapture
cargo test --release license_detection::golden_test -- --nocapture
```

Golden tests should show improved pass rate, especially for files with multiple overlapping license detections.

### Files Changed

| File | Change |
|------|--------|
| `src/license_detection/spans.rs` | Add `overlap()`, `overlap_ratio()`, `union_span()` methods |
| `src/license_detection/models.rs` | Add `hilen()`, `licensing_contains()`, `surround()`, `matcher_order()` to LicenseMatch |
| `src/license_detection/match_refine.rs` | Implement `filter_overlapping_matches()`, `restore_non_overlapping()`, update `refine_matches()` |

### Complexity Notes

1. **`hilen()` requires index access**: Need to pass index or cache high-token count in LicenseMatch
2. **`licensing_contains()` needs rule text**: May need to store rule text reference or text hash
3. **Return type changes**: Functions need to return `(kept, discarded)` tuples
4. **Sandwich detection**: Requires tracking previous match state

---

## Implementation Order (Updated)

1. ~~PLAN-003~~ (Deprecated Rule Filtering) - ✅ Done
2. ~~PLAN-001~~ (SPDX `+` Suffix) - ✅ Done
3. ~~PLAN-002~~ (License Intro Filtering) - ✅ Done
4. **PLAN-004** (Overlapping Match Filtering) - 📋 Next

---

## Testing Strategy

After each plan implementation:

1. Run relevant unit tests
2. Run golden test suite to measure improvement
3. Update DEBUG.md with results
