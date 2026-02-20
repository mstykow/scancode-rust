# PLAN-026: Fix Short License Reference Detection

**Date**: 2026-02-20
**Status**: Investigation Needed - Root Cause Hypothesis Refined
**Priority**: 3 (Pattern C from PLAN-023)
**Impact**: 3 complete detection failures + multiple related issues in lic4

## Executive Summary

Rust returns empty detections `[]` where Python successfully detects licenses for short license references and modified license text. This is **Pattern C** from the failure analysis, causing complete detection failures.

| Test File | Rust Result | Python Result |
|-----------|-------------|---------------|
| `lic4/isc_only.txt` | `[]` | `isc` |
| `lic4/lgpl_21.txt` | `[]` | `lgpl-2.0-plus` |
| `lic4/warranty-disclaimer_1.txt` | `[]` | `warranty-disclaimer` |

---

## Context: Recent Changes

This plan was created before PLAN-024 and PLAN-028 were implemented. Since then:

1. **PLAN-028** (UTF-8/binary handling) - Completed
   - Added `read_test_file_content()` with binary detection
   - Uses `content_inspector` for file type detection
   - Result: +24 tests passing

2. **PLAN-024** (Distance-based merging) - Completed
   - Added `ispan_bounds()`, `idistance_to()`, `is_after()` methods
   - Rewrote `merge_overlapping_matches()` with distance thresholds
   - Result: +40 tests passing

3. **Current golden test status**: 51 failures in lic4 (299/350 passing)

The short license reference issue remains unresolved - these files still return `[]`.

---

## Root Cause Analysis (Updated 2026-02-20)

### Finding 1: Filter Logic is Correct

Both Python and Rust filter implementations are equivalent:

**Python** (`match.py:1706-1737`):

- Only filters `MATCH_SEQ` (sequence matches)
- Uses `is_small()` with two conditions:
  - CASE 1: `matched_len < min_matched_len OR high_matched_len < min_high_matched_len`
  - CASE 2: `rule.is_small AND coverage < 80`

**Rust** (`match_refine.rs:63-85`):

- Identical logic: filters only `"3-seq"` matches
- Same `is_small()` conditions

**Conclusion**: Filtering is NOT the root cause.

### Finding 2: Small Reference Rules ARE in Aho-Corasick Automaton

From `builder.rs:256-261`:

```rust
// Only add non-empty patterns to the automaton
if !rule_token_ids.is_empty() {
    rules_automaton_patterns.push(tokens_to_bytes(&rule_token_ids));
    pattern_id_to_rid.push(rid);
}
```

Small reference rules like `lgpl_bare_single_word.RULE` (text: `LGPL`) **ARE** added to the automaton.

**Verified**: Rule token `lgpl` is assigned a token ID via `dictionary.get_or_assign()` at line 235.

### Finding 3: Weak Rules Are NOT Sequence Matchable

From `builder.rs:273-277` and `builder.rs:167-173`:

```rust
let is_approx_matchable = {
    rule.is_small = rule_length < SMALL_RULE;
    rule.is_tiny = rule_length < TINY_RULE;
    compute_is_approx_matchable(&rule)
};

fn compute_is_approx_matchable(rule: &Rule) -> bool {
    !(rule.is_false_positive
        || rule.is_required_phrase
        || rule.is_tiny
        || rule.is_continuous
        || (rule.is_small && (rule.is_license_reference || rule.is_license_tag)))
}
```

Rules like `lgpl_bare_single_word.RULE`:

- `is_license_reference: yes`
- `is_small: true` (1 token < SMALL_RULE = 15)
- `is_weak: true` (token `lgpl` is not a legalese word)
- `is_approx_matchable: false` (because small + license_reference)

**This is by design in Python** - these rules should be found via Aho-Corasick, not sequence matching.

### Finding 4: Token Dictionary Population

The dictionary is populated in order:

1. Legalese words (IDs 0 to len_legalese-1) - from `legalese::get_legalese_words()`
2. Tokens from all rules during indexing via `dictionary.get_or_assign()`

**Key insight**: Tokens like `lgpl`, `isc` are NOT in the legalese dictionary, but ARE added during rule indexing when processing rules like `lgpl_bare_single_word.RULE`.

### Finding 5: Query Tokenization (CRITICAL)

From `query.rs:337-362`:

```rust
for token in tokenize_without_stopwords(line_trimmed) {
    let is_stopword = stopwords_set.contains(token.as_str());
    let tid_opt = index.dictionary.get(&token);

    if !is_stopword {
        if let Some(tid) = tid_opt {
            // Token found - add to query.tokens
            known_pos += 1;
            started = true;
            tokens.push(tid);
            // ...
        } else if !started {
            // Unknown token before any known token - tracked separately
            *unknowns_by_pos.entry(None).or_insert(0) += 1;
        } else {
            // Unknown token after known tokens - tracked separately
            *unknowns_by_pos.entry(Some(known_pos)).or_insert(0) += 1;
        }
    }
}
```

**Critical observation**: Unknown tokens are NOT added to `query.tokens`. They're only tracked for coverage calculations.

### Finding 6: Low Matchables Calculation

From `query.rs:429-434`:

```rust
let low_matchables: HashSet<usize> = tokens
    .iter()
    .enumerate()
    .filter(|(_pos, tid)| (**tid as usize) >= len_legalese)
    .map(|(pos, _tid)| pos)
    .collect();
```

Low matchables include positions where the token ID is >= len_legalese (non-legalese tokens).

### Finding 7: Aho-Corasick Matchables Check (THE FAILURE POINT)

From `aho_match.rs:103-107`:

```rust
let is_entirely_matchable = (qstart..qend).all(|pos| matchables.contains(&pos));

if !is_entirely_matchable {
    continue;
}
```

If the token position isn't in `matchables`, the match is discarded.

---

## Updated Root Cause Hypothesis

### The Problem Chain

1. Rule `lgpl_bare_single_word.RULE` has token `lgpl` which gets assigned ID N (where N >= len_legalese)
2. The rule IS added to the automaton (verified)
3. Query text "lgpl" tokenizes to `["lgpl"]`
4. Dictionary lookup: `index.dictionary.get("lgpl")` should return Some(N)
5. Token N should be added to `query.tokens` at position 0
6. Position 0 should be in `low_matchables` (since N >= len_legalese)
7. Aho-Corasick should find the match
8. Match should pass the `is_entirely_matchable` check

**The failure must be in one of these steps.**

### Hypothesis A: Case Sensitivity in Dictionary

The rule text "LGPL" tokenizes to lowercase "lgpl" (tokenize converts to lowercase). Query "lgpl" should find it.

**Need to verify**: Dictionary keys are lowercase.

### Hypothesis B: Token Not Found During Query Tokenization

If `index.dictionary.get("lgpl")` returns `None` for some reason, the token won't be added to `query.tokens`.

**Diagnostic needed**: Check dictionary contents for tokens from small reference rules.

### Hypothesis C: Match Found But Not in Matchables

If the position isn't correctly added to `low_matchables`, the match is discarded.

**Diagnostic needed**: Print `low_matchables` for query "lgpl".

### Hypothesis D: QueryRun Matchables Mismatch

The `matchables` used in `aho_match.rs` comes from `query_run.matchables(true)`. Need to verify this returns the correct positions.

From `query.rs:87`:

```rust
pub fn matchables(&self, include_low: bool) -> HashSet<usize> {
    if include_low {
        self.high_matchables.union(&self.low_matchables).cloned().collect()
    } else {
        self.high_matchables.clone()
    }
}
```

---

## Current Test Failures Analysis (lic4)

From the golden test run, the following tests return `[]` when they should detect licenses:

| Test File | Expected | Rust | Issue Type |
|-----------|----------|------|------------|
| `isc_only.txt` | `isc` | `[]` | Short reference detection |
| `lgpl_21.txt` | `lgpl-2.0-plus` | `[]` | Short reference detection |
| `warranty-disclaimer_1.txt` | `warranty-disclaimer` | `[]` | Short reference detection |
| `gpl-2.0-plus_and_lgpl-2.1-plus_and_mpl-1.0.html` | `mpl-1.0 OR gpl-2.0-plus OR lgpl-2.1-plus` | `[]` | Complex expression |

Additional related issues (not `[]` but incorrect):

- `proprietary_ibm.txt`: Expected `proprietary-license`, got `unknown-license-reference`
- Multiple tests with missing/extra detections due to merge or filter issues

---

## Implementation Plan

### Phase 1: Diagnostic Tests (Day 1)

#### Task 1.1: Create Diagnostic Test File

Create `src/license_detection/debug_short_ref_test.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::license_detection::LicenseDetectionEngine;
    use std::path::PathBuf;

    fn get_engine() -> Option<LicenseDetectionEngine> {
        let rules_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
        LicenseDetectionEngine::new(&rules_path).ok()
    }

    #[test]
    fn debug_lgpl_bare_word_detection() {
        let Some(engine) = get_engine() else { return; };
        
        // Test the exact text from lgpl_bare_single_word.RULE
        let text = "LGPL";
        let detections = engine.detect(text).unwrap();
        eprintln!("DEBUG: 'LGPL' detections = {:?}", detections.len());
        for d in &detections {
            eprintln!("  - {}", d.license_expression);
        }
        
        // Test lowercase
        let text2 = "lgpl";
        let detections2 = engine.detect(text2).unwrap();
        eprintln!("DEBUG: 'lgpl' detections = {:?}", detections2.len());
        for d in &detections2 {
            eprintln!("  - {}", d.license_expression);
        }
    }
    
    #[test]
    fn debug_token_dictionary() {
        let Some(engine) = get_engine() else { return; };
        let index = engine.index();
        
        // Check if 'lgpl' token exists in dictionary
        let lgpl_tid = index.dictionary.get("lgpl");
        eprintln!("DEBUG: 'lgpl' token ID = {:?}", lgpl_tid);
        eprintln!("DEBUG: len_legalese = {}", index.len_legalese);
        
        // Check if 'isc' token exists in dictionary
        let isc_tid = index.dictionary.get("isc");
        eprintln!("DEBUG: 'isc' token ID = {:?}", isc_tid);
        
        // Find the lgpl_bare_single_word rule
        for (rid, rule) in index.rules_by_rid.iter().enumerate() {
            if rule.identifier == "lgpl_bare_single_word.RULE" {
                eprintln!("DEBUG: Found lgpl_bare_single_word.RULE at rid={}", rid);
                eprintln!("  license_expression: {}", rule.license_expression);
                eprintln!("  is_license_reference: {}", rule.is_license_reference);
                eprintln!("  is_small: {}", rule.is_small);
                eprintln!("  tokens: {:?}", rule.tokens);
                eprintln!("  text: {:?}", rule.text);
                
                // Check if this rule is in the automaton
                let pattern_bytes = rule.tokens.iter()
                    .flat_map(|t| t.to_le_bytes())
                    .collect::<Vec<u8>>();
                let matches: Vec<_> = index.rules_automaton.find_iter(&pattern_bytes).collect();
                eprintln!("  automaton matches: {:?}", matches.len());
            }
        }
    }
    
    #[test]
    fn debug_query_tokenization() {
        let Some(engine) = get_engine() else { return; };
        
        use crate::license_detection::query::Query;
        let query = Query::new("lgpl", engine.index()).unwrap();
        
        eprintln!("DEBUG: Query for 'lgpl':");
        eprintln!("  tokens: {:?}", query.tokens);
        eprintln!("  high_matchables: {:?}", query.high_matchables);
        eprintln!("  low_matchables: {:?}", query.low_matchables);
        
        let run = query.whole_query_run();
        eprintln!("  run.matchables(true): {:?}", run.matchables(true));
    }
}
```

#### Task 1.2: Run Diagnostic Tests

```bash
cargo test debug_lgpl_bare_word_detection -- --nocapture 2>&1 | tee debug_output.txt
cargo test debug_token_dictionary -- --nocapture 2>&1 | tee debug_output2.txt
cargo test debug_query_tokenization -- --nocapture 2>&1 | tee debug_output3.txt
```

### Phase 2: Root Cause Verification (Day 1-2)

Based on diagnostic output, verify:

1. **Token dictionary lookup**: Does `index.dictionary.get("lgpl")` return Some?
2. **Query tokenization**: Is the token correctly added to `query.tokens`?
3. **Low matchables**: Is position 0 in `low_matchables`?
4. **Aho-Corasick match**: Does the automaton actually match?

### Phase 3: Implement Fix (Day 2-3)

Based on root cause, one of:

#### Fix A: Dictionary Token Case Handling

If tokens are not being found due to case issues:

- File: `src/license_detection/index/dictionary.rs`
- Ensure dictionary lookups are case-insensitive or normalize to lowercase

#### Fix B: Query Tokenization for Unknown Tokens

If unknown tokens need different handling for Aho-Corasick matching:

- File: `src/license_detection/query.rs:337-362`
- This would be a design change - need to understand Python behavior

#### Fix C: Low Matchables Population

If low_matchables calculation is wrong:

- File: `src/license_detection/query.rs:429-434`
- Verify that positions with non-legalese dictionary tokens are included

### Phase 4: Verification (Day 3-4)

#### Task 4.1: Run Specific Golden Tests

```bash
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic4 -- --nocapture
```

#### Task 4.2: Verify All Target Tests Pass

| Test File | Expected Result |
|-----------|-----------------|
| `lic4/isc_only.txt` | `isc` |
| `lic4/lgpl_21.txt` | `lgpl-2.0-plus` |
| `lic4/warranty-disclaimer_1.txt` | `warranty-disclaimer` |

---

## Key Code Files to Modify

| File | Lines | Purpose |
|------|-------|---------|
| `src/license_detection/index/builder.rs` | 227-290 | Rule tokenization, dictionary population |
| `src/license_detection/index/dictionary.rs` | 87-96 | Token ID assignment |
| `src/license_detection/query.rs` | 337-362 | Query tokenization, unknown token handling |
| `src/license_detection/query.rs` | 429-434 | Low matchables calculation |
| `src/license_detection/aho_match.rs` | 76-107 | Aho-Corasick matching, matchables check |

---

## Test Cases for Verification

### Primary Test Cases (Must Pass)

1. `lic4/isc_only.txt` - Expected: `isc`
2. `lic4/lgpl_21.txt` - Expected: `lgpl-2.0-plus`
3. `lic4/warranty-disclaimer_1.txt` - Expected: `warranty-disclaimer`

### Additional Test Cases

From lic4 golden test failures:

- `lic4/gpl-2.0-plus_and_lgpl-2.1-plus_and_mpl-1.0.html` - Expected: `mpl-1.0 OR gpl-2.0-plus OR lgpl-2.1-plus`

---

## Success Criteria

1. All primary test cases detect expected licenses
2. No regression in existing passing tests
3. lic4 golden test pass rate improves from 299/350 to 302+/350
4. Code matches Python behavior for short license references

---

## Related Documentation

- [PLAN-023-failure-analysis-summary.md](PLAN-023-failure-analysis-summary.md) - Pattern C description
- [PLAN-024-fix-match-merging.md](PLAN-024-fix-match-merging.md) - Distance-based merging (completed)
- [PLAN-028-fix-utf8-binary-handling.md](PLAN-028-fix-utf8-binary-handling.md) - Binary handling (completed)
- [ARCHITECTURE.md](../ARCHITECTURE.md) - Matching pipeline overview
- Python reference: `reference/scancode-toolkit/src/licensedcode/match.py:1706-1737`
- Python tokenization: `reference/scancode-toolkit/src/licensedcode/tokenize.py`
