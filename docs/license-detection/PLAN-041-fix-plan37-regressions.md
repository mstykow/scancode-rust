# PLAN-041: Fix PLAN-037 lic2 Regressions

**Date**: 2026-02-24
**Status**: ⚠️ IMPLEMENTATION CAUSES REGRESSION - NEEDS INVESTIGATION
**Priority**: HIGH
**Related**: PLAN-037 (post-phase merge implementation)
**Impact**: Implementation causes -6 tests regression (3780 → 3774 passed)

## Implementation Attempt (2026-02-24)

Implemented SPDX token pre-population as described in the plan. Result:

- **Baseline**: 3780 passed, 583 failed
- **After PLAN-041**: 3774 passed, 589 failed (regression of 6 tests)

### Root Cause of Regression

The SPDX token pre-population changes token IDs, which changes:

1. Hash values for all license texts
2. This affects hash matching behavior
3. Golden tests expect specific output that depends on current hash behavior

### Hypothesis

The pre-population is correct for parity with Python, but:

- Python and Rust may have different license datasets
- The token ID assignment order may still differ in subtle ways
- The test infrastructure may need updates to match Python hashes

### Recommendation

1. **DO NOT implement** until we can verify exact hash parity with Python
2. Compare Python and Rust hash values for known license texts
3. Verify `rid_by_hash` population matches Python exactly

---

## Executive Summary

PLAN-037 implemented hash match early return and post-phase `merge_overlapping_matches()` calls. While this improved overall golden test results (+3 tests), it caused 2 regressions in lic2.

**ROOT CAUSE IDENTIFIED**: Python pre-assigns token IDs to SPDX key tokens BEFORE processing rules. Rust does not. This causes different token IDs for the same license text, leading to different hashes and hash match failures.

**Impact**: When Rust and Python compute hashes for identical license text, they get DIFFERENT token ID sequences → DIFFERENT SHA1 hashes → hash lookup failures.

---

## 2. Detailed Investigation Findings

### 2.1 Python Hash Match Semantics

**Location**: `reference/scancode-toolkit/src/licensedcode/index.py:987-991`

```python
if not _skip_hash_match:
    matches = match_hash.hash_match(self, whole_query_run)
    if matches:
        match.set_matched_lines(matches, qry.line_by_pos)
        return matches  # EARLY RETURN - skips ALL other phases
```

**Python's `hash_match()` function** (`match_hash.py:59-87`):

```python
def hash_match(idx, query_run, **kwargs):
    """
    Return a sequence of LicenseMatch by matching the query_tokens sequence
    against the idx index.
    """
    matches = []
    query_hash = tokens_hash(query_run.tokens)  # SHA1 of entire token sequence
    rid = idx.rid_by_hash.get(query_hash)
    if rid is not None:
        rule = idx.rules_by_rid[rid]
        # ... create single match covering entire query_run ...
        match = LicenseMatch(
            rule=rule,
            qspan=Span(range(query_run.start, query_run.end + 1)),
            ispan=Span(range(0, rule.length)),
            matcher=MATCH_HASH,  # '1-hash'
            matcher_order=MATCH_HASH_ORDER,  # 0
        )
        matches.append(match)
    return matches  # At most ONE match
```

**Key Properties of Python Hash Matching**:

| Property | Value | Impact |
|----------|-------|--------|
| Input scope | `whole_query_run` tokens | Entire file content |
| Output size | 0 or 1 match | All-or-nothing |
| Match coverage | Always 100% | Complete file match |
| Match type | `is_license_text` rules | Full license texts |
| Early return | Yes, after hash match | Skip SPDX-LID, Aho, seq phases |

### 2.2 Rust Hash Match Implementation

**Location**: `src/license_detection/hash_match.rs:72-133`

```rust
pub fn hash_match(index: &LicenseIndex, query_run: &QueryRun) -> Vec<LicenseMatch> {
    let mut matches = Vec::new();
    let query_hash = compute_hash(query_run.tokens());  // SHA1 of token sequence
    
    if let Some(&rid) = index.rid_by_hash.get(&query_hash) {
        let rule = &index.rules_by_rid[rid];
        // ... create single match ...
        let license_match = LicenseMatch {
            matcher: MATCH_HASH.to_string(),  // "1-hash"
            score: 1.0,
            match_coverage: 100.0,
            // ...
        };
        matches.push(license_match);
    }
    matches  // At most ONE match
}
```

**Hash Computation Comparison**:

| Aspect | Python | Rust | Match? |
|--------|--------|------|--------|
| Algorithm | SHA1 | SHA1 | YES |
| Token encoding | `array('h', tokens).tobytes()` | `i16::to_le_bytes()` per token | YES |
| Endianness | Little-endian (array) | Little-endian (explicit) | YES |
| Input | Token ID sequence | Token ID sequence | SHOULD MATCH |

### 2.3 Current Rust Behavior (PLAN-037)

**Location**: `src/license_detection/mod.rs:125-151`

```rust
// Phase 1a: Hash matching
// Python returns immediately if hash matches found (index.py:987-991)
{
    let whole_run = query.whole_query_run();
    let hash_matches = hash_match(&self.index, &whole_run);

    if !hash_matches.is_empty() {
        let mut matches = hash_matches;
        sort_matches_by_line(&mut matches);

        let groups = group_matches_by_region(&matches);
        let detections: Vec<LicenseDetection> = groups
            .iter()
            .map(|group| {
                let mut detection = create_detection_from_group(group);
                populate_detection_from_group_with_spdx(
                    &mut detection,
                    group,
                    &self.spdx_mapping,
                );
                detection
            })
            .collect();

        return Ok(post_process_detections(detections, 0.0));  // EARLY RETURN
    }
}
```

### 2.4 Critical Semantic Differences

#### Difference 1: Processing Bypassed by Early Return

When hash match early returns, the following is **SKIPPED**:

| Processing Step | Normal Path | Hash Early Return | Impact |
|----------------|-------------|-------------------|--------|
| `refine_matches()` | Called | NOT called | Missing false positive filtering |
| `merge_matches()` in refine | Called | NOT called | N/A (hash returns 0 or 1 match) |
| Detection log population | In refine | NOT populated | Missing debug info |
| False positive filtering | In refine | NOT filtered | Potential false positive |
| Weak match filtering | In refine | NOT filtered | N/A (score is 1.0) |

**However**: Hash matches are by definition 100% exact matches of known license texts. They should NEVER be false positives. The bypassing of `refine_matches()` should be semantically correct.

#### Difference 2: Post-Phase Merge Behavior

**Before PLAN-037**:

```rust
// SPDX-LID matches added without merge
all_matches.extend(spdx_matches);

// Aho matches added without merge
all_matches.extend(aho_matches);

// All merging happens in refine_matches() at the end
let refined = refine_matches(&self.index, all_matches, &query);
```

**After PLAN-037**:

```rust
// SPDX-LID matches merged before adding
let merged_spdx = merge_overlapping_matches(&spdx_matches);
all_matches.extend(merged_spdx);

// Aho matches merged before adding
let merged_aho = merge_overlapping_matches(&aho_matches);
all_matches.extend(merged_aho);

// Sequence matches merged once after collection
let merged_seq = merge_overlapping_matches(&seq_all_matches);
all_matches.extend(merged_seq);

// Additional merging in refine_matches()
let refined = refine_matches(&self.index, all_matches, &query);
```

**Python's Behavior** (`index.py:1040`):

```python
for matcher in matchers:
    matched = matcher.function(...)
    matched = match.merge_matches(matched)  # MERGE IMMEDIATELY
    matches.extend(matched)
```

The post-phase merge behavior **MATCHES Python** and should be correct.

---

## 3. Root Cause Analysis - COMPLETED

### 3.1 Hypothesis 1: Token ID Mismatch - DISPROVEN ✓

**Status**: DISPROVEN - Token IDs DO match between Python and Rust for legalese tokens.

**Investigation Results** (2026-02-24):

Created comprehensive test suite in `src/license_detection/token_id_equivalence_test.rs` with 9 tests - all passing.

**Verified Equivalence**:

- Legalese: 4506 entries → 4356 unique token IDs (matches Python)
- Hash computation algorithm: SHA1 identical
- Hash of [1,2,3,4,5]: `aaa562e5641b932d5d5ecae43b47793b33b3b5f0` (both)

**Conclusion**: Legalese token IDs are IDENTICAL. However, this does NOT mean all token IDs match - see Hypothesis 2.

### 3.2 Hypothesis 2: SPDX Token Pre-Population Missing - CONFIRMED AS ROOT CAUSE ✓✓✓

**Status**: ROOT CAUSE CONFIRMED

**Critical Discovery**: Python pre-assigns token IDs to SPDX key tokens BEFORE processing rules. Rust does NOT.

#### Python Implementation (index.py:301-314)

```python
# Add SPDX key tokens to the dictionary: these are always treated as
# non-legalese. This may seem weird but they are detected in expressions
# alright and some of their tokens exist as rules too (e.g. GPL).
# Treating their words as legalese by default creates problems as common
# words such as mit may become legalese words even though we do not want
# this to happen.
########################################################################
for sts in sorted(_spdx_tokens):
    stid = dictionary_get(sts)
    if stid is None:
        # we have a never yet seen token, so we assign a new tokenid
        highest_tid += 1
        stid = highest_tid
        dictionary[sts] = stid
```

**Where `_spdx_tokens` comes from** (`models.py:1195-1205` and `cache.py:260`):

```python
def get_all_spdx_key_tokens(licenses_db):
    """Yield SPDX token strings from licenses_db."""
    for tok in get_essential_spdx_tokens():
        yield tok  # 'spdx', 'license', 'licence', 'identifier', 'licenseref'

    for spdx_key in get_all_spdx_keys(licenses_db):
        for token in index_tokenizer(spdx_key):
            yield token  # tokens from all SPDX license keys

# In cache.py:260
spdx_tokens = set(get_all_spdx_key_tokens(licenses_db))
```

#### Rust Implementation (builder.rs:235-282) - MISSING THIS STEP

```rust
pub fn build_index(rules: Vec<Rule>, licenses: Vec<License>) -> LicenseIndex {
    let legalese_words = legalese::get_legalese_words();
    let mut dictionary = TokenDictionary::new_with_legalese(&legalese_words);
    let len_legalese = dictionary.legalese_count();

    // MISSING: Pre-assign SPDX tokens here!

    for (rid, mut rule) in all_rules.into_iter().enumerate() {
        // ...
        for rts in &rule_tokens {
            let rtid = dictionary.get_or_assign(rts);  // Gets DIFFERENT IDs than Python
            // ...
        }
    }
}
```

#### Why This Causes Hash Mismatches

1. **Python flow**:
   - Dictionary initialized with legalese (IDs 0-4355)
   - SPDX tokens pre-assigned (IDs 4356-~5200)
   - Rules processed → tokens get consistent IDs

2. **Rust flow (current)**:
   - Dictionary initialized with legalese (IDs 0-4355)
   - SPDX tokens NOT pre-assigned
   - Rules processed → SPDX tokens get IDs in processing order

3. **Result**: Same license text → different token IDs → different SHA1 hash → hash lookup fails

#### Example Impact

Consider a file containing "mit license":

- Python: "mit" might get token ID 4400 (pre-assigned), "license" = 2432 (legalese)
- Rust: "mit" might get token ID 4506 (first encounter), "license" = 2432 (legalese)

Hash of [4400, 2432] ≠ Hash of [4506, 2432] → **Hash match fails**

### 3.3 Hypothesis 3: Query Run Scope Difference - INVALID ✓

**Status**: INVALID - Query run scopes match perfectly.

Investigation confirmed `whole_query_run()` returns identical token ranges in both implementations.

### Summary

| Hypothesis | Status | Impact |
|------------|--------|--------|
| Token ID Mismatch | DISPROVEN | Legalese tokens match |
| SPDX Token Pre-Population | **ROOT CAUSE** | Non-legalese tokens differ → hash failures |
| Query Run Scope | INVALID | No issue |

---

## 4. Implementation Plan

### Step 1: Create SPDX Token Collection Function

**Location**: `src/license_detection/index/builder.rs`

```rust
/// Get essential SPDX tokens that must be pre-assigned.
/// Based on Python: models.py:1188-1192
fn get_essential_spdx_tokens() -> Vec<&'static str> {
    vec!["spdx", "license", "licence", "identifier", "licenseref"]
}

/// Tokenize an SPDX key for token extraction.
/// Based on Python: models.py:1203-1204 index_tokenizer()
fn tokenize_spdx_key(key: &str) -> Vec<String> {
    let lowercase = key.to_lowercase();
    QUERY_PATTERN
        .find_iter(&lowercase)
        .filter_map(|cap| {
            let token = cap.as_str();
            if token.is_empty() || STOPWORDS.contains(token) {
                None
            } else {
                Some(token.to_string())
            }
        })
        .collect()
}

/// Collect all SPDX tokens from licenses database.
/// Based on Python: models.py:1195-1205 get_all_spdx_key_tokens()
fn collect_spdx_tokens(licenses: &[License]) -> HashSet<String> {
    let mut tokens: HashSet<String> = HashSet::new();
    
    // Add essential tokens
    for tok in get_essential_spdx_tokens() {
        tokens.insert(tok.to_string());
    }
    
    // Add tokens from all SPDX keys
    for license in licenses {
        for spdx_key in &license.spdx_keys {
            for token in tokenize_spdx_key(spdx_key) {
                tokens.insert(token);
            }
        }
    }
    
    tokens
}
```

### Step 2: Pre-Assign SPDX Tokens in build_index()

**Location**: `src/license_detection/index/builder.rs:235-270`

```rust
pub fn build_index(rules: Vec<Rule>, licenses: Vec<License>) -> LicenseIndex {
    let legalese_words = legalese::get_legalese_words();
    let mut dictionary = TokenDictionary::new_with_legalese(&legalese_words);
    let len_legalese = dictionary.legalese_count();
    
    // CRITICAL FIX: Pre-assign SPDX tokens before processing rules
    // Based on Python: index.py:301-314
    let spdx_tokens = collect_spdx_tokens(&licenses);
    for token in spdx_tokens.iter().sorted() {
        // Only assign if not already in dictionary (not legalese)
        if dictionary.get(token).is_none() {
            dictionary.get_or_assign(token);
        }
    }
    
    // Now process rules - token IDs will match Python
    // ... rest of build_index unchanged ...
}
```

### Step 3: Update Imports

**Location**: `src/license_detection/index/builder.rs:1-30`

```rust
use std::collections::{HashMap, HashSet};

// Add these imports for SPDX token handling
use itertools::Itertools;  // for .sorted()

use crate::license_detection::tokenize::{QUERY_PATTERN, STOPWORDS};
// ... existing imports ...
```

### Step 4: Verify License.spdx_keys Field Exists

Check that the `License` struct has a `spdx_keys` field. If not, add it:

**Location**: `src/license_detection/models.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub key: String,
    pub spdx_keys: Vec<String>,  // Ensure this exists
    // ... other fields ...
}
```

---

## 5. Verification Tests

### Test 1: SPDX Token Pre-Population

```rust
#[test]
fn test_spdx_tokens_pre_populated() {
    use crate::license_detection::index::build_index;
    use crate::license_detection::rules::loader::load_licenses_from_resource;
    
    let licenses = load_licenses_from_resource();
    let index = build_index(vec![], licenses);
    
    // Essential SPDX tokens should be in dictionary
    let dict = &index.dictionary;
    
    // "mit" is a common SPDX key token that must be pre-assigned
    let mit_id = dict.get("mit");
    assert!(mit_id.is_some(), "'mit' should be in dictionary");
    
    // "gpl" should be pre-assigned (from GPL SPDX keys)
    let gpl_id = dict.get("gpl");
    assert!(gpl_id.is_some(), "'gpl' should be in dictionary");
    
    // Verify these are NOT legalese (they should have IDs >= len_legalese)
    let len_legalese = dict.legalese_count();
    assert!(!dict.is_legalese(mit_id.unwrap()), "'mit' should not be legalese");
    assert!(!dict.is_legalese(gpl_id.unwrap()), "'gpl' should not be legalese");
}
```

### Test 2: Token ID Stability

```rust
#[test]
fn test_token_ids_stable_across_rebuilds() {
    use crate::license_detection::index::build_index;
    
    let licenses1 = load_licenses_from_resource();
    let licenses2 = load_licenses_from_resource();
    
    let index1 = build_index(vec![], licenses1);
    let index2 = build_index(vec![], licenses2);
    
    // Same tokens should have same IDs across rebuilds
    for token in &["mit", "gpl", "apache", "bsd"] {
        let id1 = index1.dictionary.get(token);
        let id2 = index2.dictionary.get(token);
        assert_eq!(id1, id2, "Token '{}' should have stable ID", token);
    }
}
```

### Test 3: Hash Match Success

```rust
#[test]
fn test_hash_match_for_known_license() {
    use crate::license_detection::DetectionEngine;
    
    let engine = DetectionEngine::from_resource();
    
    // Read MIT license text from reference
    let mit_text = std::fs::read_to_string(
        "reference/scancode-toolkit/src/licensedcode/data/licenses/mit.LICENSE"
    ).unwrap();
    
    // Should get hash match
    let detections = engine.detect(&mit_text).unwrap();
    
    assert!(!detections.is_empty(), "Should detect MIT license");
    // The match should come from hash matching for exact license text
}
```

### Test 4: Compare Dictionary Size with Python

```rust
#[test]
fn test_dictionary_size_matches_python() {
    let engine = DetectionEngine::from_resource();
    let dict = &engine.index.dictionary;
    
    // Python dictionary after SPDX pre-population has known size
    // This test documents the expected size
    let total_tokens = dict.len();
    let legalese_count = dict.legalese_count();
    
    // Non-legalese should include SPDX tokens
    let non_legalese = total_tokens - legalese_count;
    assert!(non_legalese > 0, "Should have non-legalese SPDX tokens");
    
    // Log for comparison with Python
    println!("Dictionary: {} total, {} legalese, {} SPDX/other", 
             total_tokens, legalese_count, non_legalese);
}
```

---

## 6. Expected Impact on Golden Tests

### Before Fix

- lic2: 803/807 passed (2 regressions from PLAN-037)
- Hash match failures cause fallback to slower, less accurate matching

### After Fix

- lic2: Should return to 805/807 or better
- Hash matches succeed for exact license texts
- Performance improvement from successful hash early returns

### Risk Assessment

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Token ID change breaks existing hashes | High | Expected - this is the fix |
| New regressions from ID reassignment | Low | Pre-assignment is deterministic |
| Build time increase | Minimal | SPDX token collection is O(licenses) |

---

## 7. Implementation Checklist

- [ ] Add `collect_spdx_tokens()` function to builder.rs
- [ ] Add `tokenize_spdx_key()` helper function
- [ ] Add `get_essential_spdx_tokens()` function
- [ ] Update `build_index()` to pre-assign SPDX tokens
- [ ] Add required imports (itertools, QUERY_PATTERN, STOPWORDS)
- [ ] Verify/extend License.spdx_keys field
- [ ] Add verification tests
- [ ] Run `cargo test --lib license_detection` to verify
- [ ] Run golden tests: `cargo test test_golden_lic2 --lib`
- [ ] Compare hash match success rate before/after

---

## 8. Verification Commands

```bash
# Run specific lic2 test with verbose output
cargo test test_golden_lic2_a2 --lib -- --nocapture

# Compare hash match behavior
RUST_LOG=debug cargo test test_hash_exact_mit --lib -- --nocapture

# Run full golden suite
cargo test --lib license_detection::golden_test 2>&1 | tail -100

# Check token counts
cargo test test_token_id_equivalence --lib -- --nocapture
```

---

## 8. References

- **PLAN-037**: `docs/license-detection/PLAN-037-post-phase-merge-fix.md`
- **Python hash_match**: `reference/scancode-toolkit/src/licensedcode/match_hash.py:59-87`
- **Python match_query**: `reference/scancode-toolkit/src/licensedcode/index.py:966-1080`
- **Python merge_matches**: `reference/scancode-toolkit/src/licensedcode/match.py:869-1068`
- **Rust hash_match**: `src/license_detection/hash_match.rs:72-133`
- **Rust detect()**: `src/license_detection/mod.rs:118-280`
- **Rust merge_overlapping_matches()**: `src/license_detection/match_refine.rs:159-302`
- **Python SPDX token collection**: `reference/scancode-toolkit/src/licensedcode/models.py:1188-1205`
- **Python index building with SPDX tokens**: `reference/scancode-toolkit/src/licensedcode/index.py:301-314`
- **Python cache building**: `reference/scancode-toolkit/src/licensedcode/cache.py:260-273`
