# PLAN-020: Golden Test Failure Investigation
This document contains the findings from investigating remaining golden test failures.
---
## Current Test Results
| Suite | Baseline | Current | Delta |
|-------|----------|---------|-------|
| lic1 | 228 | 226 | -2 |
| lic2 | 776 | 770 | -6 |
| lic3 | 251 | 250 | -1 |
| lic4 | 281 | 285 | +4 |
| external | 1882 | 1922 | +40 |
| unknown | 2 | 3 | +1 |
Total failures below baseline: **9 tests** (lic1: -2, lic2: -6, lic3: -1)
---
## Failure Categories
### Category A: Duplicate Detections (~200+ SPDX tests)
**Example:** `external/fossology-tests/SPDX/MIT`
- Expected: `["mit"]`
- Actual: `["mit", "mit"]`
**Cause:** Hash match + Aho-Corasick both fire for the same license text file.
**Files Affected:** Most SPDX license text files in external/
### Category B: Extra Matches
**Example:** `lgpl-2.1-plus_19.txt`
- Expected: `["lgpl-2.1-plus"]`
- Actual: `["lgpl-2.1-plus", "gpl-2.0-plus", "lgpl-2.1-plus", "gpl-1.0-plus", "other-copyleft"]`
**Cause:** Missing `is_license_text` subtraction - Rust finds GPL references inside LGPL text that Python filters out.
**Pattern:** Files containing long license texts with embedded references.
### Category C: Missing Detections
**Example:** `jcharts.txt`
- Expected: `["other-permissive"]`
- Actual: `[]`
**Cause:** Rule may not be loaded, tokenization differences, or threshold issues.
### Category D: Match Count Differences
**Example:** `mit_18.txt`
- Expected: `["mit", "mit", "mit"]`
- Actual: `["mit"]`
**Cause:** Python creates multiple matches within a detection; Rust creates fewer due to different merge/grouping behavior.
---
## Sample Failures by Suite
### lic1 Failures (65 total)
- `COPYING.gplv3` - Extra GPL-related matches
- `cddl-1.0_or_gpl-2.0-classpath_and_apache-2.0-glassfish_1.txt` - Extra apache-2.0 match
- `cddl-1.1.txt` - Extra matches with unknown-license-reference
- `ecos-2.0_spdx.c` - Missing detection (expected `gpl-2.0-plus WITH ecos-exception-2.0`, got `[]`)
- `freebsd-doc_4.txt`, `freebsd-doc_5.txt` - Missing detection
- `gpl-2.0-plus_1.txt` - Extra gpl-1.0-plus match
- `gpl-2.0_44.txt` - Wrong version (expected `gpl-2.0`, got `gpl-2.0-plus`)
### lic2 Failures (83 total)
- Similar patterns to lic1
- Multiple GFDL tests with extra matches
### lic3 Failures (42 total)
- `mit_18.txt` - Fewer matches (expected 3 MIT, got 1)
- `mit_and_mit.txt` - More matches (expected 1 MIT, got 2)
- `jcharts.txt` - Missing detection (expected `other-permissive`, got `[]`)
- `lgpl-2.1-plus_19.txt` - Extra matches (expected 1, got 5)
- `mit_31.txt` - Many extra matches (expected 4, got 13)
- `lzma-sdk-original.txt` - Wrong expression
- `mixed_ansible.txt` - Missing detection
---
## Root Cause Analysis
### Issue 1: Duplicate Detections in SPDX Files
**Most common failure (~200+ tests)**
Rust detects licenses twice when scanning pure license text files. Likely:
- Hash match and Aho-Corasick both firing
- Two different rules matching the same text (license text + license notice rule)
- Missing deduplication for 100% coverage matches
### Issue 2: Missing `is_license_text` Subtraction
**Affects files with long license texts**
Python subtracts matched regions for long license texts (>120 tokens, >98% coverage) to prevent spurious matches inside. Rust doesn't have this logic, causing extra matches in GPL/LGPL/MIT files.
### Issue 3: Missing Second `filter_contained_matches()`
**Affects files with restored matches**
After `restore_non_overlapping()` adds matches back, Python filters them again for containment. Rust doesn't, potentially keeping matches that should be filtered.
### Issue 4: Missing Final `merge_matches()`
**Affects files with fragmented matches**
Matches that should be combined after filtering remain separate in Rust.
---
## Common Patterns
| Pattern | Frequency | Root Cause |
|---------|-----------|------------|
| Duplicate detections in SPDX files | ~200+ | Hash + Aho double-firing |
| Extra matches in GPL/LGPL files | ~50+ | Missing `is_license_text` subtraction |
| Missing detections | ~20+ | Rule loading or threshold issues |
| Match count differences | ~10+ | Detection grouping behavior |
---
## Recommendations
### High Priority
1. **Fix duplicate detections for SPDX files**
   - Investigate why hash match and Aho-Corasick both fire
   - Add deduplication for 100% coverage matches that overlap completely
   - Location: `src/license_detection/mod.rs` match collection phase
2. **Implement `is_license_text` subtraction**
   - See PLAN-019 Part 1 for details
   - Location: `src/license_detection/mod.rs`
3. **Add second `filter_contained_matches()` call**
   - See PLAN-019 Part 3 for details
   - Location: `src/license_detection/match_refine.rs`
### Medium Priority
4. **Add final `merge_matches()` call**
   - See PLAN-019 Part 3 for details
5. **Investigate missing detections**
   - Check if rules like `other-permissive` are loaded
   - Compare tokenization for edge cases
### Investigation Commands
```bash
# Debug specific file
cargo test --release --lib debug_glassfish_detection -- --nocapture
# Run single golden test with output
cargo test --release --lib license_detection::golden_test::golden_tests::test_golden_lic3 -- --nocapture 2>&1 | less
# Check specific failure patterns
cargo test --release --lib license_detection::golden_test 2>&1 | grep "mismatch" | head -30
```
---
## Expected Impact After Fixes
| Fix | Expected Tests Fixed |
|-----|---------------------|
| Duplicate detection fix | ~200 SPDX tests |
| `is_license_text` subtraction | ~50 GPL/LGPL tests |
| Second `filter_contained_matches()` | ~20 tests |
| Final `merge_matches()` | ~10 tests |
Total estimated improvement: **~280 tests**
---
## Next Steps
1. Implement fixes from PLAN-019
2. Re-run golden tests
3. Investigate remaining failures if any
4. Update this document with new findings