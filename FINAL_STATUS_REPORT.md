# License Detection Golden Tests - Final Status Report

## Test Progress
- **Starting baseline**: 96 failing tests  
- **Current baseline**: 90 failing tests
- **Improvement**: 6 tests fixed (6.25%)
- **Total commits**: 11

## Commits Made
1. MAX_DIST: 100 → 50 (matches Python)
2. QueryRun implementation (with fixes)
3. O(n³) → O(n²) performance fix
4. 100KB size limit (prevents hanging)
5. QueryRun disabled (due to regressions)
6. Candidate selection fix (alphabetical tiebreaker)
7. Expression rendering fix (remove extra parentheses)
8. Build fix (removed orphaned module)
9. German normalization (reverted - breaks Unicode)
10. Investigation cleanup

## Hypothesis Investigation Summary

### H1: QueryRun Splitting - PENDING
- **Impact**: ~25 tests
- **Status**: Disabled due to cross-run filtering issues
- **Root cause**: Missing detection grouping logic
- **Next step**: Implement proper cross-run match filtering

### H2: Multi-Occurrence Deduplication - INVESTIGATED  
- **Impact**: ~25 tests
- **Status**: Root cause identified
- **Root cause**: Containment filtering removes smaller matches
- **Example**: flex-readme.txt has 3 matches, Rust finds 1
- **Next step**: Study Python's detection creation from matches

### H3: Required Phrase Validation - INVESTIGATED
- **Impact**: ~20 tests  
- **Status**: NOT a required phrase issue
- **Root cause**: Candidate ranking for license variants (SA vs NC-SA)
- **Fix applied**: Alphabetical tiebreaker (helps some cases)
- **Remaining issue**: Need better specificity checking

### H4: German Text ß Character - PYTHON PARITY
- **Impact**: ~15 tests
- **Status**: Achieved parity with Python
- **Finding**: Python has SAME issue (ß doesn't match ss)
- **Dead rules**: gpl-2.0-plus_14.RULE in both Python and Rust
- **Action**: Accept as Python parity, document in DIFFERENCES.md

### H5: Match Ordering - REJECTED
- **Status**: Not an issue
- **Finding**: Ordering IS deterministic (sorts by start_token)
- **Real cause**: Missing/extra matches, not ordering

### H6: Binary File Handling - FEATURE GAP
- **Impact**: ~3-5 tests
- **Status**: Significant feature gap
- **Root cause**: Rust doesn't extract text from binaries/PDFs
- **Python behavior**: Extracts strings from ELF, text from PDFs
- **Required**: PDF extraction + binary strings extraction
- **Effort**: Medium (requires new dependencies and modules)

## Remaining 90 Failures - Categories

1. **Multi-occurrence issues** (~25 tests)
   - Need detection grouping logic
   - Matches at different locations should each be reported

2. **QueryRun issues** (~25 tests)  
   - Need cross-run filtering
   - Proper handling of split text segments

3. **Candidate ranking** (~20 tests)
   - License variant selection (SA vs NC-SA, etc.)
   - Better specificity checking needed

4. **Identifier differences** (~10 tests)
   - Different SPDX expressions for same license
   - Template rule handling

5. **Binary/PDF files** (~3-5 tests)
   - Feature gap - requires implementation

6. **Edge cases** (~5-10 tests)
   - Various other issues

## Key Technical Achievements

✅ Performance fixed (O(n³) → O(n²))
✅ Stability fixed (no hanging on large files)  
✅ MAX_DIST aligned with Python
✅ Candidate selection improved
✅ Expression rendering cleaned up
✅ Size limit added
✅ Unicode preservation verified

## Files Modified

- `src/license_detection/match_refine/merge.rs` - MAX_DIST
- `src/license_detection/seq_match/candidates.rs` - Candidate ranking
- `src/license_detection/expression/simplify.rs` - Rendering
- `src/license_detection/query/mod.rs` - QueryRun (disabled)
- `src/license_detection/mod.rs` - Size limit
- `src/license_detection/tokenize.rs` - Unicode preservation

## Recommended Next Steps

### Short-term (Quick wins)
1. Accept German ß parity with Python
2. Document intentional differences in DIFFERENCES.md
3. Add unit tests for edge cases found

### Medium-term (Algorithm improvements)
1. Re-enable QueryRun with cross-run filtering
2. Implement detection grouping logic
3. Improve candidate specificity checking

### Long-term (Feature parity)
1. Add PDF text extraction
2. Add binary strings extraction
3. License alias normalization

## Testing Strategy

Golden tests run with:
```bash
cargo test --release -q --lib license_detection::golden_test 2>&1 | \
  grep "failed, 0 skipped" | sed 's/.*, \([0-9]*\) failed,.*/\1/' | paste -sd+ | bc
```

Reference comparison:
```bash
cd reference/scancode-playground && \
  venv/bin/python src/scancode/cli.py --license <file> --json-pp -
```

## Conclusion

The foundation is solid with good performance and stability. The remaining 90 failures require:
- Algorithm refinement (QueryRun, detection grouping)
- Better candidate selection logic
- Feature additions (PDF/binary extraction)

Progress is incremental but steady. Each investigation provides deeper understanding of Python's behavior and brings us closer to full parity.
