# Plan: Fix Wrong License Variant Detection

## Verification Status: NEEDS_IMPROVEMENT

### Summary of Verification

| Item | Status | Notes |
|------|--------|-------|
| Root Cause #1 (rid tie-breaking) | **ACCURATE** | Python has no rid in ScoresVector |
| Root Cause #3 (overlap candidate scores) | **NEEDS_REVISION** | This is beyond-parity, not a bug |
| Code locations | **ACCURATE** | Line numbers verified correct |
| Python reference comparison | **PARTIALLY ACCURATE** | Missing context on intentional differences |
| Testing strategy | **NEEDS EXPANSION** | Should test both parity and beyond-parity |

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

### 1. Candidate Scoring Differences (Primary Issue)

**Location**: `src/license_detection/seq_match/candidates.rs:32-67`

The `ScoresVector` comparison in Rust differs from Python in a critical way:

**Python** (`match_set.py:458`):
```python
ScoresVector = namedtuple('ScoresVector', [
    'is_highly_resemblant',
    'containment',
    'resemblance',
    'matched_length'
])
```

**Rust** (`candidates.rs:40-67`):
```rust
impl Ord for ScoresVector {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.is_highly_resemblant
            .cmp(&other.is_highly_resemblant)
            .then_with(|| self.containment.partial_cmp(&other.containment)...)
            .then_with(|| self.resemblance.partial_cmp(&other.resemblance)...)
            .then_with(|| self.matched_length.partial_cmp(&other.matched_length)...)
            .then_with(|| self.rid.cmp(&other.rid))  // <-- PROBLEM: rid used for tie-breaking
    }
}
```

**Issue**: Rust adds `rid` as a tie-breaker, but this means the **order rules are loaded** affects which rule wins. Python does NOT use rid for tie-breaking in `ScoresVector`.

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

### 1. Remove `rid` from `ScoresVector` tie-breaking (CONFIRMED)

**File**: `src/license_detection/seq_match/candidates.rs:40-67`

**Change**: Remove the `rid` comparison from `Ord` implementation:

```rust
impl Ord for ScoresVector {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.is_highly_resemblant
            .cmp(&other.is_highly_resemblant)
            .then_with(|| self.containment.partial_cmp(&other.containment).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| self.resemblance.partial_cmp(&other.resemblance).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| self.matched_length.partial_cmp(&other.matched_length).unwrap_or(std::cmp::Ordering::Equal))
        // REMOVE: .then_with(|| self.rid.cmp(&other.rid))
    }
}
```

**Rationale**: Python's `ScoresVector` is a namedtuple with no `rid` field. Adding `rid` as a tie-breaker means rule load order affects results.

### 2. Investigate Why Candidate Scores Aren't Helping (NOT A BUG TO FIX)

**File**: `src/license_detection/match_refine/handle_overlaps.rs:192-252`

**DO NOT REMOVE** the candidate score comparison logic. This is a beyond-parity feature that should improve variant selection. Instead, investigate:

1. Are candidate scores being populated correctly for sequence matches?
2. Are the problematic cases going through sequence matching or only Aho-Corasick?
3. If only Aho-Corasick, candidate_resemblance/containment will be 0.0

**Investigation Needed**:
- Check if reported cases (bsd-simplified, json, cc-by-nc-4.0, etc.) use sequence matching
- If they only use Aho matching, the candidate score logic never activates
- May need to propagate candidate scores from candidate selection to the match objects

### 3. Add investigation test for specific cases

**File**: `src/license_detection/investigation/license_variant_test.rs` (new file)

---

## Implementation Steps

### Phase 1: Fix Confirmed Issue (High Priority)

1. **Remove `rid` from `ScoresVector::cmp`** - This is the confirmed cause of variant selection issues when scores are equal

### Phase 2: Investigation Tests

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

### Phase 3: Root Cause Investigation for Candidate Score Feature

4. **Verify candidate score propagation**:
   - Check `candidate_resemblance` and `candidate_containment` fields in `LicenseMatch`
   - Verify these are set during sequence matching
   - Check if they're 0.0 for Aho-only matches

5. **Determine if beyond-parity feature is activating**:
   - For each reported case, check if `both_have_candidate_scores` is true
   - If false, the candidate score logic isn't being used

### Phase 4: Candidate Selection Review

6. **Verify set/multiset calculations** match Python exactly
7. **Verify threshold calculations** (`min_matched_length`, `min_high_matched_length`)
8. **Verify resemblance/containment calculations** with floating-point precision

### Phase 5: Integration Testing

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
