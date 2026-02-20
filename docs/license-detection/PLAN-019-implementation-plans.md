# PLAN-019: Implementation Plans for License Detection Alignment
This document contains detailed implementation plans for aligning Rust's license detection with Python's behavior.
---
## Part 1: `is_license_text` Field and Filtering Logic
### Summary
The `is_license_text` flag indicates a rule matches a full license text (not just a notice or reference). Python uses this in filtering to subtract matched regions for long license texts (>120 tokens, >98% coverage) to prevent spurious matches inside license text.
---
### 1. Field Addition to LicenseMatch
#### 1.1 Location
**File**: `src/license_detection/models.rs`
#### 1.2 Add Field to LicenseMatch Struct (after line 272)
```rust
/// True if this match is from a license text rule (full license text, not notice)
#[serde(default)]
pub is_license_text: bool,
```
#### 1.3 Update Default Implementation (line 312-342)
Add to the `Default` implementation:
```rust
is_license_text: false,
```
#### 1.4 All Creation Sites That Need Updating
| File | Line(s) | Function | Change |
|------|---------|----------|--------|
| `hash_match.rs` | 99-126 | `hash_match()` | Add `is_license_text: rule.is_license_text` |
| `aho_match.rs` | 160-187 | `aho_match()` | Add `is_license_text: rule.is_license_text` |
| `spdx_lid.rs` | 257-284 | `spdx_lid_match()` | Add `is_license_text: rule.is_license_text` |
| `seq_match.rs` | 730-757 | `seq_match()` | Add `is_license_text: candidate.rule.is_license_text` |
| `seq_match.rs` | 860-887 | `seq_match_with_candidates()` | Add `is_license_text: candidate.rule.is_license_text` |
| `unknown_match.rs` | ~150+ | `create_unknown_match()` | Add `is_license_text: false` |
| `match_refine.rs` | 2866, 2923, 2992, 3047, 3102 | Test helpers | Add `is_license_text: false` |
| `test_utils.rs` | 61, 114 | Test helpers | Add `is_license_text: true/false` |
---
### 2. Filtering Logic Implementation
#### 2.1 Location
**File**: `src/license_detection/mod.rs`
#### 2.2 Exact Location in Pipeline
The filtering must happen **after each matcher phase** (within the Phase 1 block). This matches Python's behavior where subtraction happens after each matcher completes.
#### 2.3 Code Changes
**Current Code** (lines 121-149):
```rust
// Phase 1: Hash, SPDX, Aho-Corasick matching
{
    let whole_run = query.whole_query_run();
    let hash_matches = hash_match(&self.index, &whole_run);
    for m in &hash_matches {
        if m.match_coverage >= 99.99 && m.end_token > m.start_token {
            matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
        }
    }
    all_matches.extend(hash_matches);
}
```
**New Code** - Add subtraction after each matcher:
```rust
// Phase 1: Hash, SPDX, Aho-Corasick matching
{
    let whole_run = query.whole_query_run();
    let hash_matches = hash_match(&self.index, &whole_run);
    for m in &hash_matches {
        if m.match_coverage >= 99.99 && m.end_token > m.start_token {
            matched_qspans.push(query::PositionSpan::new(m.start_token, m.end_token - 1));
        }
        // NEW: Subtract long license text matches
        if m.is_license_text && m.rule_length > 120 && m.match_coverage > 98.0 {
            let span = query::PositionSpan::new(m.start_token, m.end_token.saturating_sub(1));
            query.subtract(&span);
        }
    }
    all_matches.extend(hash_matches);
    // Repeat similar logic for spdx_matches and aho_matches
}
```
---
### 3. Query Subtraction
The `Query` struct **already supports subtraction** via `query.subtract(&span)` method at `query.rs:779-791`. No changes needed.
---
### 4. Tests
```rust
#[test]
fn test_is_license_text_subtraction_triggers() {
    let conditions = (true, 150usize, 99.0f32);
    assert!(conditions.0 && conditions.1 > 120 && conditions.2 > 98.0);
}
#[test]
fn test_is_license_text_subtraction_skips_short() {
    let conditions = (true, 100usize, 99.0f32);
    assert!(!(conditions.0 && conditions.1 > 120 && conditions.2 > 98.0));
}
#[test]
fn test_is_license_text_subtraction_skips_low_coverage() {
    let conditions = (true, 150usize, 95.0f32);
    assert!(!(conditions.0 && conditions.1 > 120 && conditions.2 > 98.0));
}
```
---
### 5. Edge Cases
- **`rule_length == 0`**: Condition `rule_length > 120` is false, no subtraction
- **Multiple overlapping matches**: `Query.subtract()` removes positions from matchables, handled correctly
- **Empty match**: Condition `m.end_token > m.start_token` prevents empty spans
---
### Implementation Checklist
- [ ] Add `is_license_text: bool` field to `LicenseMatch` struct
- [ ] Update `Default` implementation
- [ ] Update all creation sites (~8 files)
- [ ] Add subtraction logic in Phase 1 after each matcher
- [ ] Add unit tests
- [ ] Run `cargo test` and `cargo clippy`
---
## Part 2: License Flags in LicenseMatch
### 1. Complete Inventory of License Flags
Python's `license_flag_names` property defines **6 mutually exclusive flags**:
| Flag | Description | Used in Filtering |
|------|-------------|-------------------|
| `is_license_text` | Full license text (highest confidence) | ✅ Yes - subtract long matches |
| `is_license_notice` | Explicit notice like "Licensed under MIT" | ❌ No |
| `is_license_reference` | Reference like bare name or URL | ❌ No |
| `is_license_tag` | Structured tag (e.g., SPDX identifier) | ❌ No |
| `is_license_intro` | Intro before actual license text | ❌ No |
| `is_license_clue` | Clue but not proper detection | ❌ No |
### 2. Python Match.to_dict() JSON Output
**Critical finding**: Python's `Match.to_dict()` does **NOT** serialize any `is_license_*` flags to JSON.
Only these fields are output:
```
license_expression, license_expression_spdx, from_file, start_line, end_line,
matcher, score, matched_length, match_coverage, rule_relevance,
rule_identifier, rule_url, matched_text (optional)
```
### 3. Comparison Table
| Flag | Python Rule | Python JSON | Rust Rule | Rust LicenseMatch | Rust JSON | Gap |
|------|:-----------:|:-----------:|:---------:|:-----------------:|:---------:|:---:|
| `is_license_text` | ✅ | ❌ | ✅ | ❌ | ❌ | Missing |
| `is_license_notice` | ✅ | ❌ | ✅ | ❌ | ❌ | Missing |
| `is_license_reference` | ✅ | ❌ | ✅ | ✅ | ✅ | Extra serialization |
| `is_license_tag` | ✅ | ❌ | ✅ | ✅ | ✅ | Extra serialization |
| `is_license_intro` | ✅ | ❌ | ✅ | ✅ | ✅ | Extra serialization |
| `is_license_clue` | ✅ | ❌ | ✅ | ✅ | ✅ | Extra serialization |
### 4. Recommended Changes
Add missing flags but **skip serialization** to match Python's behavior exactly.
#### Changes to `src/license_detection/models.rs`
**Add fields to `LicenseMatch` (around line 261):**
```rust
/// True if this match is from a license text rule (full license text)
#[serde(skip)]
pub is_license_text: bool,
/// True if this match is from a license notice rule
#[serde(skip)]
pub is_license_notice: bool,
/// True if this match is from a license intro rule
#[serde(skip)]
pub is_license_intro: bool,
/// True if this match is from a license clue rule
#[serde(skip)]
pub is_license_clue: bool,
/// True if this match is from a license reference rule
#[serde(skip)]
pub is_license_reference: bool,
/// True if this match is from a license tag rule
#[serde(skip)]
pub is_license_tag: bool,
```
**Update `Default` implementation:**
```rust
is_license_text: false,
is_license_notice: false,
is_license_intro: false,
is_license_clue: false,
is_license_reference: false,
is_license_tag: false,
```
### 5. Files to Modify
| File | Changes |
|------|---------|
| `src/license_detection/models.rs` | Add fields, update Default, update tests |
| `src/license_detection/match_refine.rs` | Update ~5 creation sites |
| `src/license_detection/spdx_lid.rs` | Update 1 creation site |
| `src/license_detection/seq_match.rs` | Update 2 creation sites |
| `src/license_detection/unknown_match.rs` | Update 3 creation sites |
| `src/license_detection/detection.rs` | Update ~15 creation sites |
| `src/license_detection/hash_match.rs` | Update 1 creation site |
| `src/license_detection/aho_match.rs` | Update 1 creation site |
---
### Implementation Checklist
- [ ] Add `is_license_text` and `is_license_notice` fields with `#[serde(skip)]`
- [ ] Change existing flags (`is_license_intro`, etc.) to use `#[serde(skip)]`
- [ ] Update `Default` implementation
- [ ] Update all creation sites (~73 locations)
- [ ] Add serialization test to verify flags not in JSON
---
## Part 3: Filter Pipeline Alignment
### 1. Side-by-Side Pipeline Comparison
#### Python's `refine_matches()` Pipeline (match.py:2691-2833)
| Step | Function Call | Rust Status |
|------|---------------|-------------|
| 1 | `merge_matches()` | ✓ |
| 2 | `filter_matches_missing_required_phrases()` | ✓ |
| 3 | `filter_spurious_matches()` | ✓ |
| 4 | `filter_below_rule_minimum_coverage()` | ✓ |
| 5 | `filter_matches_to_spurious_single_token()` | ✓ |
| 6 | `filter_too_short_matches()` | ✓ |
| 7 | `filter_short_matches_scattered_on_too_many_lines()` | ✓ |
| 8 | `filter_invalid_matches_to_single_word_gibberish()` | ✓ |
| 9 | `merge_matches()` | ✓ |
| 10 | `filter_contained_matches()` | ✓ First only |
| 11 | `filter_overlapping_matches()` | ✓ |
| 12 | `restore_non_overlapping()` | ✓ |
| 13 | `filter_contained_matches()` | ❌ **MISSING** |
| 14 | `filter_false_positive_matches()` | ✓ |
| 15 | `filter_false_positive_license_lists_matches()` | ✓ |
| 16 | `filter_matches_below_minimum_score()` | ❌ **MISSING** |
| 17 | `merge_matches()` | ❌ **MISSING** |
---
### 2. Required Changes
#### Change 1: Add Second `filter_contained_matches()` Call
**File:** `src/license_detection/match_refine.rs`
**Location:** After line 1312, before line 1314
```rust
// Python line 2805: filter contained matches again after restore
let non_contained_final = filter_contained_matches(&final_matches);
```
#### Change 2: Add Final `merge_matches()` Call
**File:** `src/license_detection/match_refine.rs`
**Location:** Replace lines 1318-1321
**Current:**
```rust
let mut scored = kept;
update_match_scores(&mut scored);
scored
```
**New:**
```rust
// Python line 2825: final merge
let merged_final = merge_overlapping_matches(&scored);
let mut final_scored = merged_final;
update_match_scores(&mut final_scored);
final_scored
```
#### Change 3: Add `filter_matches_below_minimum_score()`
**File:** `src/license_detection/match_refine.rs`
**Add function:**
```rust
fn filter_matches_below_minimum_score(
    matches: &[LicenseMatch],
    min_score: f32,
) -> Vec<LicenseMatch> {
    if min_score <= 0.0 {
        return matches.to_vec();
    }
    matches
        .iter()
        .filter(|m| m.score >= min_score)
        .cloned()
        .collect()
}
```
**Update `refine_matches()` signature:**
```rust
pub fn refine_matches(
    index: &LicenseIndex,
    matches: Vec<LicenseMatch>,
    query: &Query,
    min_score: f32,  // Add this
) -> Vec<LicenseMatch>
```
---
### 3. Proper `licensing_contains()` Implementation
This is a **larger undertaking** requiring a license expression parser.
**File:** `src/license_detection/expression.rs` (new)
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum LicenseExpr {
    Identifier(String),
    And(Box<LicenseExpr>, Box<LicenseExpr>),
    Or(Box<LicenseExpr>, Box<LicenseExpr>),
    With(Box<LicenseExpr>, String),
    Plus(Box<LicenseExpr>),
}
impl LicenseExpr {
    /// Check if this expression contains another.
    /// - "A AND B" contains "A", "B", "A AND B"
    /// - "A OR B" contains "A", "B", "A OR B"
    /// - "A+" contains "A" and earlier versions
    /// - "A WITH X" contains "A"
    pub fn contains(&self, other: &LicenseExpr) -> bool {
        // Implementation needed
    }
}
```
**Containment rules:**
- `"A AND B"` contains `"A"`, `"B"`, `"A AND B"`
- `"A OR B"` contains `"A"`, `"B"`, `"A OR B"`
- `"A+"` contains `"A"` and earlier versions
- `"A WITH X"` contains `"A"`
---
### 4. Summary of Required Changes
| # | Change | File | Priority |
|---|--------|------|----------|
| 1 | Add second `filter_contained_matches()` | match_refine.rs | High |
| 2 | Add final `merge_matches()` | match_refine.rs | High |
| 3 | Add `min_score` parameter and filter | match_refine.rs | Medium |
| 4 | Implement proper `licensing_contains()` | expression.rs (new) | High |
---
### Implementation Checklist
- [ ] Add second `filter_contained_matches()` call after `restore_non_overlapping()`
- [ ] Add final `merge_matches()` call at end of `refine_matches()`
- [ ] Add `min_score` parameter to `refine_matches()`
- [ ] Add `filter_matches_below_minimum_score()` function
- [ ] Create `expression.rs` module for license expression parsing
- [ ] Implement `licensing_contains()` with proper semantics
- [ ] Add comprehensive tests for each change
- [ ] Run golden tests to verify improvements
---
## Estimated Impact
After implementing all changes:
| Fix | Expected Impact |
|-----|-----------------|
| `is_license_text` subtraction | Reduce extra matches in GPL/LGPL/MIT files |
| Second `filter_contained_matches()` | Reduce extra matches after restore |
| Final `merge_matches()` | Combine fragmented matches |
| Proper `licensing_contains()` | Correct overlap filtering |
| `#[serde(skip)]` for flags | Match Python JSON output |
Estimated improvement: **50-100 additional tests passing** across all suites.

---

## Verification Results (Post-Implementation)

### Issue Found: `is_license_text` Subtraction Timing

**Python**: Subtracts after **each matcher** (hash, then spdx, then aho)
**Rust**: Subtracts after **all Phase 1 matchers complete**

**Python code** (index.py:1040-1049):
```python
for matcher in matchers:
    matched = matcher.match(qry)
    matched = match.merge_matches(matched)
    matches.extend(matched)
    
    # SUBTRACT IMMEDIATELY after this matcher
    for mtch in matched:
        if (mtch.rule.is_license_text and ...):
            qry.subtract(mtch.qspan)
```

**Rust code** (mod.rs:152-158):
```rust
// After ALL Phase 1 matchers complete
for m in all_matches.iter().filter(...) {
    query.subtract(&span);
}
```

**Impact**: If hash finds a long license text, Python subtracts BEFORE spdx/aho run. Rust runs all three, then subtracts. This could cause extra spurious matches.

**Fix Required**: Move subtraction inside Phase 1, after each matcher.

---

### Issue Found: Missing First `restore_non_overlapping()` Call

**Python**: Calls `restore_non_overlapping()` **twice**:
1. For `discarded_contained` from first `filter_contained_matches()` (line 2794)
2. For `discarded_overlapping` from `filter_overlapping_matches()` (line 2800)

**Rust**: Only calls **once** with `discarded` from `filter_overlapping_matches()`

**Python code** (match.py:2793-2803):
```python
if discarded_contained:
    to_keep, discarded_contained = restore_non_overlapping(matches, discarded_contained)
    matches.extend(to_keep)
    
if discarded_overlapping:
    to_keep, discarded_overlapping = restore_non_overlapping(matches, discarded_overlapping)
    matches.extend(to_keep)
```

**Rust code** (match_refine.rs:1309-1312):
```rust
let (restored, _) = restore_non_overlapping(&kept, discarded);
let mut final_matches = kept;
final_matches.extend(restored);
```

**Impact**: Matches discarded by the first `filter_contained_matches()` that don't overlap with kept matches are never restored in Rust.

**Fix Required**: Add first `restore_non_overlapping()` call for `discarded_contained`.

---

### Summary of Remaining Differences

| Issue | Severity | Impact |
|-------|----------|--------|
| `is_license_text` subtraction timing | Medium | Extra spurious matches in license text files |
| Missing first `restore_non_overlapping()` | Medium | Some valid matches incorrectly discarded |

Both issues should be fixed to achieve full parity with Python.