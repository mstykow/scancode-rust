# License Detection Implementation Plans

This document outlines the implementation plans to close the gaps between the Rust and Python license detection implementations.

---

## Overview

| Plan ID | Title | Priority | Effort |
|---------|-------|----------|--------|
| PLAN-001 | SPDX `+` Suffix Support | High | Medium |
| PLAN-002 | License Intro Filtering | High | Medium |
| PLAN-003 | Deprecated Rule Filtering | Medium | Low |
| PLAN-004 | Overlapping Match Filtering | Medium | High |

---

## PLAN-001: SPDX `+` Suffix Support

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

## PLAN-002: License Intro Filtering

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

## PLAN-003: Deprecated Rule Filtering

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

## PLAN-004: Overlapping Match Filtering

### Problem

Complex overlap scenarios between matches are not handled correctly, leading to incorrect expression combinations.

### Root Cause

Rust has basic `filter_contained_matches()` but lacks the sophisticated `filter_overlapping_matches()` logic from Python.

### Python Implementation

**Overlap thresholds:**

```python
OVERLAP_SMALL = 0.10      # 10%
OVERLAP_MEDIUM = 0.40     # 40%
OVERLAP_LARGE = 0.70      # 70%
OVERLAP_EXTRA_LARGE = 0.90  # 90%
```

**Priority order (sorting):**

```python
sorter = lambda m: (m.qspan.start, -m.hilen(), -m.len(), m.matcher_order)
```

**Filtering logic:**

1. Sort matches by start position, then by quality (hilen, len, matcher order)
2. For each pair of overlapping matches, calculate overlap ratios
3. Based on overlap level and relative quality, decide which to discard
4. Run `restore_non_overlapping()` to recover discarded matches that don't conflict

### Required Changes

This is a complex change. Consider incremental approach:

#### Phase 1: Add overlap ratio calculation

**File:** `src/license_detection/spans.rs`

```rust
impl Span {
    pub fn overlap(&self, other: &Span) -> usize {
        // Count of positions in intersection
    }
    
    pub fn overlap_ratio(&self, other: &Span) -> f64 {
        let overlap = self.overlap(other);
        overlap as f64 / self.len().max(other.len()) as f64
    }
}
```

#### Phase 2: Implement `hilen()` for matches

Add method to count high-value (legalese) tokens in a match:

```rust
impl LicenseMatch {
    pub fn hilen(&self) -> usize {
        // Count of high-value tokens in matched range
    }
}
```

#### Phase 3: Implement `filter_overlapping_matches()`

**File:** `src/license_detection/match_refine.rs`

```rust
const OVERLAP_SMALL: f64 = 0.10;
const OVERLAP_MEDIUM: f64 = 0.40;
const OVERLAP_LARGE: f64 = 0.70;
const OVERLAP_EXTRA_LARGE: f64 = 0.90;

pub fn filter_overlapping_matches(matches: Vec<LicenseMatch>) -> Vec<LicenseMatch> {
    // 1. Sort by (start, -hilen, -len, matcher_order)
    // 2. For each pair, calculate overlap
    // 3. Apply thresholds to decide which to discard
    // 4. Return filtered list
}
```

#### Phase 4: Implement `restore_non_overlapping()`

```rust
pub fn restore_non_overlapping(
    kept: &[LicenseMatch],
    discarded: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    // Return discarded matches that don't overlap with any kept match
}
```

#### Phase 5: Update refine pipeline

```rust
pub fn refine_matches(...) -> Vec<LicenseMatch> {
    let filtered = filter_contained_matches(matches);
    let (kept, discarded) = filter_overlapping_matches(filtered);
    let restored = restore_non_overlapping(&kept, &discarded);
    // ...
}
```

### Test Verification

Golden tests should show improved pass rate, especially for files with multiple overlapping license detections.

### Files Changed

| File | Change |
|------|--------|
| `src/license_detection/spans.rs` | Add `overlap()`, `overlap_ratio()` methods |
| `src/license_detection/models.rs` | Add `hilen()` method to `LicenseMatch` |
| `src/license_detection/match_refine.rs` | Implement `filter_overlapping_matches()`, `restore_non_overlapping()` |

---

## Implementation Order

1. **PLAN-003** (Deprecated Rule Filtering) - Low effort, immediate golden test improvement
2. **PLAN-001** (SPDX `+` Suffix) - Fixes specific failing test, medium effort
3. **PLAN-002** (License Intro Filtering) - High impact on golden tests, medium effort
4. **PLAN-004** (Overlapping Match Filtering) - Complex, defer until other issues resolved

---

## Testing Strategy

After each plan implementation:

1. Run relevant unit tests
2. Run golden test suite to measure improvement
3. Update DEBUG.md with results
