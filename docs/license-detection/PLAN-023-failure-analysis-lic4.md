# PLAN-023: lic4 Golden Test Failure Analysis

**Status**: Analysis Complete  
**Date**: 2025-02-20  
**Suite**: lic4 (350 tests)

## Summary

| Metric | Count |
|--------|-------|
| Total Tests | 350 |
| Passed | 285 |
| Failed | 65 |
| Pass Rate | 81.4% |

---

## Failure Pattern Analysis

### Pattern 1: Duplicate Detection Count (22 failures)

**Description**: Rust produces fewer OR more license detections than Python expects.

**Sub-patterns**:

- **Fewer detections** (10 cases): Rust merges/combines matches that Python keeps separate
- **More detections** (12 cases): Rust produces duplicate expressions Python doesn't

**Examples**:

| File | Expected | Actual | Issue |
|------|----------|--------|-------|
| `gpl-2.0-plus_and_gpl-2.0-plus.txt` | 2× gpl-2.0-plus | 1× gpl-2.0-plus | Merged identical matches |
| `putty.txt` | 3× mit | 1× mit | Merged identical licenses |
| `aac` | 1× fraunhofer-fdk-aac-codec | 2× fraunhofer-fdk-aac-codec | Extra detection |
| `here-proprietary_4.RULE` | 1× here-proprietary | 2× here-proprietary | Extra detection |
| `gplv2+ce.html` | 4× gpl-2.0 WITH classpath-exception-2.0 | 1× gpl-2.0 WITH classpath-exception-2.0 | Merged identical matches |

**Root Cause**: `merge_overlapping_matches()` and `filter_contained_matches()` logic differs from Python. Python's `merge_matches()` uses `qdistance_to()` and `idistance_to()` with `max_rule_side_dist` to control merge distance, while Rust merges more aggressively based on token overlap only.

**Code Locations**:

- `src/license_detection/match_refine.rs:128-229` - `merge_overlapping_matches()`
- `reference/scancode-toolkit/src/licensedcode/match.py:869-1068` - Python `merge_matches()`

**Recommendation**: Investigate merge distance thresholds and `qdistance_to()` / `idistance_to()` usage in Python's merge logic.

---

### Pattern 2: Empty Detection (5 failures)

**Description**: Rust produces no license detection where Python finds one.

**Examples**:

| File | Expected | Actual |
|------|----------|--------|
| `isc_only.txt` | isc | (empty) |
| `isc_redhat.txt` | isc | (empty) |
| `lgpl_21.txt` | lgpl-2.0-plus | (empty) |
| `warranty-disclaimer_1.txt` | warranty-disclaimer | (empty) |
| `proprietary_9.txt` | proprietary-license | (empty) |

**Root Cause Analysis**:

- `isc_only.txt`: Contains only "Copyright: ISC" in an RPM spec file - Python detects ISC license from this reference
- `warranty-disclaimer_1.txt`: Contains a short Microsoft warranty disclaimer - Python detects it, Rust doesn't
- `lgpl_21.txt`: Contains just "lgpl" text - Python detects lgpl-2.0-plus, Rust doesn't

**Root Cause**: License tag/reference detection differs. Python has rules that match short license references like "ISC", "lgpl" alone, while Rust may filter these as too short or not match them at all.

**Code Locations**:

- `src/license_detection/aho_match.rs` - Aho-Corasick matching
- `src/license_detection/match_refine.rs:62-84` - `filter_too_short_matches()`
- License rules in `resources/licenses/rules/`

**Recommendation**: Check if license tag rules for ISC, LGPL, warranty-disclaimer exist and are being filtered.

---

### Pattern 3: Wrong License Expression Type (8 failures)

**Description**: Rust detects a different license expression than Python expects.

**Examples**:

| File | Expected | Actual | Issue |
|------|----------|--------|-------|
| `airo.c` | gpl-2.0 OR bsd-new | unknown-license-reference, unknown-license-reference | No detection |
| `openjdk-assembly-exception.html` | openjdk-exception, gpl-2.0 WITH classpath-exception-2.0 | gpl-2.0 WITH openjdk-exception, gpl-2.0 WITH classpath-exception-2.0 | Different exception combination |
| `should_detect_something_6.rtf` | pdl-1.0 | proprietary-license | Wrong license |
| `zip_not_gpl.c` | warranty-disclaimer | proprietary-license | Wrong license |

**Root Cause Analysis**:

- `airo.c`: Text says "released under both GPL version 2 and BSD licenses" but Rust can't parse this dual-license pattern
- `openjdk-assembly-exception.html`: Rust combines openjdk-exception with gpl-2.0 differently than Python
- RTF/PDF files: Text extraction or preprocessing may differ

**Root Cause**:

1. Dual-license text parsing ("both X and Y" → "X OR Y") not implemented
2. Exception combination logic differs
3. RTF/PDF text extraction may be missing or different

**Code Locations**:

- `src/license_detection/expression.rs` - Expression combination
- `src/license_detection/detection.rs` - Detection assembly
- Text extraction for RTF/PDF

**Recommendation**:

1. Add handling for "both X and Y" dual-license patterns
2. Compare openjdk-exception rule handling between Python and Rust

---

### Pattern 4: Extra Unknown/Warranty Detections (6 failures)

**Description**: Rust produces extra "unknown-license-reference" or "warranty-disclaimer" detections not in Python output.

**Examples**:

| File | Expected | Actual |
|------|----------|--------|
| `gpl-2.0-plus_and_gfdl-1.1_debian.txt` | gpl-2.0-plus, gfdl-1.1-plus | gpl-2.0-plus, unknown-license-reference, gfdl-1.1-plus |
| `ijg.txt` | ijg | ijg, warranty-disclaimer, ijg, free-unknown, ... |
| `hs-regexp_and_proprietary_and_x11-opengroup.txt` | x11-opengroup, x11-xconsortium, ... | x11-opengroup, ghostpdl-permissive, unknown, unknown, ... |

**Root Cause**: Rust's unknown license matching and warranty-disclaimer detection is more aggressive than Python's, producing matches that Python filters out.

**Code Locations**:

- `src/license_detection/unknown_match.rs` - Unknown license matching
- `src/license_detection/match_refine.rs:42-60` - `filter_invalid_contained_unknown_matches()`

**Recommendation**: Tighten unknown license matching thresholds or add more filtering.

---

### Pattern 5: Expression Parentheses Difference (1 failure)

**Description**: License expression rendered with different parentheses.

**Example**:

| File | Expected | Actual |
|------|----------|--------|
| `plantuml_license_notice.txt` | mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus | (mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus) |

**Root Cause**: Rust adds outer parentheses to OR expressions when they shouldn't be there. PLAN-012 fixed this for WITH expressions but OR expressions in certain contexts still get wrapped.

**Code Locations**:

- `src/license_detection/expression.rs:426-430` - Expression rendering

**Recommendation**: Check if this is from expression parsing (input has parens) or rendering (output adds parens).

---

### Pattern 6: Binary/Non-UTF8 Files (5 failures)

**Description**: Test files contain binary data and can't be read as UTF-8.

**Examples**:

| File | Error |
|------|-------|
| `NamespaceNode.class` | stream did not contain valid UTF-8 |
| `should_detect_something_4.pdf` | stream did not contain valid UTF-8 |
| `should_detect_something_5.pdf` | stream did not contain valid UTF-8 |
| `w3c_1.txt` | stream did not contain valid UTF-8 |

**Root Cause**: Rust tries to read binary files as UTF-8 text. Python likely has text extraction for PDF/class files.

**Recommendation**: Add text extraction for binary formats or skip binary files with appropriate error handling.

---

### Pattern 7: Match Count/Sequence Difference (18 failures)

**Description**: The sequence of detected licenses differs between Python and Rust in non-trivial ways.

**Examples**:

| File | Issue |
|------|-------|
| `openssh.LICENSE` | Expected 14 detections, actual 13; different license at position 6 |
| `kde_licenses_test.txt` | Expected 15 detections, actual 16; lgpl-2.0 vs lgpl-2.1 differences |
| `should_detect_something.html` | Expected 5 detections, actual 11; extra proprietary/warranty/unknown matches |
| `url_badge.md` | Expected 33 detections, actual 27; CC license duplicates missing |

**Root Cause**: Complex interaction between:

- Match grouping by region
- License intro/clue detection
- False positive filtering
- Expression combination

**Code Locations**:

- `src/license_detection/detection.rs:136-199` - `group_matches_by_region()`
- `src/license_detection/match_refine.rs:698-775` - `filter_false_positive_license_lists_matches()`

**Recommendation**: Compare match-by-match output between Python and Rust for specific test cases.

---

## Root Cause Summary by Code Area

| Area | Failure Count | Key Issue |
|------|---------------|-----------|
| Match merging | ~22 | Merge distance/overlap logic differs |
| License tag rules | ~5 | Short license references not detected |
| Unknown filtering | ~6 | Too aggressive on unknown/warranty |
| Expression rendering | ~1 | Parentheses on OR expressions |
| Binary handling | ~5 | No text extraction |
| Complex interaction | ~18 | Multiple factors in detection pipeline |

---

## Priority Recommendations

### High Priority

1. **Merge Distance Logic** (`match_refine.rs:128-229`)
   - Implement `idistance_to()` for index-based distance
   - Add `max_rule_side_dist` threshold like Python
   - Reference: `match.py:906` - `max_rule_side_dist = min((rule_length // 2) or 1, max_dist)`

2. **Short License Tag Detection**
   - Verify ISC, LGPL, warranty-disclaimer rules exist
   - Check `filter_too_short_matches()` thresholds
   - Ensure license reference/tag rules are loaded

### Medium Priority

1. **Unknown License Filtering**
   - Tighten thresholds in `unknown_match.rs`
   - Add more containment filtering

2. **Binary File Handling**
   - Add PDF text extraction (e.g., pdf-extract crate)
   - Add class file handling or skip gracefully

### Low Priority

1. **Expression Parentheses**
   - Investigate single remaining case in plantuml test

2. **Complex Interaction Debugging**
   - Add detailed logging for match-by-match comparison
   - Create test harness for Python/Rust diff output

---

## Files to Investigate

| File | Purpose | Issue |
|------|---------|-------|
| `src/license_detection/match_refine.rs` | Match merging/filtering | Merge distance, containment |
| `src/license_detection/aho_match.rs` | License tag matching | Short reference detection |
| `src/license_detection/unknown_match.rs` | Unknown license detection | Filtering thresholds |
| `src/license_detection/detection.rs` | Detection assembly | Grouping logic |
| `resources/licenses/rules/` | License rules | Missing rules for ISC, etc. |

---

## Next Steps

1. Run single test with verbose output to compare Python vs Rust match-by-match
2. Implement `idistance_to()` method for proper merge distance
3. Add missing license tag rules or fix filtering
4. Add binary file text extraction or graceful handling
