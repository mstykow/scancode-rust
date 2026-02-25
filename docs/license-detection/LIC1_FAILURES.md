# Lic1 Golden Test Failures Analysis

## Summary

50 lic1 tests are failing. This document categorizes all failures.

---

## Failure Categories Overview

| Category | Count | Root Cause | Status |
|----------|-------|------------|--------|
| CDDL Rule Selection | 8 | Surround merge bug | Blocked by stricter-surround-merge |
| Duplicate Detections Merged | 16 | Matches incorrectly combined | Needs investigation |
| Extra Detections | 8 | Additional unexpected expressions | Needs investigation |
| Missing Detections | 12 | Fewer expressions than expected | Needs investigation |
| Wrong Detection | 6 | Completely different expression | Needs investigation |

---

## Category 1: CDDL Rule Selection (8 tests)

**Root Cause**: Surround merge not checking overlap before combining. CDDL 1.1 matches incorrectly inflate and beat CDDL 1.0.

**Status**: Blocked by `improvements/stricter-surround-merge.md` (deferred until Python parity)

| Test | Expected | Actual | Issue |
|------|----------|--------|-------|
| `cddl-1.0.txt` | `["cddl-1.0"]` | `["unknown-license-reference", "cddl-1.0"]` | Extra detection |
| `cddl-1.0_or_gpl-2.0-classpath_and_apache-2.0-glassfish_1.txt` | `["(cddl-1.0 OR ...) AND apache-2.0"]` | `["(cddl-1.1 OR ...) AND apache-2.0"]` | Wrong version |
| `cddl-1.0_or_gpl-2.0-classpath_and_apache-2.0-glassfish_2.txt` | `["(cddl-1.0 OR ...) AND apache-2.0"]` | `["(cddl-1.1 OR ...) AND apache-2.0"]` | Wrong version |
| `cddl-1.0_or_gpl-2.0-glassfish.txt` | `["cddl-1.0 OR gpl-2.0"]` | `["cddl-1.1 OR gpl-2.0 WITH classpath-exception-2.0"]` | Wrong version |
| `cddl-1.1.txt` | `["cddl-1.0"]` | `[]` | Missing detection |
| `cddl-1.1_or_gpl-2.0-classpath_and_apache-2.0-glassfish_1.txt` | `["(cddl-1.1 OR ...) AND apache-2.0"]` | `[...cddl-1.1..., ...cddl-1.0...]` | Extra CDDL 1.0 |
| `cddl-1.1_or_gpl-2.0-classpath_and_mit-glassfish.txt` | `["(...mit)"]` | `[...apache-2.0..., ...mit...]` | Extra detection |
| `cddl-1.1_or_gpl-2.0-classpath_and_w3c-glassfish.txt` | `["(...w3c)"]` | `[...w3c, ...w3c]` | Duplicate |

---

## Category 2: Duplicate Detections Merged (16 tests)

**Root Cause**: Multiple license instances in a file are being incorrectly merged into one detection.

**Pattern**: Expected N expressions, got N-1 or fewer.

| Test | Expected Count | Actual Count | Missing |
|------|----------------|--------------|---------|
| `cjdict-liconly.txt` | 8 | 5 | 3 bsd-new |
| `com-oreilly-servlet.txt` | 1 | 2 | Extra duplicate |
| `e2fsprogs.txt` | 5 | 4 | 1 lgpl-2.1-plus |
| `eclipse-openj9.LICENSE` | 9 | 8 | 1 zlib |
| `eclipse-openj9_html.html` | 13 | 11 | 1 mit, 1 zlib |
| `ecos-license.html` | 2 | 1 | 1 gpl-2.0-plus |
| `edl-1.0.txt` | 2 | 1 | 1 bsd-new |
| `flex-readme.txt` | 3 | 2 | 1 flex-2.5 |
| `fsf-free_and_fsf-free_and_fsf-free.txt` | 3 | 1 | 2 fsf-free |
| `fsf-free_and_fsf-free_and_fsf-free_and_gpl-2.0-autoconf_and_other.txt` | 4 | 2 | 2 fsf-free |
| `fsf-free_and_fsf-free_and_fsf-free_and_gpl-2.0-autoconf_and_other_1.txt` | 4 | 2 | 2 fsf-free |
| `fsf-unlimited-no-warranty_and_...txt` | 17 | 3 | 14 fsf-unlimited |
| `gfdl-1.2_1.RULE` | 1 | 2 | Extra duplicate |
| `gfdl-1.3_2.RULE` | 2 | 3 | Extra duplicate |
| `gpl-2.0_82.RULE` | 3 | 1 | 2 gpl-2.0 |
| `gpl_65.txt` | 2 | 1 | 1 gpl-1.0-plus |

---

## Category 3: Extra Detections (8 tests)

**Root Cause**: Additional unexpected license expressions detected.

**Pattern**: Expected N expressions, got N+1 or more.

| Test | Expected Count | Actual Count | Extra |
|------|----------------|--------------|-------|
| `complex.el` | 18 | 19 | 1 lgpl-2.0-plus |
| `curl_2.txt` | 1 | 2 | 1 unknown-license-reference |
| `epl-2.0.html` | 2 | 4 | 2 unknown, 1 proprietary |
| `gfdl-1.1-en_gnome_1.RULE` | 2 | 12 | 10 extra |
| `gfdl-1.1_1.RULE` | 3 | 11 | 8 extra |
| `gfdl-1.1_10.RULE` | 1 | 11 | 10 extra |
| `gfdl-1.1_9.RULE` | 2 | 10 | 8 extra |
| `gpl-2.0-plus_21.txt` | 1 | 3 | 2 extra |

---

## Category 4: Missing Detections (12 tests)

**Root Cause**: Expected license expressions not detected.

**Pattern**: Actual count significantly less than expected, or completely different licenses.

| Test | Expected Count | Actual Count | Notes |
|------|----------------|--------------|-------|
| `cpl-1.0_in_html.html` | 1 | 1 | Wrong: unknown-license-reference |
| `d-zlib_and_gfdl-1.2_and_gpl_and_gpl_and_other.txt` | 19 | 19 | 1 wrong expression |
| `fsf-unlimited_not_fsf-unlimited-no-warranty.txt` | 1 | 1 | Wrong: fsfullrsd |
| `godot2_COPYRIGHT.txt` | 95 | 44 | Many missing |
| `godot_COPYRIGHT.txt` | 84 | 47 | Many missing |
| `gpl-1.0-plus_or_artistic-1.0_licenses.txt` | 7 | 6 | 1 gpl-1.0 missing |
| `gpl-2.0-plus_33.txt` | 6 | 5 | 1 missing |
| `gpl-2.0_and_gpl-2.0-plus.txt` | 9 | 6 | 3 missing |
| `gpl-2.0_and_gpl-2.0_and_gpl-2.0-plus.txt` | 6 | 4 | 2 missing |
| `gpl-2.0_and_lgpl-2.0-plus.txt` | 1 | 2 | Extra gpl-2.0-plus |
| `gpl-2.0_complex.txt` | 2 | 1 | 1 gpl-2.0 missing |
| `gpl_and_gpl_and_gpl_and_lgpl-2.0_and_other.txt` | 5 | 7 | 2 warranty, 1 unknown |

---

## Category 5: Wrong Detection (6 tests)

**Root Cause**: Completely different license expression detected.

| Test | Expected | Actual |
|------|----------|--------|
| `cpl-1.0_in_html.html` | `["cpl-1.0"]` | `["unknown-license-reference"]` |
| `fsf-unlimited_not_fsf-unlimited-no-warranty.txt` | `["fsf-unlimited"]` | `["fsfullrsd"]` |
| `gpl-2.0-plus_1.txt` | `["gpl-2.0-plus"]` | `["gpl-1.0-plus", "gpl-2.0-plus"]` |
| `gpl-2.0-plus_4.txt` | `["gpl-2.0-plus AND free-unknown"]` | `[..., "gpl-2.0-plus"]` |
| `gpl-2.0-plus_41.txt` | `["gpl-2.0-plus AND free-unknown"]` | `[..., "gpl-2.0-plus"]` |
| `gpl_and_lgpl_and_gfdl-1.2.txt` | `["gpl-1.0-plus AND lgpl-2.0-plus AND gfdl-1.2"]` | `["gpl-1.0-plus", "lgpl-2.0-plus", "gfdl-1.2"]` |

---

## Cluster Analysis

### High-Impact Clusters

1. **CDDL (8 tests)** - Single root cause, blocked on Python parity
2. **Duplicate Merging (16 tests)** - Likely single root cause in match merging logic
3. **GFDL Extra Detections (4 tests)** - Pattern: gfdl-1.1 and gfdl-1.3 rules matching too broadly

### Medium-Impact Clusters

4. **godot tests (2 tests)** - Large COPYRIGHT files with many missing detections
5. **gpl-2.0-plus tests (6 tests)** - Various expression handling issues

### Low-Impact (Individual Investigation Needed)

6. **fsf-free tests (3 tests)** - Duplicate merging pattern
7. **Remaining tests (7 tests)** - No clear pattern

---

## Recommended Investigation Order

### Phase 1: High-Impact Single Root Cause
1. Duplicate merging (16 tests) - likely same fix for all
2. GFDL extra detections (4 tests) - rule matching issue

### Phase 2: Medium-Impact  
3. godot COPYRIGHT files (2 tests)
4. gpl-2.0-plus expression handling (6 tests)

### Phase 3: Individual Investigation
5. Remaining tests without clear pattern

---

## Files to Investigate

| Category | Key Files |
|----------|-----------|
| Duplicate Merging | `src/license_detection/match_refine.rs` - merge/contain logic |
| Extra Detections | `src/license_detection/match_refine.rs` - filter functions |
| Missing Detections | `src/license_detection/detection.rs` - detection creation |
| Wrong Detection | Rule matching, tokenization |
