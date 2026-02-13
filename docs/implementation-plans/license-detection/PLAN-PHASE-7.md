# Phase 7 Completion Plan

> **Status**: ✅ COMPLETE (2026-02-13)

## Overview

Phase 7 was marked as PARTIAL PASS. Three issues were identified and have been resolved:

1. **CRITICAL**: MIT license text detection produces incorrect results (detects `psytec-freesoft` instead of `mit`) - ✅ FIXED
2. **MEDIUM**: `--include-text` CLI flag not implemented - ✅ FIXED
3. **MEDIUM**: `matched_text` field not populated in matches - ✅ FIXED

---

## Issue 1: MIT License Detection Bug (CRITICAL)

### Root Cause Analysis

The Aho-Corasick matcher has a **critical mapping bug** between automaton pattern IDs and rule IDs.

In `src/license_detection/aho_match.rs:108-111`:

```rust
let rid = pattern_id.as_usize();
```

**This is incorrect!** The `pattern_id` from the AhoCorasick library is the index of the pattern in the patterns iterator used to build the automaton (0, 1, 2, ...), NOT the actual rule ID (rid).

### Python Reference Behavior

Python's `match_aho.py:52-75` explicitly stores the rid with each pattern:

```python
def add_sequence(automaton, tids, rid, start=0, with_duplicates=False):
    value = rid, start, start + end
    tokens = tuple(tids)
    automaton.add_word(tokens, [value])  # stores (rid, start, end) as value
```

When matching (match_aho.py:169-176):

```python
def get_matched_positions(tokens, qbegin, automaton):
    for qend, matched_rule_segments in get_matches(tokens, qbegin, automaton):
        for rid, istart, iend in matched_rule_segments:  # rid comes from stored value
            ...
```

The automaton's **value** (not the pattern index) contains the actual rid.

### Rust Implementation Gap

The Rust implementation:

1. Does NOT store rid with each pattern in the automaton
2. Incorrectly assumes `pattern_id.as_usize()` is the rid
3. This causes completely wrong rules to be matched

Example: If pattern 42 in the automaton corresponds to rule with rid=500, the Rust code will use rid=42 instead of rid=500.

### Fix Plan

**Step 1**: Modify `src/license_detection/index/mod.rs` to store rid-to-pattern mapping.

Create a `Vec<usize>` that maps `pattern_id -> rid` when building the automaton.

**Step 2**: Update `src/license_detection/aho_match.rs` to use the mapping.

Change line 108 from:

```rust
let rid = pattern_id.as_usize();
```

To:

```rust
let rid = self.index.pattern_id_to_rid[pattern_id.as_usize()];
```

**Step 3**: Ensure the mapping is populated during index building.

In `build_index()`, when adding patterns to the automaton, track the mapping.

### Files to Modify

1. **`src/license_detection/index/mod.rs`**
   - Add field `pattern_id_to_rid: Vec<usize>` to `LicenseIndex`
   - Populate this mapping when building `rules_automaton`
   - Line ~100-150 in `build_index()` function

2. **`src/license_detection/aho_match.rs`**
   - Line 108: Use the mapping instead of direct pattern_id
   - Lines 109-111: Guard against index out of bounds with the mapping

### Verification

1. Run the existing test:

   ```bash
   cargo test test_engine_detect_mit_license -- --nocapture
   ```

   Should detect "mit" license expression.

2. Run against MIT license text file:

   ```bash
   cargo run -- reference/scancode-toolkit/src/licensedcode/data/licenses/mit.LICENSE -o test-output.json
   ```

   Should show `"license_expression": "mit"` in output.

---

## Issue 2: `--include-text` Flag Missing (MEDIUM)

### Root Cause Analysis

The `--include-text` flag was specified in the Phase 7 plan but never implemented. This flag controls whether `matched_text` is included in JSON output.

### Python Reference Behavior

Python's `LicenseDetection.to_dict()` method (detection.py:476-500):

```python
def to_dict(self, include_text=False, ...):
    for match in self.matches:
        data_matches.append(
            match.to_dict(
                include_text=include_text,  # passed through
                ...
            )
        )
```

The `include_text` parameter is passed from CLI through to match serialization.

### Rust Implementation Gap

1. `src/cli.rs` has no `--include-text` flag
2. The flag is not passed through the scanner pipeline
3. Match serialization doesn't conditionally include `matched_text`

### Fix Plan

**Step 1**: Add CLI flag in `src/cli.rs`:

```rust
/// Include matched text in license detection output
#[arg(long)]
pub include_text: bool,
```

**Step 2**: Pass flag through scanner pipeline in `src/main.rs`:

- Store in a configuration struct or pass directly to `process()`

**Step 3**: Update `src/models/file_info.rs` Match serialization:

- Add conditional `matched_text` inclusion based on flag

### Files to Modify

1. **`src/cli.rs`**
   - Add `include_text: bool` field

2. **`src/main.rs`**
   - Pass `include_text` flag to scanner
   - Store in context accessible to file processing

3. **`src/scanner/process.rs`**
   - Accept `include_text: bool` parameter
   - Pass to match conversion

4. **`src/models/file_info.rs`**
   - Update `Match` struct serialization
   - Skip `matched_text` field if `include_text` is false and text is empty

### Verification

```bash
# Without flag - matched_text should be null/absent
cargo run -- testdata/licenses/mit.txt -o output.json
cat output.json | grep matched_text  # should show null

# With flag - matched_text should have content
cargo run -- testdata/licenses/mit.txt --include-text -o output.json
cat output.json | grep matched_text  # should show license text
```

---

## Issue 3: `matched_text` Not Populated (MEDIUM)

### Root Cause Analysis

The `matched_text` field in `LicenseMatch` is set to `None` in all matchers. The text is never extracted from the query using the matched position (qspan).

### Python Reference Behavior

Python's `LicenseMatch.matched_text()` method (match.py:630-660):

```python
def matched_text(self, whole_lines=True, ...):
    """Return the matched text from the query."""
    return self.query.text_at_span(self.qspan, whole_lines=whole_lines)
```

The match stores a reference to the Query object and can extract text using `qspan` positions.

### Rust Implementation Gap

1. `LicenseMatch` has `matched_text: Option<String>` but it's never populated
2. Matchers (hash_match, aho_match, seq_match) set `matched_text: None`
3. The Query object is not accessible from LicenseMatch

### Fix Plan

**Step 1**: Populate `matched_text` in match creation.

Each matcher should extract matched text from the query using qspan positions:

```rust
let matched_text = query_run.text_at_span(qstart, qend);
```

**Step 2**: Add helper method to `QueryRun` or `Query`:

```rust
impl QueryRun {
    pub fn matched_text(&self, start: usize, end: usize) -> Option<String> {
        // Extract text from original query string using positions
    }
}
```

**Step 3**: Update all matchers:

- `src/license_detection/hash_match.rs`
- `src/license_detection/aho_match.rs`
- `src/license_detection/seq_match.rs`
- `src/license_detection/spdx_lid.rs`
- `src/license_detection/unknown_match.rs`

### Files to Modify

1. **`src/license_detection/query.rs`**
   - Add method to extract text at span positions
   - Store original text reference in Query struct

2. **`src/license_detection/models.rs`** (or individual matcher files)
   - Populate `matched_text` when creating LicenseMatch

3. **All matcher files** - Set matched_text when creating matches

### Verification

```bash
cargo run -- reference/scancode-toolkit/src/licensedcode/data/licenses/mit.LICENSE --include-text -o output.json
# Check that matches have non-null matched_text
cat output.json | jq '.files[0].license_detections[0].matches[0].matched_text'
```

---

## Implementation Order

1. **FIRST: Issue 1 (MIT Detection Bug)** - This is critical and affects all detection correctness
   - Fix the pattern_id to rid mapping
   - This alone should fix the MIT detection

2. **SECOND: Issue 3 (matched_text)** - Depends on correct detection working
   - Add text extraction from query
   - Update all matchers

3. **THIRD: Issue 2 (include-text flag)** - Depends on matched_text being populated
   - Add CLI flag
   - Wire through pipeline

---

## Testing Strategy

### Unit Tests

1. **Pattern ID Mapping Test**

   ```rust
   #[test]
   fn test_pattern_id_maps_to_correct_rid() {
       // Build index with known rules
       // Add patterns to automaton in specific order
       // Verify pattern_id_to_rid mapping is correct
   }
   ```

2. **MIT Detection Test** (already exists, should pass after fix)

   ```rust
   #[test]
   fn test_engine_detect_mit_license() {
       // Should now correctly detect "mit" instead of wrong license
   }
   ```

### Integration Tests

1. Run against reference license files:

   ```bash
   for license in reference/scancode-toolkit/src/licensedcode/data/licenses/*.LICENSE; do
       cargo run -- "$license" -o test-output.json
       # Verify correct license_expression detected
   done
   ```

2. Compare with Python reference output for MIT license:

   ```bash
   # Python
   scancode reference/scancode-toolkit/src/licensedcode/data/licenses/mit.LICENSE -o python-output.json
   
   # Rust (after fix)
   cargo run -- reference/scancode-toolkit/src/licensedcode/data/licenses/mit.LICENSE -o rust-output.json
   
   # Compare license_expression
   diff <(jq '.files[0].license_detections[0].license_expression' python-output.json) \
        <(jq '.files[0].license_detections[0].license_expression' rust-output.json)
   ```

### Regression Tests

Run full test suite after all fixes:

```bash
cargo test --all
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

---

## Detailed Code Changes for Issue 1

### `src/license_detection/index/mod.rs`

Add field to `LicenseIndex`:

```rust
pub struct LicenseIndex {
    // ... existing fields ...
    
    /// Maps AhoCorasick pattern_id to rule id (rid).
    /// This is needed because pattern_id is just the index in the
    /// patterns iterator, not the actual rule id.
    pub pattern_id_to_rid: Vec<usize>,
}
```

Update `build_index()`:

```rust
pub fn build_index(rules: Vec<Rule>, licenses: Vec<License>) -> LicenseIndex {
    // ... existing code ...
    
    let mut pattern_id_to_rid = Vec::new();
    let mut patterns: Vec<Vec<u8>> = Vec::new();
    
    // Sort rules by rid to ensure consistent ordering
    let mut sorted_rules: Vec<_> = rules.iter().enumerate().collect();
    sorted_rules.sort_by_key(|(_, r)| r.license_expression.clone());
    
    for (rid, rule) in sorted_rules {
        let tids = &tids_by_rid[rid];
        let pattern_bytes = tokens_to_bytes(tids);
        patterns.push(pattern_bytes);
        pattern_id_to_rid.push(rid);
    }
    
    let rules_automaton = AhoCorasickBuilder::new()
        .build(patterns.iter().map(|p| p.as_slice()))
        .unwrap();
    
    LicenseIndex {
        // ... existing fields ...
        pattern_id_to_rid,
    }
}
```

### `src/license_detection/aho_match.rs`

Fix the rid lookup:

```rust
pub fn aho_match(index: &LicenseIndex, query_run: &QueryRun) -> Vec<LicenseMatch> {
    // ... existing code ...
    
    for ac_match in automaton.find_iter(&encoded_query) {
        let pattern_id = ac_match.pattern();
        
        // FIXED: Use the mapping instead of assuming pattern_id == rid
        let rid = index.pattern_id_to_rid.get(pattern_id.as_usize());
        let Some(&rid) = rid else {
            continue;  // Invalid pattern_id, skip
        };
        
        // ... rest of match creation ...
    }
}
```

---

## Risk Assessment

### High Risk Areas

1. **Automaton pattern ordering** - Must ensure patterns are added in consistent order
2. **Memory for pattern_id_to_rid** - One entry per rule, should be manageable
3. **Breaking existing tests** - Fix should make tests pass, but verify all

### Mitigation

1. Add assertion in index building to verify mapping is complete
2. Add debug logging to trace pattern_id -> rid resolution
3. Run full test suite after each change

---

## Success Criteria

- [x] MIT license text correctly detected as "mit"
- [x] All existing tests continue to pass (1758 tests)
- [x] `--include-text` flag added and functional
- [x] `matched_text` populated when `--include-text` is specified
- [x] No regressions in other license detection

## Commits

1. `718437e` - fix(license-detection): correct pattern_id to rid mapping in Aho-Corasick matcher
2. `f1bc462` - feat(license-detection): populate matched_text field in all matchers
3. `2f8bae6` - feat(cli): add --include-text flag for matched_text in output
