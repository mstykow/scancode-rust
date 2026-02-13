# Query Performance Refactoring Plan

## Problem

The license detection tests are extremely slow (~36s for 10 tests). Root cause:

```rust
// In detect()
let query = Query::new(text, (*self.index).clone())?;
```

The `Query` struct owns a `LicenseIndex` by value, and `detect()` clones the entire index (~36,000 rules, HashMaps, HashSets, Aho-Corasick automaton) on every call.

## Solution

Change `Query` to hold a **reference** to `LicenseIndex` instead of owning it:

```rust
pub struct Query<'a> {
    // ... other fields ...
    pub index: &'a LicenseIndex,
}
```

## Files to Modify

### 1. `src/license_detection/query.rs`

- Change `Query` to use lifetime parameter `<'a>`
- Change `index: LicenseIndex` to `index: &'a LicenseIndex`
- Update all methods that create/return Query
- Update `QueryRun` similarly if it owns Query

### 2. `src/license_detection/mod.rs`

- Update `detect()` to pass `&self.index` instead of cloning
- Update any other methods that create Query

### 3. `src/license_detection/aho_match.rs`

- Update function signatures to accept `&LicenseIndex` or `&Query`

### 4. `src/license_detection/hash_match.rs`

- Update function signatures

### 5. `src/license_detection/seq_match.rs`

- Update function signatures

### 6. `src/license_detection/spdx_lid.rs`

- Update function signatures

### 7. `src/license_detection/unknown_match.rs`

- Update function signatures

### 8. `src/license_detection/match_refine.rs`

- Update function signatures

### 9. Tests

- Update all tests that create Query manually
- Ensure lifetime parameters propagate correctly

## Execution Plan

### Phase 1: Analyze current structure

- Map all places where Query is created
- Map all places where LicenseIndex is used
- Identify lifetime constraints

### Phase 2: Refactor Query struct

- Add lifetime parameter to Query
- Change index field to reference
- Fix compilation errors

### Phase 3: Update all callers

- Update matchers to accept references
- Update detect() method
- Update tests

### Phase 4: Verify

- Run tests
- Measure performance improvement
- Run clippy

## Expected Impact

- **Before**: ~36s for 10 tests
- **After**: Expected ~1-3s for 10 tests (eliminating ~36,000 HashMap clones per test)

## Risk

Lifetime propagation can be tricky. The engine holds `Arc<LicenseIndex>`, so references are valid for the engine's lifetime. Query should borrow from the engine.
