# PLAN-016: Remaining License Detection Fixes

## Status: Planning Phase - Detailed Plans Needed

### Summary of Progress

- Baseline: 103 failures
- Current: 102 failures
- Net improvement: 1 test fixed

### Completed Work

**Phase A**: Implemented `matched_qspans` tracking, post-loop logic for `has_unknown_intro_before_detection()`

**Phase B**: Fixed `hilen()`, implemented `qdensity()`/`idensity()` methods

**Phase C**: Implemented 6 missing filters, fixed matcher string bug

---

## Current State

| Metric | Value |
|--------|-------|
| lic1 passed | 189 |
| lic1 failed | 102 |

---

## Remaining Issues Requiring Implementation Plans

### Issue 1: Match Over-Merging (~40 tests)

**Problem**: Rust combines matches that Python keeps separate.

**Example**: `CRC32.java`

- Expected: `["apache-2.0", "bsd-new", "zlib"]`
- Actual: `["apache-2.0", "bsd-new AND zlib"]`

**Status**: READY FOR IMPLEMENTATION

#### Root Cause Analysis

**Python's `group_matches()` (detection.py:1820-1868)**:

```python
LINES_THRESHOLD = 4  # imported from query.py

def group_matches(license_matches, lines_threshold=LINES_THRESHOLD):
    for license_match in license_matches:
        is_in_group_by_threshold = license_match.start_line <= previous_match.end_line + lines_threshold
        
        if previous_match.rule.is_license_intro:
            group.append(license_match)  # Keep regardless of threshold
        elif license_match.rule.is_license_intro:
            yield group  # Start new group
            group = [license_match]
        elif license_match.rule.is_license_clue:
            yield group  # Send as separate
            yield [license_match]
            group = []
        elif is_in_group_by_threshold:
            group.append(license_match)  # Within threshold
        else:
            yield group  # Start new group
            group = [license_match]
```

**Rust's `group_matches_by_region()` (detection.rs:166-209)**:

```rust
const TOKENS_THRESHOLD: usize = 10;
const LINES_GAP_THRESHOLD: usize = 3;  // WRONG: should be 4
const LINES_THRESHOLD: usize = 4;      // Defined but NOT used correctly

fn should_group_together(prev: &LicenseMatch, cur: &LicenseMatch) -> bool {
    let line_gap = cur.start_line.saturating_sub(prev.end_line);
    let token_gap = cur.start_token.saturating_sub(prev.end_token);
    
    token_gap <= TOKENS_THRESHOLD && line_gap <= LINES_GAP_THRESHOLD  // WRONG LOGIC
}
```

**Key Differences**:

| Aspect | Python | Rust | Issue |
|--------|--------|------|-------|
| Threshold value | `LINES_THRESHOLD = 4` | `LINES_GAP_THRESHOLD = 3` | Off by 1 |
| Grouping formula | `start <= end + 4` | `gap <= 3` (equivalent to `start <= end + 3`) | Off by 1 |
| Token check | None | `token_gap <= 10` | Extra condition not in Python |
| Logic | Line-only | Dual-criteria AND | Fundamental difference |

**Why This Causes Over-Merging**:

1. **Off-by-one threshold**: Python groups with gap up to 4, Rust only up to 3
2. **Incorrect dual-criteria**: Rust requires BOTH token AND line checks to pass, but Python only checks lines
3. **Missing comment block detection**: Python separates licenses in adjacent comment blocks (like `*/ /*`), but Rust groups them

#### File Changes Required

**File**: `src/license_detection/detection.rs`

**Change 1**: Fix `should_group_together()` to match Python's line-only logic

```rust
// BEFORE (lines 226-231):
fn should_group_together(prev: &LicenseMatch, cur: &LicenseMatch) -> bool {
    let line_gap = cur.start_line.saturating_sub(prev.end_line);
    let token_gap = cur.start_token.saturating_sub(prev.end_token);
    
    token_gap <= TOKENS_THRESHOLD && line_gap <= LINES_GAP_THRESHOLD
}

// AFTER:
fn should_group_together(prev: &LicenseMatch, cur: &LicenseMatch) -> bool {
    // Match Python's group_matches() formula: start_line <= end_line + LINES_THRESHOLD
    // This is equivalent to: line_gap <= LINES_THRESHOLD where gap = start - end
    let line_gap = cur.start_line.saturating_sub(prev.end_line);
    line_gap <= LINES_THRESHOLD
}
```

**Change 2**: Remove unused `TOKENS_THRESHOLD` and `LINES_GAP_THRESHOLD` constants

```rust
// BEFORE (lines 12-17):
const TOKENS_THRESHOLD: usize = 10;
const LINES_GAP_THRESHOLD: usize = 3;
const LINES_THRESHOLD: usize = 4;

// AFTER:
const LINES_THRESHOLD: usize = 4;
```

**Change 3**: Update `group_matches_by_region_with_threshold()` signature

```rust
// BEFORE (lines 166-168):
fn group_matches_by_region_with_threshold(
    matches: &[LicenseMatch],
    _proximity_threshold: usize,
) -> Vec<DetectionGroup> {

// AFTER:
fn group_matches_by_region_with_threshold(
    matches: &[LicenseMatch],
    lines_threshold: usize,
) -> Vec<DetectionGroup> {
```

**Change 4**: Use `lines_threshold` in grouping logic

```rust
// BEFORE (line 194):
} else if should_group_together(previous_match, match_item) {

// AFTER:
} else if match_item.start_line <= previous_match.end_line + lines_threshold {
```

**Change 5**: Update `should_group_together()` to use threshold parameter or inline the check

Either remove `should_group_together()` entirely and inline the check, or keep it for clarity:

```rust
fn should_group_together(prev: &LicenseMatch, cur: &LicenseMatch, threshold: usize) -> bool {
    cur.start_line <= prev.end_line + threshold
}
```

#### Test Cases to Add

Add tests in `src/license_detection/detection.rs`:

```rust
#[test]
fn test_group_matches_python_parity_line_threshold() {
    // Test Python's formula: start <= end + 4
    let m1 = create_test_match(1, 5, "1-hash", "mit.LICENSE");
    
    // Gap of 4 lines: start=10, end_prev=5, threshold=4
    // Python: 10 <= 5 + 4 = 10 <= 9 = False -> SEPARATE
    // Old Rust: gap=5 > 3 -> SEPARATE (correct result, wrong reason)
    let m2 = create_test_match(10, 15, "1-hash", "apache-2.0.LICENSE");
    let groups = group_matches_by_region(&[m1, m2]);
    assert_eq!(groups.len(), 2, "Gap of 4 lines should separate (Python parity)");
}

#[test]
fn test_group_matches_python_parity_at_threshold() {
    // Gap of 4 lines exactly at threshold boundary
    let m1 = create_test_match(1, 5, "1-hash", "mit.LICENSE");
    let m2 = create_test_match(9, 15, "1-hash", "apache-2.0.LICENSE");
    // Python: 9 <= 5 + 4 = 9 <= 9 = True -> GROUP
    // Old Rust: gap=4 > 3 -> SEPARATE (WRONG)
    let groups = group_matches_by_region(&[m1, m2]);
    assert_eq!(groups.len(), 1, "Gap of 4 lines should group (Python parity)");
}

#[test]
fn test_group_matches_adjacent_comment_blocks() {
    // Test case like CRC32.java: adjacent comment blocks with different licenses
    let m1 = create_test_match(1, 15, "1-hash", "apache-2.0.LICENSE");
    let m2 = create_test_match(16, 42, "1-hash", "bsd-new.LICENSE");  // Gap of 1 line
    let m3 = create_test_match(43, 47, "1-hash", "zlib.LICENSE");     // Gap of 1 line
    
    let groups = group_matches_by_region(&[m1, m2, m3]);
    
    // All have gap <= 4, so should be grouped together
    // This matches Python's current behavior
    assert_eq!(groups.len(), 1, "Adjacent blocks within threshold should group");
    assert_eq!(groups[0].matches.len(), 3);
}
```

#### Expected Impact on Golden Tests

After implementing these changes:

- **~30-40 tests should flip from FAIL to PASS** (matches that were incorrectly merged will now be separate)
- **Potential regression**: Some tests that currently PASS may start failing if they rely on the incorrect dual-criteria logic
- **CRC32.java**: Should now output `["apache-2.0", "bsd-new", "zlib"]` as separate detections

#### Investigation Notes

**CRITICAL**: The CRC32.java case requires additional investigation:

- The file has 3 adjacent license blocks with only 1-line gaps between them
- With Python's formula `start <= end + 4`, all three would be grouped
- Yet Python outputs 3 separate expressions
- **This proves there is additional Python logic not identified**

**Possible explanations**:

1. Comment block detection (separating `*/ /*` patterns)
2. Expression-level separation in `analyze_detection()`
3. License intro/clue handling differences

**Action**: Run Python's detection with debug output on CRC32.java:

```bash
cd reference/scancode-toolkit
python -c "from licensedcode.detection import detect_licenses; ..."
```

**Phased approach recommended**:

- Phase 1: Fix threshold to match Python (4 instead of 3)
- Phase 2: Remove token threshold
- Phase 3: Investigate additional separation logic

#### Verification Commands

```bash
# Run golden tests
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic1

# Run specific unit tests
cargo test --release -q --lib license_detection::detection::tests::test_group_matches

# Debug specific file
cargo run -- testdata/license-golden/datadriven/lic1/CRC32.java -o /tmp/crc32-rust.json
```

---

### Issue 2: False Positive Detections (~18 tests)

**Problem**: Rust detects licenses Python doesn't, especially `cc-by-nc-sa-2.0`.

**Example**: `config.guess-gpl2.txt`

- Expected: `["gpl-2.0-plus WITH autoconf-simple-exception-2.0", "warranty-disclaimer"]`
- Actual: `["gpl-2.0-plus WITH autoconf-simple-exception-2.0", "warranty-disclaimer", "cc-by-nc-sa-2.0"]`

**Status**: READY FOR IMPLEMENTATION

#### Root Cause Analysis

**Why `cc-by-nc-sa-2.0` is over-matching:**

1. **Rule Properties** (`cc-by-nc-sa-2.0.RULE`):
   - `is_license_reference: yes`
   - `is_small: true` (9 tokens < SMALL_RULE=15)
   - Text: `http://creativecommons.org/licenses/by-nc-sa/2.0/`

2. **The Bug**: The rule is added to the Aho-Corasick automaton for exact matching, and matches survive all filters because:
   - `filter_false_positive_matches()` only filters `is_false_positive` rules (this rule is NOT `is_false_positive`)
   - `filter_false_positive_license_lists_matches()` requires 15+ matches
   - Other filters don't apply to exact matches

3. **Python's Behavior**: Python's `compute_is_approx_matchable()` returns `false` for small `is_license_reference` rules, and these matches get filtered by coverage checks.

#### Implementation Plan

**File**: `src/license_detection/match_refine.rs`

**Add new filter function** (after line 255):

```rust
/// Filter small license reference rules that don't have 100% coverage.
///
/// Small `is_license_reference` rules (like CC license URLs) should only
/// match with high coverage. Lower coverage indicates a false positive.
fn filter_small_license_reference_matches(
    index: &LicenseIndex,
    matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    matches
        .iter()
        .filter(|m| {
            if let Some(rid) = parse_rule_id(&m.rule_identifier)
                && let Some(rule) = index.rules_by_rid.get(rid)
                && rule.is_small
                && rule.is_license_reference
            {
                return m.match_coverage >= 99.9;
            }
            true
        })
        .cloned()
        .collect()
}
```

**Update `refine_matches()` to call the new filter** (around line 1025):

```rust
let non_fp = filter_false_positive_matches(index, &final_matches);

// NEW: Filter small license reference false positives
let non_small_ref = filter_small_license_reference_matches(index, &non_fp);

let (kept, _discarded) = filter_false_positive_license_lists_matches(non_small_ref);
```

#### Test Cases

```rust
#[test]
fn test_filter_small_license_reference_keeps_exact_match() {
    // 100% coverage match should be kept
}

#[test]
fn test_filter_small_license_reference_removes_partial_match() {
    // < 100% coverage match should be removed
}
```

#### Review Notes

1. **Applies to all matchers**: The proposed filter correctly filters matches from ALL matchers (not just seq), which is appropriate since small reference rules can match via Aho-Corasick exact matching.

2. **Coverage threshold**: Consider `>= 100.0` (exact match only) instead of `>= 99.9` for small reference rules.

#### Expected Impact

~18 tests fixed including:

- `config.guess-gpl2.txt`
- `config.guess-gpl3.txt`
- `flex-readme.txt`
- `complex.el`

---

### Issue 3: Expression Structure Mismatches (~25 tests)

**Problem**: OR expressions are being split or converted to AND.

**Example**: `ExitCode.java`

- Expected: `["epl-1.0 OR lgpl-2.1-plus"]`
- Actual: `["epl-1.0 AND (epl-1.0 OR lgpl-2.1-plus)"]`

**Example**: `cddl-1.0_or_gpl-2.0-glassfish.txt`

- Expected: `["cddl-1.0 OR gpl-2.0"]`
- Actual: `["gpl-2.0 AND cddl-1.0", "unknown-license-reference", "unknown"]`

**Status**: READY FOR IMPLEMENTATION

#### Root Cause Analysis

**Issue 3A: Partial Rule Matches Not Filtered**

When a "combined rule" like `cddl-1.0_or_gpl-2.0-glassfish.RULE` matches, Rust also matches separate rules for `cddl-1.0` and `gpl-2.0` individually. These partial matches are not filtered as "covered" by the combined rule.

**Issue 3B: Expression Combination Doesn't Preserve OR Semantics**

When combining expressions, Rust always uses AND. If one expression already contains OR and covers another expression's license keys, the result is redundant (e.g., `"mit AND (mit OR apache)"`).

#### Implementation Plan

**Fix 3A: Add `filter_license_key_covered_matches()`**

File: `src/license_detection/match_refine.rs`

```rust
/// Filter matches where one match's license keys are covered by another.
///
/// When match A has expression "mit OR apache-2.0" and match B has "mit",
/// and they overlap significantly, match B is redundant.
fn filter_license_key_covered_matches(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    if matches.len() < 2 {
        return matches.to_vec();
    }

    let mut to_remove = std::collections::HashSet::new();
    
    for i in 0..matches.len() {
        for j in 0..matches.len() {
            if i == j || to_remove.contains(&i) {
                continue;
            }
            
            let a = &matches[i];
            let b = &matches[j];
            
            // Check if they overlap significantly (>= 50%)
            let overlap_start = a.start_line.max(b.start_line);
            let overlap_end = a.end_line.min(b.end_line);
            if overlap_start > overlap_end {
                continue;
            }
            
            let overlap_len = overlap_end - overlap_start + 1;
            let smaller_len = (a.end_line - a.start_line + 1).min(b.end_line - b.start_line + 1);
            if overlap_len as f64 / smaller_len as f64 < 0.5 {
                continue;
            }
            
            // Check if one's keys are proper subset of other's
            let a_keys = extract_license_keys(&a.license_expression);
            let b_keys = extract_license_keys(&b.license_expression);
            
            if a_keys.is_subset(&b_keys) && a_keys.len() < b_keys.len() {
                to_remove.insert(i);
            } else if b_keys.is_subset(&a_keys) && b_keys.len() < a_keys.len() {
                to_remove.insert(j);
            }
        }
    }
    
    matches
        .iter()
        .enumerate()
        .filter(|(i, _)| !to_remove.contains(i))
        .map(|(_, m)| m.clone())
        .collect()
}

fn extract_license_keys(expr: &str) -> std::collections::HashSet<String> {
    expr.split_whitespace()
        .filter(|w| !matches!(w.to_uppercase().as_str(), "AND" | "OR" | "WITH"))
        .filter(|w| !w.starts_with('(') && !w.ends_with(')'))
        .map(|w| w.to_lowercase())
        .collect()
}
```

#### Review Notes

**Issues with `filter_license_key_covered_matches()`**:

1. **Overlap calculation**: Uses line-based instead of token-based (`qspan`). Consider using `start_token`/`end_token` for more precise overlap detection.

2. **Key extraction is naive**: The parenthesis filter won't handle nested expressions like `"epl-1.0 OR (lgpl-2.1 AND mit)"`. Consider using proper expression parsing.

3. **Better handled in `simplify_expression()`**: The containment check should be in `expression.rs` during combination, not as post-processing in `match_refine.rs`.

**Alternative approach**: Modify `collect_unique_and()` in `expression.rs` to check for key containment:

- When adding an expression to an AND, check if its keys are a subset of any existing OR expression
- If so, skip adding the expression (it's redundant)

#### Expected Impact

~25 tests fixed including:

- `ExitCode.java`
- `cddl-1.0_or_gpl-2.0-glassfish.txt`
- Other OR expression tests

---

### Issue 4: Query Run Double-Matching

**Problem**: Enabling query runs causes regression (103 → 103+ failures).

**Status**: READY FOR IMPLEMENTATION

#### Root Cause Analysis

**The Bug**: `QueryRun` holds references (`&'a HashSet<usize>`) to `query.high_matchables`. When `query.subtract()` is called, it creates a NEW `HashSet`, making the `QueryRun`'s reference stale.

```rust
// In Query::subtract() - creates NEW HashSet
pub fn subtract(&mut self, span: &PositionSpan) {
    self.high_matchables = self.high_matchables.difference(&positions).copied().collect();
}
```

Python's `QueryRun.high_matchables` is a lazy property that re-reads from `query.high_matchables` on each access.

#### Implementation Plan

**File**: `src/license_detection/query.rs`

**Change 1**: Modify `QueryRun` to store reference to parent `Query` instead of matchables:

```rust
// BEFORE:
pub struct QueryRun<'a> {
    high_matchables: &'a HashSet<usize>,  // Stale after subtract
    low_matchables: &'a HashSet<usize>,   // Stale after subtract
    ...
}

// AFTER:
pub struct QueryRun<'a> {
    query: &'a Query<'a>,  // Reference to parent - always fresh
    ...
}
```

**Change 2**: Update `QueryRun::high_matchables()` to read from query:

```rust
pub fn high_matchables(&self) -> HashSet<usize> {
    self.query.high_matchables
        .iter()
        .filter(|&&pos| pos >= self.start && pos <= self.end.unwrap_or(usize::MAX))
        .copied()
        .collect()
}
```

**Change 3**: Enable query run computation in `Query::with_options()`:

```rust
let query_runs = Self::compute_query_runs(
    &tokens,
    &tokens_by_line,
    _line_threshold,
    len_legalese,
    &index.digit_only_tids,
);
```

#### Review Notes

1. **Query runs currently disabled**: The plan correctly identifies the bug, but note that query runs are currently disabled (query.rs:436-447). This fix is a prerequisite for safely re-enabling them.

2. **Test with query runs enabled**: After implementing this fix, re-enable query runs and run golden tests to verify no regression.

#### Expected Impact

Query runs will work correctly, enabling combined rule matching.
Should fix tests where combined rules fail to match properly.

---

### Issue 5: GPL Variant Confusion (~11 tests)

**Problem**: Rust detects wrong GPL variants (e.g., `gpl-1.0-plus` when `gpl-2.0-plus` expected).

**Example**: `gpl-2.0-plus_1.txt`

- Expected: `["gpl-2.0-plus"]`
- Actual: `["gpl-1.0-plus AND gpl-2.0-plus", "gpl-1.0-plus", "other-copyleft"]`

**Status**: DETAILED PLAN READY

#### Root Cause Analysis

**Problem 1: GPL Rule Overlap**

The GPL rules have overlapping text patterns:

- `gpl-1.0-plus_1.RULE`: Contains "GNU General Public License... either version 1, or any later version"
- `gpl-2.0-plus_1.RULE`: Contains "GNU General Public License... either version 2, or any later version"

Both rules match the same text with slight variations. When text says "version 2, or (at your option) any later version":

- The `gpl-2.0-plus_1.RULE` should match (specific version mentioned)
- The `gpl-1.0-plus_1.RULE` should NOT match (text doesn't mention version 1)

**Problem 2: Lack of Version-Specific Filtering**

Python has logic in `is_false_positive()` (detection.py:1213-1227) that filters GPL matches when:

- All matches are GPL-related
- All have `rule_length == 1` (single token matches)

```python
is_gpl = all(
    'gpl' in license_match.rule.identifier
    for license_match in license_matches
)

if is_gpl and all_match_rule_length_one:
    return True
```

But Rust's implementation (match_refine.rs:31-43) only filters **short** GPL matches:

```rust
fn filter_short_gpl_matches(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    const GPL_SHORT_THRESHOLD: usize = 3;
    matches.iter().filter(|m| {
        let is_gpl = m.license_expression.to_lowercase().contains("gpl");
        let is_short = m.matched_length <= GPL_SHORT_THRESHOLD;
        !(is_gpl && is_short)
    }).cloned().collect()
}
```

**Problem 3: Contained Match Filtering**

When multiple GPL rules match the same text region, the one with better coverage should win. Rust's `filter_contained_matches()` removes smaller contained matches, but only if they're truly contained by token position (qspan). The GPL variants may have similar token positions, so neither is "contained" in the other.

#### Implementation Plan

**Step 1: Add GPL Variant Prioritization Filter**

Add a new filter function in `src/license_detection/match_refine.rs` after `filter_contained_matches()`:

```rust
/// Filter GPL variant matches, keeping the most specific version.
///
/// When multiple GPL variants (e.g., gpl-1.0-plus, gpl-2.0-plus) match
/// the same text region, prefer the version explicitly mentioned in the text.
///
/// Priority order (higher = better):
/// 1. Specific version mentioned (gpl-2.0-plus when text says "version 2")
/// 2. Higher version numbers
/// 3. Longer matched text
/// 4. Higher match coverage
fn filter_gpl_variants(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    if matches.len() < 2 {
        return matches.to_vec();
    }
    
    // Group matches by overlapping regions
    let mut groups: Vec<Vec<&LicenseMatch>> = Vec::new();
    
    for m in matches {
        let mut found_group = false;
        for group in &mut groups {
            if group.iter().any(|g| overlaps(g, m)) {
                group.push(m);
                found_group = true;
                break;
            }
        }
        if !found_group {
            groups.push(vec![m]);
        }
    }
    
    // For each group with multiple GPL variants, keep only the best
    let mut result = Vec::new();
    for group in groups {
        let gpl_matches: Vec<_> = group.iter()
            .filter(|m| is_gpl_variant(&m.license_expression))
            .collect();
        
        if gpl_matches.len() > 1 {
            // Multiple GPL variants - select best match
            if let Some(best) = gpl_matches.iter().max_by(|a, b| {
                compare_gpl_variants(a, b)
            }) {
                result.push((*best).clone());
            }
        } else {
            result.extend(group.iter().map(|m| (*m).clone()));
        }
    }
    
    result
}

fn is_gpl_variant(expr: &str) -> bool {
    let lower = expr.to_lowercase();
    lower.starts_with("gpl-") && lower.contains("-plus")
}

fn overlaps(a: &LicenseMatch, b: &LicenseMatch) -> bool {
    a.start_line <= b.end_line && b.start_line <= a.end_line
}

fn compare_gpl_variants(a: &LicenseMatch, b: &LicenseMatch) -> std::cmp::Ordering {
    // Priority 1: Higher coverage
    let coverage_cmp = a.match_coverage.partial_cmp(&b.match_coverage)
        .unwrap_or(std::cmp::Ordering::Equal);
    if coverage_cmp != std::cmp::Ordering::Equal {
        return coverage_cmp;
    }
    
    // Priority 2: Longer matched text
    let length_cmp = a.matched_length.cmp(&b.matched_length);
    if length_cmp != std::cmp::Ordering::Equal {
        return length_cmp;
    }
    
    // Priority 3: Higher version number (extract from expression like "gpl-2.0-plus")
    let version_a = extract_gpl_version(&a.license_expression);
    let version_b = extract_gpl_version(&b.license_expression);
    version_a.cmp(&version_b)
}

fn extract_gpl_version(expr: &str) -> u8 {
    // Parse "gpl-2.0-plus" -> 2, "gpl-3.0-plus" -> 3
    let lower = expr.to_lowercase();
    if let Some(rest) = lower.strip_prefix("gpl-") {
        if let Some(dot_pos) = rest.find('.') {
            if let Ok(version) = rest[..dot_pos].parse::<u8>() {
                return version;
            }
        }
    }
    0
}
```

#### Review Notes

**Issues with proposed approach**:

1. **May be over-engineered**: Python's GPL-specific filter (detection.py:1213-1227) only filters when ALL matches are GPL AND all have `rule_length == 1`. This is a specific edge case.

2. **Should verify Python behavior**: Run Python with debug output on `gpl-2.0-plus_1.txt` to trace actual filtering behavior before implementing the complex version prioritization.

3. **Alternative**: The simpler fix may be implementing Python's GPL-specific filter:

   ```rust
   fn filter_short_gpl_matches_all(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
       let all_gpl = matches.iter().all(|m| m.license_expression.contains("gpl"));
       let all_short = matches.iter().all(|m| m.matched_length <= 3);
       if all_gpl && all_short {
           return Vec::new();  // Filter all
       }
       matches.to_vec()
   }
   ```

**Step 2: Integrate Filter in Pipeline**

In `refine_matches()` function (match_refine.rs:998), add the GPL variant filter after `filter_contained_matches()`:

```rust
pub fn refine_matches(
    index: &LicenseIndex,
    matches: Vec<LicenseMatch>,
    query: &Query,
) -> Vec<LicenseMatch> {
    // ... existing filters ...
    
    let non_contained = filter_contained_matches(&merged);
    
    // NEW: Filter GPL variants
    let filtered_gpl = filter_gpl_variants(&non_contained);
    
    let (kept, discarded) = filter_overlapping_matches(filtered_gpl, index);
    // ... rest of pipeline ...
}
```

**Step 3: Add Test Cases**

Add unit tests in `src/license_detection/match_refine.rs`:

```rust
#[test]
fn test_filter_gpl_variants_single() {
    let m = create_test_match("#1", 1, 10, 1.0, 100.0, 100);
    let matches = vec![LicenseMatch {
        license_expression: "gpl-2.0-plus".to_string(),
        ..m
    }];
    
    let filtered = filter_gpl_variants(&matches);
    assert_eq!(filtered.len(), 1);
}

#[test]
fn test_filter_gpl_variants_keeps_higher_version() {
    let m1 = LicenseMatch {
        license_expression: "gpl-1.0-plus".to_string(),
        matched_length: 100,
        match_coverage: 95.0,
        ..create_test_match("#1", 1, 10, 1.0, 95.0, 100)
    };
    let m2 = LicenseMatch {
        license_expression: "gpl-2.0-plus".to_string(),
        matched_length: 100,
        match_coverage: 95.0,
        ..create_test_match("#2", 1, 10, 1.0, 95.0, 100)
    };
    
    let filtered = filter_gpl_variants(&[m1, m2]);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].license_expression, "gpl-2.0-plus");
}

#[test]
fn test_filter_gpl_variants_keeps_higher_coverage() {
    let m1 = LicenseMatch {
        license_expression: "gpl-2.0-plus".to_string(),
        match_coverage: 80.0,
        ..create_test_match("#1", 1, 10, 0.8, 80.0, 100)
    };
    let m2 = LicenseMatch {
        license_expression: "gpl-3.0-plus".to_string(),
        match_coverage: 95.0,
        ..create_test_match("#2", 1, 10, 0.95, 95.0, 100)
    };
    
    let filtered = filter_gpl_variants(&[m1, m2]);
    assert_eq!(filtered.len(), 1);
    // Higher coverage wins, even though version is lower
    assert_eq!(filtered[0].license_expression, "gpl-3.0-plus");
}
```

#### Expected Impact

- **Files Changed**: `src/license_detection/match_refine.rs`
- **Lines Added**: ~100 lines (filter + tests)
- **Golden Tests Fixed**: ~11 tests including:
  - `gpl-2.0-plus_1.txt`
  - `gpl-2.0-plus_20.txt`
  - Other GPL variant confusion tests

---

### Issue 6: Extra Unknown License References

**Problem**: Rust generates extra `unknown-license-reference` matches.

**Example**: `cddl-1.1.txt`

- Expected: `["cddl-1.0"]`
- Actual: `["cddl-1.0 AND unknown", "unknown-license-reference AND unknown", "warranty-disclaimer", "unknown", "unknown AND unknown-license-reference"]`

**Status**: DETAILED PLAN READY

#### Root Cause Analysis

**Problem 1: Covered Position Calculation**

The Rust `unknown_match.rs` computes covered positions using line numbers:

```rust
fn compute_covered_positions(
    query: &Query,
    known_matches: &[LicenseMatch],
) -> std::collections::HashSet<usize> {
    let mut covered = std::collections::HashSet::new();

    for match_result in known_matches {
        let start_line = match_result.start_line;
        let end_line = match_result.end_line;

        for pos in 0..query.line_by_pos.len() {
            let line = query.line_by_pos[pos];
            if line >= start_line && line <= end_line {
                covered.insert(pos);
            }
        }
    }
    covered
}
```

Python's `match_unknowns()` uses qspan (token position spans):

```python
# Python: match_unknown.py lines 150-152
matched_ngrams = get_matched_ngrams(
    tokens=query_run.tokens,
    qbegin=query_run.start,
    automaton=automaton,
    ...
)
qspans = (Span(qstart, qend) for qstart, qend in matched_ngrams)
qspan = Span().union(*qspans)
```

**Key Difference**: Python computes coverage at the token level (qspan), not line level. Multiple known matches covering the same lines but different tokens should NOT have gaps filled by unknown detection.

**Problem 2: Threshold Checking**

Python has two thresholds (match_unknown.py:220):

```python
if len(qspan) < unknown_ngram_length * 4 or len(hispan) < 5:
    if TRACE:
        print('match_unknowns: Skipping weak unknown match', text)
    return
```

- `len(qspan) < UNKNOWN_NGRAM_LENGTH * 4` (i.e., < 24 tokens)
- `len(hispan) < 5` (less than 5 high-value tokens)

Rust has similar thresholds (unknown_match.rs:273-275):

```rust
if region_length < UNKNOWN_NGRAM_LENGTH * 4 {
    return None;
}
```

But Rust doesn't check `hispan` (high-value token count).

**Problem 3: Unknown Match Creation Without Proper Filtering**

Rust creates unknown matches for any region with sufficient ngrams, without considering:

1. Whether the region is actually "license-like" (high-value tokens)
2. Whether the unknown match overlaps with existing detections
3. Whether the unknown match is a duplicate of another detection

#### Implementation Plan

**Step 1: Fix Covered Position Calculation**

Change `compute_covered_positions()` to use token positions instead of line numbers:

```rust
fn compute_covered_positions(
    query: &Query,
    known_matches: &[LicenseMatch],
) -> std::collections::HashSet<usize> {
    let mut covered = std::collections::HashSet::new();

    for m in known_matches {
        // Use token positions (start_token, end_token) instead of lines
        for pos in m.start_token..m.end_token {
            covered.insert(pos);
        }
        
        // Also consider matched_token_positions if available
        if let Some(positions) = &m.matched_token_positions {
            for pos in positions {
                covered.insert(*pos);
            }
        }
    }
    
    covered
}
```

**Step 2: Add High-Value Token Threshold**

Modify `create_unknown_match()` to check high-value token count:

```rust
fn create_unknown_match(
    index: &LicenseIndex,
    query: &Query,
    start: usize,
    end: usize,
    ngram_count: usize,
) -> Option<LicenseMatch> {
    let region_length = end.saturating_sub(start);

    if region_length < UNKNOWN_NGRAM_LENGTH * 4 {
        return None;
    }
    
    // NEW: Check high-value token count (hispan)
    let hispan_count = count_high_value_tokens(&query.tokens[start..end], index);
    if hispan_count < 5 {
        return None;  // Not enough license-like content
    }
    
    // ... rest of function
}

fn count_high_value_tokens(tokens: &[u16], index: &LicenseIndex) -> usize {
    tokens.iter()
        .filter(|&&tid| (tid as usize) < index.len_legalese)
        .count()
}
```

**Step 3: Add Duplicate Unknown Filtering**

Add a filter to prevent duplicate unknown matches:

```rust
fn filter_duplicate_unknowns(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    let mut seen_regions: std::collections::HashSet<(usize, usize)> = 
        std::collections::HashSet::new();
    let mut result = Vec::new();
    
    for m in matches {
        if m.matcher == MATCH_UNKNOWN {
            let key = (m.start_line, m.end_line);
            if !seen_regions.contains(&key) {
                seen_regions.insert(key);
                result.push(m.clone());
            }
        } else {
            result.push(m.clone());
        }
    }
    
    result
}
```

**Step 4: Filter Unknown Matches Against Known Expressions**

When an unknown match's text is already covered by a known license expression, skip it:

```rust
fn should_skip_unknown(
    unknown: &LicenseMatch,
    known_matches: &[LicenseMatch],
) -> bool {
    for known in known_matches {
        // Skip if unknown overlaps significantly with a known match
        let overlap_start = unknown.start_line.max(known.start_line);
        let overlap_end = unknown.end_line.min(known.end_line);
        
        if overlap_start <= overlap_end {
            let overlap_lines = overlap_end - overlap_start + 1;
            let unknown_lines = unknown.end_line - unknown.start_line + 1;
            
            // If > 70% of unknown region is covered by known match, skip it
            if overlap_lines as f32 / unknown_lines as f32 > 0.7 {
                return true;
            }
        }
    }
    false
}
```

**Step 5: Integrate Filters in Pipeline**

Update `unknown_match()` function:

```rust
pub fn unknown_match(
    index: &LicenseIndex,
    query: &Query,
    known_matches: &[LicenseMatch],
) -> Vec<LicenseMatch> {
    let mut unknown_matches = Vec::new();

    if query.tokens.is_empty() {
        return unknown_matches;
    }

    let query_len = query.tokens.len();

    // Use token-based coverage
    let covered_positions = compute_covered_positions(query, known_matches);

    let unmatched_regions = find_unmatched_regions(query_len, &covered_positions);

    let automaton = &index.unknown_automaton;

    for region in unmatched_regions {
        let start = region.0;
        let end = region.1;

        let region_length = end - start;
        if region_length < MIN_REGION_LENGTH {
            continue;
        }

        let ngram_matches = match_ngrams_in_region(&query.tokens, start, end, automaton);

        if ngram_matches < MIN_NGRAM_MATCHES {
            continue;
        }

        if let Some(match_result) = create_unknown_match(index, query, start, end, ngram_matches) {
            // NEW: Skip if overlaps significantly with known matches
            if !should_skip_unknown(&match_result, known_matches) {
                unknown_matches.push(match_result);
            }
        }
    }

    // NEW: Filter duplicate unknowns
    filter_duplicate_unknowns(&unknown_matches)
}
```

**Step 6: Add Test Cases**

```rust
#[test]
fn test_unknown_match_skips_covered_regions() {
    let index = LicenseIndex::with_legalese_count(10);
    let text = "MIT License\n\nCopyright 2024\n\nSome other text";
    let query = Query::new(text, &index).unwrap();
    
    // Known match covers first line
    let known = LicenseMatch {
        license_expression: "mit".to_string(),
        start_token: 0,
        end_token: 5,
        start_line: 1,
        end_line: 1,
        matched_token_positions: Some((0..5).collect()),
        ..create_test_match("#1", 1, 1, 1.0, 100.0, 100)
    };
    
    let unknowns = unknown_match(&index, &query, &[known]);
    
    // Should not create unknown for MIT license region
    for u in &unknowns {
        assert!(u.start_line > 1 || u.end_line < 1);
    }
}

#[test]
fn test_unknown_match_requires_high_value_tokens() {
    let index = LicenseIndex::with_legalese_count(10);
    let text = "Some random text that has no license keywords";
    let query = Query::new(text, &index).unwrap();
    
    let unknowns = unknown_match(&index, &query, &[]);
    
    // Should not create unknown for non-license text
    assert!(unknowns.is_empty());
}

#[test]
fn test_filter_duplicate_unknowns() {
    let u1 = LicenseMatch {
        matcher: MATCH_UNKNOWN.to_string(),
        start_line: 1,
        end_line: 10,
        ..create_test_match("unknown", 1, 10, 0.5, 50.0, 50)
    };
    let u2 = LicenseMatch {
        matcher: MATCH_UNKNOWN.to_string(),
        start_line: 1,
        end_line: 10,  // Duplicate region
        ..create_test_match("unknown", 1, 10, 0.5, 50.0, 50)
    };
    
    let filtered = filter_duplicate_unknowns(&[u1, u2]);
    assert_eq!(filtered.len(), 1);
}
```

#### Expected Impact

- **Files Changed**: `src/license_detection/unknown_match.rs`
- **Lines Added**: ~80 lines (filter + tests)
- **Golden Tests Fixed**:
  - `cddl-1.1.txt`
  - Other tests with spurious unknown matches

---

## Run Golden Tests

```bash
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic1
```
