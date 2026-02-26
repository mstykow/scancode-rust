# PLAN-083: Investigation Steps for Aho-Corasick Pattern Mismatch

## Problem Summary

**File**: `testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_gpl-2.0-plus_and_gpl-3.0-plus_and_lgpl-2.1-plus_and_other.txt`

**Symptom**: Rust's Aho-Corasick matcher does NOT find `lgpl-2.1-plus_24.RULE` at lines 13-17, while Python does.

**Impact**: This causes an extra `lgpl-2.1-plus` detection because:
- Rust finds `lgpl-2.1-plus_108.RULE` (3 tokens) at lines 13-13
- The containment filter cannot filter this tiny match because the containing match was never found

## Key Observations

| Observation | Python | Rust |
|-------------|--------|------|
| `lgpl-2.1-plus_24.RULE` at lines 13-17 | FOUND | NOT FOUND |
| `lgpl-2.1-plus_24.RULE` at lines 22-26 | FOUND | FOUND |
| `lgpl-2.1-plus_108.RULE` at lines 13-13 | Filtered out (contained) | NOT filtered (no container) |

**Critical Insight**: The pattern IS in the automaton because Rust finds it at lines 22-26. The issue is specific to the match at lines 13-17.

## Possible Causes

### 1. Tokenization Difference (MOST LIKELY)
The token sequence at lines 13-17 may differ from the expected pattern due to:
- Different whitespace handling
- Different line ending handling
- Different tokenization of specific tokens (e.g., "LGPL-2.1+")

### 2. Aho-Corasick Match Kind Configuration
Rust uses `MatchKind::Standard` while Python may use a different match kind that affects overlapping matches.

### 3. Pattern Encoding Issue
The pattern encoding (u16 little-endian bytes) may have alignment issues at certain positions.

### 4. Matchables Check Failure
The `is_entirely_matchable` check in `aho_match.rs:103` may be rejecting the match incorrectly.

## Investigation Plan

### Phase 1: Verify Pattern is in Automaton

**Step 1.1**: Add debug test to verify `lgpl-2.1-plus_24.RULE` is in the automaton

```rust
#[test]
fn test_plan_083_verify_pattern_in_automaton() {
    let Some(engine) = ensure_engine() else { return; };
    
    // Find the rule
    let target_rule = "lgpl-2.1-plus_24.RULE";
    let rid = engine.index.rules_by_rid.iter()
        .position(|r| r.identifier == target_rule)
        .expect("Rule should exist");
    
    // Get the pattern bytes
    let rule_tokens = &engine.index.tids_by_rid[rid];
    let pattern_bytes: Vec<u8> = rule_tokens.iter()
        .flat_map(|t| t.to_le_bytes())
        .collect();
    
    eprintln!("Rule: {}", target_rule);
    eprintln!("Token count: {}", rule_tokens.len());
    eprintln!("First 10 tokens: {:?}", &rule_tokens[..10.min(rule_tokens.len())]);
    eprintln!("Pattern bytes length: {}", pattern_bytes.len());
    
    // Verify the pattern can be matched by the automaton
    let matches: Vec<_> = engine.index.rules_automaton
        .find_iter(&pattern_bytes)
        .collect();
    
    assert_eq!(matches.len(), 1, "Pattern should match itself");
    eprintln!("Pattern matches itself at byte positions: {}-{}", 
              matches[0].start(), matches[0].end());
    
    // Find which pattern_id this is
    let pattern_id = matches[0].pattern();
    let mapped_rid = engine.index.pattern_id_to_rid[pattern_id.as_usize()];
    eprintln!("Pattern ID: {}, Mapped RID: {}", pattern_id.as_usize(), mapped_rid);
    assert_eq!(mapped_rid, rid, "Pattern ID should map to correct RID");
}
```

### Phase 2: Compare Tokenization at Lines 13-17 vs Lines 22-26

**Step 2.1**: Add debug test to compare query tokens at both locations

```rust
#[test]
fn test_plan_083_compare_tokenization() {
    let Some(engine) = ensure_engine() else { return; };
    
    let text = std::fs::read_to_string("testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_gpl-2.0-plus_and_gpl-3.0-plus_and_lgpl-2.1-plus_and_other.txt")
        .expect("Test file should exist");
    
    // Create query
    let query = crate::license_detection::query::Query::new(&text, &engine.index);
    
    // Find the rule tokens for lgpl-2.1-plus_24.RULE
    let target_rule = "lgpl-2.1-plus_24.RULE";
    let rid = engine.index.rules_by_rid.iter()
        .position(|r| r.identifier == target_rule)
        .expect("Rule should exist");
    let rule_tokens = &engine.index.tids_by_rid[rid];
    
    eprintln!("\n=== Rule Tokens (first 10) ===");
    for (i, &tid) in rule_tokens.iter().take(10).enumerate() {
        let token_str = engine.index.dictionary.get_by_id(tid).unwrap_or("?");
        eprintln!("  [{}] {} ({})", i, tid, token_str);
    }
    
    // Lines 13-17 should start around token position 72 (based on qspan 72-119)
    // Lines 22-26 should start around token position 135 (based on qspan 135-182)
    
    eprintln!("\n=== Query Tokens at Lines 13-17 (positions 70-80) ===");
    for i in 70..80.min(query.tokens.len()) {
        let line = query.line_by_pos.get(i).copied().unwrap_or(0);
        let tid = query.tokens[i];
        let token_str = engine.index.dictionary.get_by_id(tid).unwrap_or("?");
        eprintln!("  [{}] {} ({}) line={}", i, tid, token_str, line);
    }
    
    eprintln!("\n=== Query Tokens at Lines 22-26 (positions 133-143) ===");
    for i in 133..143.min(query.tokens.len()) {
        let line = query.line_by_pos.get(i).copied().unwrap_or(0);
        let tid = query.tokens[i];
        let token_str = engine.index.dictionary.get_by_id(tid).unwrap_or("?");
        eprintln!("  [{}] {} ({}) line={}", i, tid, token_str, line);
    }
    
    // Try to match the rule pattern at both locations
    let pattern_bytes: Vec<u8> = rule_tokens.iter()
        .flat_map(|t| t.to_le_bytes())
        .collect();
    
    eprintln!("\n=== Trying to match pattern at position 72 ===");
    if query.tokens.len() >= 72 + rule_tokens.len() {
        let query_slice: Vec<u8> = query.tokens[72..72+rule_tokens.len()]
            .iter().flat_map(|t| t.to_le_bytes()).collect();
        let matches: Vec<_> = engine.index.rules_automaton.find_iter(&query_slice).collect();
        eprintln!("  Matches at position 72: {}", matches.len());
        for m in &matches {
            eprintln!("    Pattern {} at bytes {}-{}", m.pattern().as_usize(), m.start(), m.end());
        }
    }
    
    eprintln!("\n=== Trying to match pattern at position 135 ===");
    if query.tokens.len() >= 135 + rule_tokens.len() {
        let query_slice: Vec<u8> = query.tokens[135..135+rule_tokens.len()]
            .iter().flat_map(|t| t.to_le_bytes()).collect();
        let matches: Vec<_> = engine.index.rules_automaton.find_iter(&query_slice).collect();
        eprintln!("  Matches at position 135: {}", matches.len());
        for m in &matches {
            eprintln!("    Pattern {} at bytes {}-{}", m.pattern().as_usize(), m.start(), m.end());
        }
    }
}
```

### Phase 3: Debug Aho-Corasick Matching

**Step 3.1**: Add debug test to trace all Aho-Corasick matches in the query

```rust
#[test]
fn test_plan_083_debug_aho_matches() {
    let Some(engine) = ensure_engine() else { return; };
    
    let text = std::fs::read_to_string("testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_gpl-2.0-plus_and_gpl-3.0-plus_and_lgpl-2.1-plus_and_other.txt")
        .expect("Test file should exist");
    
    let query = crate::license_detection::query::Query::new(&text, &engine.index);
    let query_run = query.whole_query_run();
    
    // Encode query tokens as bytes
    let encoded_query: Vec<u8> = query.tokens.iter()
        .flat_map(|t| t.to_le_bytes())
        .collect();
    
    eprintln!("\n=== All Aho-Corasick matches in range 140-240 bytes (token 70-120) ===");
    for m in engine.index.rules_automaton.find_overlapping_iter(&encoded_query) {
        let byte_start = m.start();
        let byte_end = m.end();
        
        if byte_start >= 140 && byte_start <= 240 {
            let tok_start = byte_start / 2;
            let tok_end = byte_end / 2;
            let rid = engine.index.pattern_id_to_rid[m.pattern().as_usize()];
            let rule = &engine.index.rules_by_rid[rid];
            let start_line = query.line_by_pos.get(tok_start).copied().unwrap_or(0);
            let end_line = query.line_by_pos.get(tok_end.saturating_sub(1)).copied().unwrap_or(0);
            
            eprintln!("  bytes={}-{} tokens={}-{} lines={}-{} rule={} tokens_len={}",
                byte_start, byte_end, tok_start, tok_end, start_line, end_line,
                rule.identifier, engine.index.tids_by_rid[rid].len());
        }
    }
    
    // Specifically look for lgpl-2.1-plus_24.RULE matches
    let target_rule = "lgpl-2.1-plus_24.RULE";
    let target_rid = engine.index.rules_by_rid.iter()
        .position(|r| r.identifier == target_rule)
        .expect("Rule should exist");
    
    eprintln!("\n=== All matches for {} (RID {}) ===", target_rule, target_rid);
    for m in engine.index.rules_automaton.find_overlapping_iter(&encoded_query) {
        let rid = engine.index.pattern_id_to_rid[m.pattern().as_usize()];
        if rid == target_rid {
            let tok_start = m.start() / 2;
            let tok_end = m.end() / 2;
            let start_line = query.line_by_pos.get(tok_start).copied().unwrap_or(0);
            let end_line = query.line_by_pos.get(tok_end.saturating_sub(1)).copied().unwrap_or(0);
            
            eprintln!("  tokens={}-{} lines={}-{}", tok_start, tok_end, start_line, end_line);
        }
    }
}
```

### Phase 4: Compare with Python Implementation

**Step 4.1**: Run Python reference and capture the exact match data

```bash
cd reference/scancode-toolkit
python -c "
from licensedcode.cache import get_index
idx = get_index()

text = open('../../testdata/license-golden/datadriven/lic4/gpl-2.0-plus_and_gpl-2.0-plus_and_gpl-3.0-plus_and_lgpl-2.1-plus_and_other.txt').read()
results = idx.match(text, include_matches=True)

for m in results.matches:
    if 'lgpl-2.1-plus_24' in m.rule_identifier:
        print(f'rule={m.rule_identifier} qspan={m.qspan} lines={m.start_line}-{m.end_line}')
        print(f'  matched tokens: {list(m.matched_tokens())[:10]}...')
"
```

**Step 4.2**: Compare automaton building between Python and Rust

Look at `reference/scancode-toolkit/src/licensedcode/match_aho.py` to understand how Python builds the automaton.

### Phase 5: Identify and Fix the Root Cause

Based on findings from phases 1-4, implement the fix.

**Likely Fix Areas**:

1. **If tokenization differs**: Fix tokenization in `src/license_detection/tokenize.rs`
2. **If automaton config differs**: Adjust `MatchKind` or other Aho-Corasick settings in `builder.rs:438-441`
3. **If pattern encoding issue**: Fix encoding in `tokens_to_bytes` function
4. **If matchables check issue**: Fix the matchable positions calculation

## Implementation Steps

### Step 1: Add Investigation Tests

Add the debug tests from Phases 1-3 to `src/license_detection/extra_detection_investigation_test.rs`.

### Step 2: Run Tests and Analyze Output

```bash
cargo test --lib test_plan_083 -- --nocapture 2>&1 | tee plan_083_debug.log
```

### Step 3: Compare Token-by-Token

Based on test output, compare:
- Rule tokens for `lgpl-2.1-plus_24.RULE`
- Query tokens at lines 13-17 (positions 72+)
- Query tokens at lines 22-26 (positions 135+)

Look for any difference that would prevent matching at position 72.

### Step 4: Implement Fix

Based on root cause, implement the fix in the appropriate file.

### Step 5: Verify Fix

```bash
cargo test --lib test_plan_083_gpl_lgpl_complex -- --nocapture
```

Expected output: Rust should now find `lgpl-2.1-plus_24.RULE` at lines 13-17 and the total match count should be 8 (matching Python).

## Success Criteria

1. `lgpl-2.1-plus_24.RULE` is found at lines 13-17
2. `lgpl-2.1-plus_108.RULE` at lines 13-13 is correctly filtered as contained
3. Total match count is 8 (matching Python)
4. No other tests regress

## Files to Modify

- `src/license_detection/aho_match.rs` - If match logic is wrong
- `src/license_detection/index/builder.rs` - If automaton building is wrong
- `src/license_detection/tokenize.rs` - If tokenization differs
- `src/license_detection/query.rs` - If query token handling differs
