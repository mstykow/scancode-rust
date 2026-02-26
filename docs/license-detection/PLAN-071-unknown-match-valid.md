# PLAN-071: Unknown Match Valid Test

## Status: RESOLVED

## Problem Statement

`test_create_unknown_match_valid` was failing - `create_unknown_match` returned `None` instead of `Some(LicenseMatch)`.

## Root Cause

The test used `Query::new()` with `LicenseIndex::with_legalese_count(10)` which creates an empty dictionary (no tokens mapped). The query had no legalese tokens, so `hispan` count was 0.

The `create_unknown_match` function requires:
1. `len(qspan) >= unknown_ngram_length * 4` (≥24 tokens)
2. `len(hispan) >= 5` (≥5 legalese tokens)

## Fix

Use `create_mock_query_with_tokens` to create a query with legalese tokens:

```rust
// Create tokens with IDs 0-9 (legalese tokens) so hispan count >= 5
let tokens: Vec<u16> = (0..30).collect();
let query = create_mock_query_with_tokens(&tokens, &index);
```

With `len_legalese = 10`, tokens 0-9 are marked as legalese, giving hispan count of 10.

## Python Reference

Python's `match_unknown.py:220`:
```python
if len(qspan) < unknown_ngram_length * 4 or len(hispan) < 5:
    return  # Skip weak unknown match
```

## Files Changed

- `src/license_detection/unknown_match.rs:546-554` (test)

## Tests Fixed

- `test_create_unknown_match_valid`
