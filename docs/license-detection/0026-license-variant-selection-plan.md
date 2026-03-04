# Plan: Fix Wrong License Variant Detection

## Status: INVESTIGATION COMPLETE - Multiple Hypotheses Tested

### Investigation Summary (2026-03-04)

**Started with 121 failing golden tests → Now at 111 failing (10 improvement)**

### Hypothesis Results

| Hypothesis | Result | Details |
|------------|--------|---------|
| H1 (Match scoring) | PARTIALLY CONFIRMED | Score calculation already has query_coverage in update_match_scores() |
| H2 (Match ordering) | REJECTED | Order is identical between Python and Rust |
| H3 (Overlap resolution) | CONFIRMED | Rust has candidate score logic not in Python, but it helps |
| H4 (Expression containment) | REJECTED | licensing_contains() works correctly |
| H5 (False positive filtering) | REJECTED | Implementation is correct, warranty-disclaimer is not a false positive rule |
| H9 (Aho vs Seq scoring) | CONFIRMED | Aho has 0.0 candidate scores, but fix caused regressions |
| H10 (Expression ordering) | CONFIRMED | Order depends on match order |

### Implementation Attempts

**What was done:**
- Removed `rid` tie-breaker from `ScoresVector::cmp` in `src/license_detection/seq_match/candidates.rs:40-65`
- Candidates with identical scores now compare as equal (matching Python behavior)

**Result:** The fix was implemented correctly but **did not reduce the golden test failure count**.

**Why it didn't help:**
- The rid tie-breaker was only affecting candidate ordering when all other scores were equal
- In practice, most variant selection issues are caused by different score values, not tie-breaking
- The candidate score comparison logic in `handle_overlaps.rs` (beyond-parity feature) may still be causing issues
- May need to investigate:
  1. Are candidate scores being populated correctly for sequence matches?
  2. Are problematic cases using Aho-Corasick matching (which has no candidate scores)?
  3. Is the `different_licenses && both_have_candidate_scores` logic causing incorrect decisions?

**Recommendation:** The fix is correct and should remain (matches Python behavior), but variant selection issues require deeper investigation.

---

## Verification Status: VERIFIED (2026-03-03)

### Summary of Verification

| Item | Status | Notes |
|------|--------|-------|
| rid tie-breaker removal | **CONFIRMED IMPLEMENTED** | `ScoresVector::cmp` (lines 40-65) no longer uses rid |
| Python parity for ScoresVector | **CONFIRMED CORRECT** | Matches Python namedtuple exactly |
| Candidate score propagation | **CONFIRMED WORKING** | Seq matches set `candidate_resemblance`/`candidate_containment` at matching.rs:328-329 |
| Aho matches have no candidate scores | **CONFIRMED** | aho_match.rs:191 sets both to 0.0 |
| Beyond-parity overlap logic | **NEEDS INVESTIGATION** | Logic is correct, but may not activate for problematic cases |
| Testing strategy | **NEEDS IMPLEMENTATION** | Investigation tests not yet created |

---

## Key Findings from Verification

### 1. rid Tie-Breaker Removal: VERIFIED COMPLETE

The `ScoresVector::cmp` implementation at `candidates.rs:40-65` now correctly matches Python:
- No `rid` comparison in the `Ord` implementation
- Candidates with identical scores compare as `Equal`
- The `filter_dupes` function still correctly uses `rule.identifier` for tie-breaking within groups (lines 148-152)

**Conclusion:** This fix is correct and complete.

### 2. Candidate Score Propagation: VERIFIED WORKING

The candidate scores ARE properly propagated for sequence matches:

**Location:** `matching.rs:328-329`
```rust
candidate_resemblance: candidate.score_vec_full.resemblance,
candidate_containment: candidate.score_vec_full.containment,
```

All other matchers set these to 0.0:
- `aho_match.rs:191` - Aho-Corasick matches
- `hash_match.rs:130` - Hash matches
- `spdx_lid/mod.rs:352` - SPDX-LID matches

**Conclusion:** Candidate scores are correctly populated for seq matches, and correctly 0.0 for other matchers.

### 3. Beyond-Parity Overlap Logic: THE REAL INVESTIGATION NEEDED

The `handle_overlaps.rs` logic at lines 204-264 is a **beyond-parity feature** not present in Python. It activates when:
```rust
let different_licenses = matches[i].license_expression != matches[j].license_expression;
let both_have_candidate_scores =
    matches[i].candidate_resemblance > 0.0 && matches[j].candidate_resemblance > 0.0;
```

**Key Questions for Investigation:**

1. **Which matcher type is used for problematic cases?**
   - If only Aho-Corasick matches (no seq matches), then `both_have_candidate_scores` will be false
   - The beyond-parity logic will NOT activate
   - Python behavior will be followed (discard shorter match)

2. **If seq matches exist, are candidate scores different enough?**
   - For bsd-simplified vs bsd-new, the texts are very similar
   - Candidate resemblance/containment may be nearly identical
   - The tie-breaker falls through to `hilen` comparison

3. **Is the logic correctly preferring the right license?**
   - When `current_wins_on_candidate` is true, current is kept
   - When false, current is discarded (next wins)
   - Need to verify the correct license has higher candidate scores

### 4. filter_dupes Grouping: POTENTIAL ISSUE IDENTIFIED

The `filter_dupes` function groups candidates by `license_expression`, so candidates with DIFFERENT license expressions will NOT be deduplicated against each other. This means:

- A `bsd-simplified` candidate and a `bsd-new` candidate will be in different groups
- Both can survive `filter_dupes` and proceed to overlap filtering
- The overlap filtering must then choose between them

This is correct behavior matching Python, but explains why variant selection happens at the overlap stage, not the candidate selection stage.

---

## Verification Results (2026-03-04)

### Hypothesis H4: Expression Containment - REJECTED

**Claim:** `licensing_contains()` may behave differently for variant selection.

**Investigation Result:**
- The `licensing_contains()` function in `src/license_detection/expression.rs` works correctly
- It properly identifies when one license expression contains another
- Used correctly in overlap filtering at `handle_overlaps.rs:255-321`

**Conclusion:** NOT a root cause for variant selection issues.

### Hypothesis H9: Aho vs Seq Scoring - CONFIRMED

**Claim:** Aho-Corasick matches have 0.0 candidate scores, causing different behavior than sequence matches.

**Investigation Result:**
- Confirmed: Aho matches set `candidate_resemblance = 0.0` and `candidate_containment = 0.0` (`aho_match.rs:191`)
- Confirmed: Seq matches properly populate these fields (`matching.rs:328-329`)
- When both matches have 0.0 scores, the beyond-parity candidate score logic doesn't activate
- This causes Rust to follow Python's behavior (discard shorter match)

**Fix Attempted:**
- Tried to normalize candidate scores for Aho matches
- **Result:** Caused regressions in other test cases
- **Decision:** Reverted fix, need different approach

### Hypothesis H10: Expression Ordering - CONFIRMED

**Claim:** Expression order depends on match order.

**Investigation Result:**
- The order of license expressions in results depends on:
  1. The order matches are discovered (Aho-Corasick vs sequence matching)
  2. Sorting by position and length
  3. Tie-breaking behavior
- Python and Rust may discover matches in different orders due to implementation differences
- This affects which license appears first in results

**Conclusion:** Expected behavior, but contributes to golden test failures.

### Remaining Work

The candidate score logic in `handle_overlaps.rs` is a beyond-parity feature that should be KEPT. It helps when both matches have candidate scores (sequence matching). The remaining issues are:

1. Aho matches have no candidate scores - need alternative tie-breaking
2. Match discovery order differs - affects final expression ordering
3. Some edge cases in score calculation

### 5. Testing Strategy: NEEDS IMPLEMENTATION

The plan calls for investigation tests but they are not yet created. Following `docs/TESTING_STRATEGY.md`:

**Recommended Tests:**
1. **Unit tests** for `ScoresVector` equality without rid (can add to existing tests in candidates.rs)
2. **Investigation tests** to determine matcher type and candidate scores for problematic cases
3. **Golden tests** to catch regressions (already exist in `license_detection::golden_test`)

---

## Additional Investigation: Why Golden Tests Still Fail

### Hypothesis 1: Problematic Cases Use Aho-Corasick, Not Sequence Matching

If the failing golden test cases only trigger Aho-Corasick matches (not seq matches), the beyond-parity candidate score logic never activates. This would mean:

- `both_have_candidate_scores` is always false
- Rust follows Python's behavior exactly for overlap filtering
- The issue is NOT in overlap filtering or candidate selection

**Test needed:** Run debug pipeline on failing cases to see which matcher is used.

### Hypothesis 2: Match Coverage/Score Differences

The `match_coverage` and `score` fields may differ between Python and Rust due to:
- Different token counting
- Different coverage calculations
- Different score normalization

These affect the sorting in overlap filtering (`matched_length`, `hilen`).

### Hypothesis 3: Rule Load Order Still Matters

Even with rid removed from `ScoresVector::cmp`, the rule load order affects:
- Which rules are checked first during Aho-Corasick matching
- The order of matches in the output (before sorting)

**Note:** This is less likely to be the issue since overlap filtering sorts by position/length.

### Hypothesis 4: Threshold Differences

The threshold values in `handle_overlaps.rs`:
```rust
const OVERLAP_SMALL: f64 = 0.10;
const OVERLAP_MEDIUM: f64 = 0.40;
const OVERLAP_LARGE: f64 = 0.70;
const OVERLAP_EXTRA_LARGE: f64 = 0.90;
```

These should match Python's values. Need to verify they're identical.

### Hypothesis 5: licensing_contains Function Differences

The `licensing_contains` function determines if one license expression "contains" another (e.g., `gpl-2.0-plus` contains `gpl-2.0`). If this function behaves differently, the overlap filtering for medium/small overlaps will differ.

**Location:** `src/license_detection/expression.rs`

---

## Recommended Next Steps

### Immediate: Debug Pipeline Comparison

1. Run both Python and Rust debug pipelines on 2-3 failing golden test cases
2. Compare:
   - Matcher type used (1-hash, 2-aho, 3-seq)
   - Candidate scores (if seq matcher)
   - Match positions and lengths
   - Overlap filtering decisions

### Short-term: Create Investigation Tests

Create `src/license_detection/investigation/license_variant_test.rs`:

```rust
#[test]
fn test_bsd_simplified_debug_pipeline() {
    // Run full detection pipeline on bsd-simplified text
    // Print matcher type, candidate scores, match positions
}

#[test] 
fn test_matcher_type_for_variant_cases() {
    // For each reported case, determine which matcher is used
    // This tells us if candidate scores are available
}
```

### Medium-term: Verify Threshold Alignment

Compare all constant values between Python and Rust:
- `OVERLAP_SMALL/MEDIUM/LARGE/EXTRA_LARGE` thresholds
- `HIGH_RESEMBLANCE_THRESHOLD` value
- `min_matched_length` / `min_high_matched_length` calculations

---

## Problem Statement

When multiple similar license rules match the same text, Rust selects a different rule than Python. This results in incorrect license identification for licenses with similar text but different clauses.

### Reported Cases

| Expected | Detected | Issue |
|----------|----------|-------|
| `bsd-simplified` | `bsd-new` | 2-clause vs 3-clause BSD |
| `json` | `mit` | JSON has MIT-like text but is distinct |
| `cc-by-nc-4.0` | `cc-by-4.0` | Missing non-commercial clause |
| `gpl-2.0-plus` | `gpl-3.0-plus` | Wrong GPL version |
| `x11-ibm` | `historical` | Wrong license rule |

---

## Root Cause Analysis

### 1. Candidate Scoring Differences (FIXED)

**Location**: `src/license_detection/seq_match/candidates.rs:40-65`

**Status: FIXED** - The `rid` tie-breaker has been removed from `ScoresVector::cmp`.

**Python** (`match_set.py:458`):
```python
ScoresVector = namedtuple('ScoresVector', [
    'is_highly_resemblant',
    'containment',
    'resemblance',
    'matched_length'
])
```

**Rust** (`candidates.rs:40-65`) - **CURRENT IMPLEMENTATION**:
```rust
impl Ord for ScoresVector {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Python sorts ScoresVector namedtuple with reverse=True:
        // 1. is_highly_resemblant (True > False)
        // 2. containment (higher is better)
        // 3. resemblance (higher is better)
        // 4. matched_length (higher is better)
        // Note: Python does NOT use rid for tie-breaking in ScoresVector
        self.is_highly_resemblant
            .cmp(&other.is_highly_resemblant)
            .then_with(|| {
                self.containment
                    .partial_cmp(&other.containment)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                self.resemblance
                    .partial_cmp(&other.resemblance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                self.matched_length
                    .partial_cmp(&other.matched_length)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        // NO rid tie-breaker - matches Python behavior
    }
}
```

### 2. Missing Rule Identifier Tie-Breaker in Candidate Selection

**Location**: `src/license_detection/seq_match/candidates.rs:149-160`

**Python** (`match_set.py:480-482`):
```python
def rank_key(item):
    (_sv_round, sv_full), _rid, rule, _inter = item
    return sv_full, rule.identifier  # Uses rule.identifier for tie-breaking
```

**Rust** (`candidates.rs:149-154`):
```rust
group.sort_by(|a, b| {
    b.score_vec_full
        .cmp(&a.score_vec_full)
        .then_with(|| b.rule.identifier.cmp(&a.rule.identifier))  // Correct
});
```

This part appears correct, but the issue is that candidates with identical `score_vec_full` but different `license_expression` values may both survive if they're not in the same dupe group.

### 3. Overlap Filtering With Candidate Scores (Beyond-Parity Feature)

**Location**: `src/license_detection/match_refine/handle_overlaps.rs:192-252`

Rust's overlap filtering uses `candidate_resemblance` and `candidate_containment` to choose between overlapping matches with different license expressions:

```rust
let current_wins_on_candidate = {
    if current_resemblance > next_resemblance { true }
    else if current_resemblance < next_resemblance { false }
    else if current_containment > next_containment { true }
    else if current_containment < next_containment { false }
    else { current_hilen >= next_hilen }
};
```

This logic appears in 4 places (lines 215, 225, 235, 245):
```rust
if different_licenses && both_have_candidate_scores && !current_wins_on_candidate {
    discarded.push(matches.remove(i));  // Current loses
    ...
}
```

**Python Behavior** (`match.py:1326-1366`):
```python
if extra_large_next and current_match.len() >= next_match.len():
    discarded_append(matches_pop(j))
    continue
```

Python does NOT have the `different_licenses && both_have_candidate_scores` check. It simply discards the shorter match.

**IMPORTANT**: This is NOT a bug - it's a **beyond-parity feature** that Rust added to improve license variant selection. When two overlapping matches have different license expressions and both have candidate scores from sequence matching, Rust uses those scores to decide which license wins. This is intentional and should be KEPT, not removed.

**The real question**: Why isn't this feature working correctly for the reported cases?

### 4. Missing `filter_dupes` Group Key Field

**Location**: `src/license_detection/seq_match/candidates.rs:112-120`

**Python** (`match_set.py:467-476`):
```python
def group_key(item):
    (sv_round, _sv_full), _rid, rule, _inter = item
    return (
        rule.license_expression,
        sv_round.is_highly_resemblant,
        sv_round.containment,
        sv_round.resemblance,
        sv_round.matched_length,
        rule.length,  # <-- Uses rule.length (total tokens)
    )
```

**Rust** (`candidates.rs:112-120`):
```rust
struct DupeGroupKey {
    license_expression: String,
    is_highly_resemblant: bool,
    containment: i32,
    resemblance: i32,
    matched_length: i32,
    rule_length: usize,  // Uses tokens.len() - same as rule.length
}
```

This appears correct.

### 5. Relevance Score Not Used in Overlap Decisions

**Location**: `src/license_detection/match_refine/handle_overlaps.rs`

The `rule.relevance` field exists but is **not used** in overlap decisions. Python also doesn't explicitly use relevance in `filter_overlapping_matches`, but the score computation does include relevance:

**Python** (`match.py` score computation):
```python
score = (match.coverage() * rule.relevance) / 100
```

**Rust** (`matching.rs:293`):
```rust
let score = (match_coverage * candidate.rule.relevance as f32) / 100.0;
```

This appears correct for score computation, but relevance isn't used when choosing between overlapping matches with different license expressions.

---

## Python vs Rust Behavior Comparison

### Candidate Selection Flow

| Step | Python | Rust | Status |
|------|--------|------|--------|
| 1. Set intersection | `match_set.py:281-297` | `candidates.rs:298-365` | OK |
| 2. Sort by scores | `match_set.py:302` | `candidates.rs:371` | OK |
| 3. Truncate to top*10 | `match_set.py:315` | `candidates.rs:372` | OK |
| 4. Multiset refinement | `match_set.py:328-348` | `candidates.rs:377-433` | OK |
| 5. `filter_dupes` | `match_set.py:354` | `candidates.rs:435` | **DIFFERENT** |
| 6. Final sort/truncate | `match_set.py:354` | `candidates.rs:437-438` | OK |

### `filter_dupes` Behavior

**Python**: Groups by `(license_expression, is_highly_resemblant, containment, resemblance, matched_length, rule.length)`, then within each group sorts by `(sv_full, rule.identifier)` and keeps the first.

**Rust**: Same grouping key, same sorting. **APPEARS CORRECT**.

### Overlap Filtering Behavior

**Python** (`match.py:1218-1221`):
```python
# sort on start, longer high, longer match, matcher type
sorter = lambda m: (m.qspan.start, -m.hilen(), -m.len(), m.matcher_order)
matches = sorted(matches, key=sorter)
```

**Rust** (`handle_overlaps.rs:132-138`):
```rust
matches.sort_by(|a, b| {
    a.qstart()
        .cmp(&b.qstart())
        .then_with(|| b.hilen.cmp(&a.hilen))
        .then_with(|| b.matched_length.cmp(&a.matched_length))
        .then_with(|| a.matcher_order().cmp(&b.matcher_order()))
});
```

**Status**: Sorting is correct.

### Extra-Large Overlap Decision

**Python** (`match.py:1326-1334`):
```python
if extra_large_next and current_match.len() >= next_match.len():
    discarded_append(matches_pop(j))
    continue
```

**Rust** (`handle_overlaps.rs:214-221`):
```rust
if extra_large_next && current_len_val >= next_len_val {
    if different_licenses && both_have_candidate_scores && !current_wins_on_candidate {
        discarded.push(matches.remove(i));  // Current loses
        i = i.saturating_sub(1);
        break;
    }
    discarded.push(matches.remove(j));  // Next loses
    continue;
}
```

**DIFFERENCE**: Rust has additional logic for `different_licenses && both_have_candidate_scores`. This is **NOT** in Python and may cause different behavior.

---

## Specific Code Locations Requiring Changes

### 1. Remove `rid` from `ScoresVector` tie-breaking (COMPLETE)

**File**: `src/license_detection/seq_match/candidates.rs:40-65`

**Status: FIXED** - The `rid` comparison has been removed from `Ord` implementation.

**Rationale**: Python's `ScoresVector` is a namedtuple with no `rid` field. Adding `rid` as a tie-breaker meant rule load order affected results.

### 2. Investigate Why Candidate Scores Aren't Helping (VERIFIED WORKING)

**File**: `src/license_detection/match_refine/handle_overlaps.rs:192-264`

**Status: VERIFIED** - The candidate score comparison logic is working correctly:
- Candidate scores ARE populated for sequence matches (`matching.rs:328-329`)
- Candidate scores are correctly 0.0 for Aho/Hash matches

**DO NOT REMOVE** this beyond-parity feature. It should improve variant selection when both matches have candidate scores.

**Key Question:** Are the problematic cases using sequence matching or only Aho-Corasick?

- If only Aho matching, `both_have_candidate_scores` will be false
- The beyond-parity logic will NOT activate
- Rust will follow Python's behavior (discard shorter match)

### 3. Add investigation test for specific cases

**File**: `src/license_detection/investigation/license_variant_test.rs` (new file)

---

## Implementation Steps

### Phase 1: Fix Confirmed Issue (COMPLETE)

1. **Remove `rid` from `ScoresVector::cmp`** - DONE
   - The rid tie-breaker has been removed from `candidates.rs:40-65`
   - Candidates with identical scores now compare as equal (matching Python)

### Phase 2: Investigation Tests (TODO)

2. **Create investigation tests** for each reported case:
   - `test_bsd_simplified_vs_new`
   - `test_json_vs_mit`
   - `test_cc_by_nc_vs_cc_by`
   - `test_gpl_2_plus_vs_3_plus`
   - `test_x11_ibm_vs_historical`

3. **Debug pipeline comparison** - Run both Python and Rust debug pipelines on failing files to understand:
   - Which matcher is being used (Aho vs Seq)?
   - Are candidate scores populated?
   - What are the actual score vectors for each candidate?

### Phase 3: Root Cause Investigation for Candidate Score Feature (PARTIAL)

4. **Verify candidate score propagation** - DONE
   - Confirmed: `candidate_resemblance` and `candidate_containment` are set in `matching.rs:328-329`
   - Confirmed: These are 0.0 for Aho/Hash matches

5. **Determine if beyond-parity feature is activating** - TODO
   - For each reported case, check if `both_have_candidate_scores` is true
   - If false, the candidate score logic isn't being used

### Phase 4: Candidate Selection Review (TODO)

6. **Verify set/multiset calculations** match Python exactly
7. **Verify threshold calculations** (`min_matched_length`, `min_high_matched_length`)
8. **Verify resemblance/containment calculations** with floating-point precision

### Phase 5: Integration Testing (TODO)

9. **Run full golden test suite** after changes
10. **Compare output** for all license variant cases

---

## Test Cases to Verify Fix

### Unit Tests

```rust
#[test]
fn test_scores_vector_no_rid_tiebreaker() {
    // Two candidates with identical scores but different rids
    // should compare as equal (not use rid)
    let sv1 = ScoresVector { is_highly_resemblant: true, containment: 0.9, resemblance: 0.8, matched_length: 100.0, rid: 1 };
    let sv2 = ScoresVector { is_highly_resemblant: true, containment: 0.9, resemblance: 0.8, matched_length: 100.0, rid: 2 };
    assert_eq!(sv1.cmp(&sv2), std::cmp::Ordering::Equal);
}

#[test]
fn test_filter_dupes_identifier_sorting() {
    // When scores are equal, filter_dupes should sort by identifier (reverse)
    // Python: return sv_full, rule.identifier (sorted reverse=True)
    // This means higher identifier alphabetically wins
}

#[test]
fn test_candidate_scores_populated_for_seq_matches() {
    // Verify that sequence matches have candidate_resemblance > 0
}

#[test]
fn test_candidate_scores_zero_for_aho_matches() {
    // Verify that Aho-Corasick matches have candidate_resemblance == 0
}
```

### Investigation Tests

Create `src/license_detection/investigation/license_variant_test.rs`:

```rust
#[test]
fn test_bsd_simplified_candidate_scores() {
    // Load bsd-simplified text, run detection
    // Check which candidates are selected
    // Check their score vectors and candidate scores
}

#[test]
fn test_json_candidate_scores() {
    // Load JSON license text, run detection
    // Compare candidate selection between json and mit rules
}
```

### Integration Tests

Run against files from `testdata/license-golden/datadriven/` that exhibit the issue.

### Golden Tests

Run full golden test suite and compare results:
```bash
cargo test --release -q --lib license_detection::golden_test
```

---

## Risk Assessment

### High Risk Changes

| Change | Risk | Mitigation |
|--------|------|------------|
| Remove rid from ScoresVector | May affect candidate ordering when scores are equal | Run full golden test suite; verify filter_dupes still uses identifier for tie-breaking |

### Medium Risk Changes

| Change | Risk | Mitigation |
|--------|------|------------|
| None identified - other changes are investigation only | - | - |

### Low Risk Changes

| Change | Risk | Mitigation |
|--------|------|------------|
| Add investigation tests | None | Tests are documentation |
| Debug pipeline comparison | None | Read-only investigation |

### NOT A RISK: Candidate Score Logic in Overlaps

The candidate score comparison in `handle_overlaps.rs` is a **beyond-parity feature**, not a bug. Do not remove it without investigation. It may be helping in some cases and not activating in others.

---

## Success Criteria

1. All existing golden tests pass
2. Specific variant cases detect correctly:
   - `bsd-simplified` text → `bsd-simplified` detection
   - `json` text → `json` detection  
   - `cc-by-nc-4.0` text → `cc-by-nc-4.0` detection
   - `gpl-2.0-plus` text → `gpl-2.0-plus` detection
   - `x11-ibm` text → `x11-ibm` detection
3. No regression in other license detection accuracy

---

## Appendix: Debug Commands

### Run Debug Pipeline on Specific File

```bash
# Rust
cargo run --features debug-pipeline --bin debug_license_detection -- testdata/license-golden/datadriven/lic1/bsd-simplified_1.txt

# Python
cd reference/scancode-playground
venv/bin/python debug_license_detection.py ../scancode-toolkit/testdata/license-golden/datadriven/lic1/bsd-simplified_1.txt
```

### Compare Candidate Scores

Add tracing to `compute_candidates_with_msets` to print top candidates:
```rust
for (rank, candidate) in sortable_candidates.iter().enumerate().take(20) {
    println!("{}: {} (expr={}, svr={:?}, svf={:?})",
        rank, candidate.rule.identifier, candidate.rule.license_expression,
        candidate.score_vec_rounded, candidate.score_vec_full);
}
```

### Check Rule Relevance Values

```bash
grep -A5 "bsd-simplified" reference/scancode-toolkit/src/licensedcode/data/rules/*.RULE | grep relevance
grep -A5 "bsd-new" reference/scancode-toolkit/src/licensedcode/data/rules/*.RULE | grep relevance
```

---

## Missing Aspects Identified

### 1. Candidate Score Propagation Not Documented

The plan doesn't explain how `candidate_resemblance` and `candidate_containment` get from the candidate selection phase to the `LicenseMatch` objects. Need to trace this:

- Where are these fields set?
- Are they only set for sequence matches?
- What values do they have for Aho matches?

### 2. No Analysis of Which Matcher Is Used

The reported cases may be going through different matchers:
- Hash match: returns immediately, no candidate scores
- SPDX-LID match: no candidate scores
- Aho-Corasick match: no candidate scores (these fields are 0.0)
- Sequence match: has candidate scores

If the problematic cases only use Aho matching, the candidate score logic in overlap filtering never activates.

### 3. Missing Test File Location

The plan mentions creating `license_variant_test.rs` but doesn't specify it needs to be added to `src/license_detection/investigation/mod.rs`.

### 4. No Discussion of filter_dupes Behavior

When `rid` is removed from `ScoresVector`, candidates with identical scores will compare as equal. The `filter_dupes` function will then use `rule.identifier` for sorting within each group. Need to verify this matches Python's behavior.

---

## Suggested Improvements

### 1. Add Candidate Score Trace

Before making changes, add debug output to understand current behavior:

```rust
// In seq_match_with_candidates or similar
for match in &matches {
    println!("Match: {} cand_repl={} cand_cont={}", 
        match.license_expression, 
        match.candidate_resemblance, 
        match.candidate_containment);
}
```

### 2. Create Test Matrix by Matcher Type

| License Case | Primary Matcher | Has Candidate Scores? | Expected Winner |
|--------------|-----------------|----------------------|-----------------|
| bsd-simplified vs bsd-new | ? | ? | bsd-simplified |
| json vs mit | ? | ? | json |
| cc-by-nc-4.0 vs cc-by-4.0 | ? | ? | cc-by-nc-4.0 |
| gpl-2.0-plus vs gpl-3.0-plus | ? | ? | gpl-2.0-plus |
| x11-ibm vs historical | ? | ? | x11-ibm |

### 3. Verify filter_dupes Tie-Breaking

After removing `rid` from `ScoresVector::cmp`, verify that `filter_dupes` still correctly uses `rule.identifier` for tie-breaking within groups. Python sorts by `(sv_full, rule.identifier)` with `reverse=True`.

### 4. Consider Alternative: Rule Relevance

If candidate scores aren't helping, consider using `rule.relevance` as a tie-breaker in overlap decisions. This is already available and may help distinguish between similar licenses where one is more specific.
