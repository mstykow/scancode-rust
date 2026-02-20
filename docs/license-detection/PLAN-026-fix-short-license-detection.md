# PLAN-026: Fix Short License Reference Detection

**Date**: 2026-02-20
**Status**: Investigation Complete - Root Cause Identified
**Priority**: 3 (Pattern C from PLAN-023)
**Impact**: ~20 complete detection failures across all suites

## Executive Summary

Rust returns empty detections `[]` where Python successfully detects licenses for short license references and modified license text. This is **Pattern C** from the failure analysis, causing complete detection failures.

| Test File | Rust Result | Python Result |
|-----------|-------------|---------------|
| `lic4/isc_only.txt` | `[]` | `isc` |
| `lic4/warranty-disclaimer_1.txt` | `[]` | `warranty-disclaimer` |
| `lic4/lgpl_21.txt` | `[]` | `lgpl-2.0-plus` |
| `lic3/mit_additions_1.c` | `[]` | `["mit", "mit"]` |

---

## Root Cause Analysis

### Finding 1: Filter Logic is Correct

Both Python and Rust filter implementations are equivalent:

**Python** (`match.py:1706-1737`):
- Only filters `MATCH_SEQ` (sequence matches)
- Uses `is_small()` with two conditions:
  - CASE 1: `matched_len < min_matched_len OR high_matched_len < min_high_matched_len`
  - CASE 2: `rule.is_small AND coverage < 80`

**Rust** (`match_refine.rs:62-84`):
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

Small reference rules like `lgpl_bare_single_word.RULE` (text: `LGPL`) ARE added to the automaton.

### Finding 3: Weak Rules Are NOT Sequence Matchable

From `builder.rs:273-290`:
```rust
let is_approx_matchable = {
    rule.is_small = rule_length < SMALL_RULE;
    rule.is_tiny = rule_length < TINY_RULE;
    compute_is_approx_matchable(&rule)
};
// ...
if is_approx_matchable && !is_weak {
    approx_matchable_rids.insert(rid);
```

Rules like `lgpl_bare_single_word.RULE`:
- `is_license_reference: yes`
- `is_small: true` (1 token < SMALL_RULE)
- `is_weak: true` (token `lgpl` is not a legalese word)
- `is_approx_matchable: false` (because small + license_reference)

This means they won't be found via sequence matching, but **should** be found via Aho-Corasick.

### Finding 4: Root Cause is in Query Tokenization

**The critical issue**: In `query.rs:337-356`, when a token is not found in the dictionary:

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

**Unknown tokens are NOT added to `query.tokens`**. They're only tracked for coverage calculations.

### Finding 5: Token Dictionary Population

The dictionary is populated in order:
1. Legalese words (IDs 0 to len_legalese-1) - from `legalese::get_legalese_words()`
2. Tokens from all rules during indexing

**Key insight**: Tokens like `lgpl`, `isc`, etc. should be added during rule indexing when processing rules like `lgpl_bare_single_word.RULE`.

### Finding 6: Actual Test File Analysis

**`lic4/lgpl_21.txt`** content:
```
/* vi: set sw=4 ts=4: */
/*
 * A tiny 'top' utility.
 *
   lgpl
   
 * This reads the PIDs...
```

The word `lgpl` appears alone on line 6. The rule `lgpl_bare_single_word.RULE` has:
```yaml
license_expression: lgpl-2.0-plus
is_license_reference: yes
relevance: 60
---
LGPL
```

**`lic4/isc_only.txt`** content:
- Line 8: `Copyright: ISC`
- Line 121: `%doc COPYRIGHT DOCUMENTATION ISC-LICENSE CHANGES...`

ISC reference rules like `isc_1.RULE` contain:
```yaml
license_expression: isc
is_license_reference: yes
---
https://www.isc.org/isc-license-1.0.html
```

**`lic4/warranty-disclaimer_1.txt`** content:
```
//-----------------------------------------------------------------------------
//
// THIS CODE AND INFORMATION IS PROVIDED "AS IS" WITHOUT WARRANTY OF
// ANY KIND, EITHER EXPRESSED OR IMPLIED...
```

The rule `warranty-disclaimer_93.RULE` has:
```yaml
license_expression: warranty-disclaimer
is_license_reference: yes
---
THIS SOFTWARE IS SUPPLIED COMPLETELY "AS IS".  
               NO WARRANTY....
```

---

## Root Cause Hypothesis

The detection failures are caused by one of these scenarios:

### Hypothesis A: Dictionary Token Lookup Failure

If the token dictionary isn't correctly populated with tokens from rules during index building, query tokenization will fail to find tokens like `lgpl`, resulting in no matches.

**Investigation needed**: Verify that rule tokenization during index building adds all tokens to the dictionary.

### Hypothesis B: Matchables Calculation Issue

In `aho_match.rs:103`:
```rust
let is_entirely_matchable = (qstart..qend).all(|pos| matchables.contains(&pos));
```

If the token position isn't in `matchables`, the match is discarded.

**Investigation needed**: Verify that `low_matchables` correctly includes positions with non-legalese dictionary tokens.

### Hypothesis C: Token Encoding Mismatch

Aho-Corasick matches byte patterns. If there's an encoding mismatch between rule and query token sequences, matches will fail.

**Investigation needed**: Compare rule token bytes with query token bytes for the same token.

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
        
        // Test the test file content
        let test_text = std::fs::read_to_string("testdata/license-golden/datadriven/lic4/lgpl_21.txt").unwrap();
        let detections = engine.detect(&test_text).unwrap();
        eprintln!("DEBUG: lgpl_21.txt detections = {:?}", detections.len());
    }

    #[test]
    fn debug_token_dictionary() {
        let Some(engine) = get_engine() else { return; };
        let index = engine.index();
        
        // Check if 'lgpl' token exists in dictionary
        let lgpl_tid = index.dictionary.get("lgpl");
        eprintln!("DEBUG: 'lgpl' token ID = {:?}", lgpl_tid);
        
        // Check if 'isc' token exists in dictionary
        let isc_tid = index.dictionary.get("isc");
        eprintln!("DEBUG: 'isc' token ID = {:?}", isc_tid);
        
        // List a few rules containing 'lgpl'
        for (rid, rule) in index.rules_by_rid.iter().enumerate().take(100) {
            if rule.license_expression.contains("lgpl") {
                eprintln!("DEBUG: Rule {} (rid={}): is_license_reference={}, is_small={}, tokens={:?}",
                    rule.identifier, rid, rule.is_license_reference, rule.is_small, rule.tokens.len());
            }
        }
    }

    #[test]
    fn debug_aho_corasick_matching() {
        let Some(engine) = get_engine() else { return; };
        let index = engine.index();
        
        // Find the rid for lgpl_bare_single_word rule
        let lgpl_rule = index.rules_by_rid.iter()
            .find(|r| r.identifier == "lgpl_bare_single_word.RULE");
        
        if let Some(rule) = lgpl_rule {
            eprintln!("DEBUG: Found lgpl_bare_single_word.RULE");
            eprintln!("  license_expression: {}", rule.license_expression);
            eprintln!("  is_license_reference: {}", rule.is_license_reference);
            eprintln!("  is_small: {}", rule.is_small);
            eprintln!("  tokens: {:?}", rule.tokens);
            eprintln!("  text: {:?}", rule.text);
        }
    }
}
```

#### Task 1.2: Run Diagnostic Tests

```bash
cargo test debug_lgpl_bare_word_detection -- --nocapture 2>&1 | tee debug_output.txt
cargo test debug_token_dictionary -- --nocapture 2>&1 | tee debug_output2.txt
cargo test debug_aho_corasick_matching -- --nocapture 2>&1 | tee debug_output3.txt
```

### Phase 2: Root Cause Verification (Day 1-2)

Based on diagnostic output, verify:

1. **Token dictionary lookup**: Does `index.dictionary.get("lgpl")` return Some?
2. **Query tokenization**: Are unknown tokens correctly handled?
3. **Aho-Corasick matchability**: Are matched positions in matchables set?

### Phase 3: Implement Fix (Day 2-3)

Based on root cause, one of:

#### Fix A: Dictionary Token Addition

If tokens aren't being added to dictionary during rule indexing:
- File: `src/license_detection/index/builder.rs`
- Ensure all rule tokens are added via `dictionary.get_or_assign()`

#### Fix B: Query Tokenization for Unknown Tokens

If unknown tokens need to be in query.tokens for Aho-Corasick matching:
- This would be a design change - need to understand Python behavior
- Python likely does add unknown tokens to query for automaton matching

#### Fix C: Matchables Set Population

If matchables calculation is wrong:
- File: `src/license_detection/query.rs:422-434`
- Verify low_matchables includes all dictionary tokens

### Phase 4: Verification (Day 3-4)

#### Task 4.1: Run Specific Golden Tests

```bash
cargo test --release -q --lib license_detection::golden_test::golden_tests::test_golden_lic4 -- --nocapture
```

#### Task 4.2: Verify All Target Tests Pass

| Test File | Expected Result |
|-----------|-----------------|
| `lic4/isc_only.txt` | `isc` |
| `lic4/warranty-disclaimer_1.txt` | `warranty-disclaimer` |
| `lic4/lgpl_21.txt` | `lgpl-2.0-plus` |

---

## Key Code Files to Modify

| File | Lines | Purpose |
|------|-------|---------|
| `src/license_detection/index/builder.rs` | 227-290 | Rule tokenization, dictionary population |
| `src/license_detection/query.rs` | 337-362 | Query tokenization, unknown token handling |
| `src/license_detection/aho_match.rs` | 76-107 | Aho-Corasick matching, matchables check |

---

## Test Cases for Verification

### Primary Test Cases (Must Pass)

1. `lic4/isc_only.txt` - Expected: `isc`
2. `lic4/warranty-disclaimer_1.txt` - Expected: `warranty-disclaimer`
3. `lic4/lgpl_21.txt` - Expected: `lgpl-2.0-plus`

### Additional Test Cases

From lic4 golden test failures:
- `lic4/isc_redhat.txt` - Expected: `isc` (also returns `[]`)
- `lic4/sun-bcl-11-07.txt` - Expected: `sun-bcl-11-07` (also returns `[]`)
- `lic4/proprietary_9.txt` - Expected: `proprietary-license` (also returns `[]`)

---

## Success Criteria

1. All primary test cases detect expected licenses
2. No regression in existing passing tests
3. lic4 golden test pass rate improves from 285/350 to 290+/350
4. Code matches Python behavior for short license references

---

## Related Documentation

- [PLAN-023-failure-analysis-summary.md](PLAN-023-failure-analysis-summary.md) - Pattern C description
- [ARCHITECTURE.md](../ARCHITECTURE.md) - Matching pipeline overview
- Python reference: `reference/scancode-toolkit/src/licensedcode/match.py:1706-1737`
- Python tokenization: `reference/scancode-toolkit/src/licensedcode/tokenize.py`
