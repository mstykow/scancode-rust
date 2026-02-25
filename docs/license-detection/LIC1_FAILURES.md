# Lic1 Golden Test Failures Analysis

## Summary

50 lic1 tests are failing. This document analyzes 20 representative failures.

---

## Failure Categories

| Category | Count | Description |
|----------|-------|-------------|
| CDDL Rule Selection | 8 | Wrong CDDL version (1.0 vs 1.1) selected |
| Extra Detections | 5 | Additional unexpected license expressions |
| Missing Detections | 4 | Fewer expressions than expected |
| Wrong Expression | 2 | Correct count but wrong license |
| Unknown License Reference | 1 | Returns unknown-license-reference instead of correct license |

---

## Detailed Failures

### 1. CDDL Rule Selection Issues (8 tests)

These are blocked by the stricter-surround-merge improvement (deferred until Python parity).

| Test | Expected | Actual | Issue |
|------|----------|--------|-------|
| `cddl-1.0.txt` | `["cddl-1.0"]` | `["unknown-license-reference", "cddl-1.0"]` | Extra detection |
| `cddl-1.0_or_gpl-2.0-classpath_and_apache-2.0-glassfish_1.txt` | `["(cddl-1.0 OR gpl-2.0 WITH classpath-exception-2.0) AND apache-2.0"]` | `["(cddl-1.1 OR ...) AND apache-2.0"]` | Wrong CDDL version |
| `cddl-1.0_or_gpl-2.0-classpath_and_apache-2.0-glassfish_2.txt` | `["(cddl-1.0 OR ...) AND apache-2.0"]` | `["(cddl-1.1 OR ...) AND apache-2.0"]` | Wrong CDDL version |
| `cddl-1.0_or_gpl-2.0-glassfish.txt` | `["cddl-1.0 OR gpl-2.0"]` | `["cddl-1.1 OR gpl-2.0 WITH classpath-exception-2.0"]` | Wrong CDDL version |
| `cddl-1.1.txt` | `["cddl-1.0"]` | `[]` | Missing detection |
| `cddl-1.1_or_gpl-2.0-classpath_and_apache-2.0-glassfish_1.txt` | `["(cddl-1.1 OR ...) AND apache-2.0"]` | `[...cddl-1.1..., ...cddl-1.0...]` | Extra CDDL 1.0 detection |
| `cddl-1.1_or_gpl-2.0-classpath_and_mit-glassfish.txt` | `["(cddl-1.1 OR ...) AND mit"]` | `[...apache-2.0..., ...mit...]` | Extra apache-2.0 detection |
| `cddl-1.1_or_gpl-2.0-classpath_and_w3c-glassfish.txt` | `["(...w3c)"]` | `[...w3c, ...w3c]` | Duplicate detection |

**Root Cause**: Surround merge not checking overlap before combining matches (see `improvements/stricter-surround-merge.md`).

---

### 2. Extra Detections (5 tests)

| Test | Expected | Actual | Issue |
|------|----------|--------|-------|
| `com-oreilly-servlet.txt` | `["com-oreilly-servlet"]` | `["com-oreilly-servlet", "com-oreilly-servlet"]` | Duplicate detection |
| `curl_2.txt` | `["curl"]` | `["unknown-license-reference", "curl"]` | Extra unknown detection |
| `cpl-1.0_in_html.html` | `["cpl-1.0"]` | `["unknown-license-reference"]` | Wrong detection |
| `complex.el` | 18 expressions | 19 expressions | Extra lgpl-2.0-plus at start |
| `epl-2.0.html` | `["epl-2.0", "epl-2.0"]` | `["epl-2.0", "unknown-license-reference", "unknown-license-reference", "proprietary-license"]` | Extra detections |

**Root Cause**: Unknown - needs investigation per test.

---

### 3. Missing Detections (4 tests)

| Test | Expected | Actual | Issue |
|------|----------|--------|-------|
| `cjdict-liconly.txt` | 8 expressions | 5 expressions | Missing 3 bsd-new detections |
| `e2fsprogs.txt` | 5 expressions | 4 expressions | Missing lgpl-2.1-plus |
| `eclipse-openj9.LICENSE` | 9 expressions | 8 expressions | Missing zlib |
| `eclipse-openj9_html.html` | 13 expressions | 11 expressions | Missing mit, zlib |

**Root Cause**: Unknown - needs investigation per test.

---

### 4. Wrong Expression (2 tests)

| Test | Expected | Actual | Issue |
|------|----------|--------|-------|
| `d-zlib_and_gfdl-1.2_and_gpl_and_gpl_and_other.txt` | Contains `gpl-1.0-plus` only | Contains extra `lgpl-2.1-plus` | Wrong expression |
| `ecos-license.html` | 2 expressions | 1 expression | Missing duplicate |

**Root Cause**: Unknown - needs investigation per test.

---

### 5. Merged Detections (1 test)

| Test | Expected | Actual | Issue |
|------|----------|--------|-------|
| `edl-1.0.txt` | `["bsd-new", "bsd-new"]` | `["bsd-new"]` | Duplicates merged |

**Root Cause**: Related to PLAN-058 (duplicate merge issue).

---

## Action Items

### High Priority (Blocking CDDL)

1. **Stricter surround merge** - Implement overlap check (deferred until Python parity)

### Medium Priority (Investigation Needed)

2. **Extra unknown-license-reference detections** - Investigate curl_2.txt, cpl-1.0_in_html.html, epl-2.0.html
3. **Missing detections** - Investigate cjdict-liconly.txt, e2fsprogs.txt, eclipse-openj9 tests
4. **Duplicate merging** - Investigate edl-1.0.txt (may be fixed by PLAN-058 preprocessing)

### Low Priority

5. **Complex.el extra detection** - Single extra expression, minor issue
6. **D-zlib wrong expression** - Minor expression difference

---

## Individual Test Plans Needed

| Test | Priority | Status |
|------|----------|--------|
| `cddl-*.txt` (8 tests) | HIGH | Blocked by stricter-surround-merge |
| `curl_2.txt` | MEDIUM | Needs investigation |
| `cpl-1.0_in_html.html` | MEDIUM | Needs investigation |
| `epl-2.0.html` | MEDIUM | Needs investigation |
| `cjdict-liconly.txt` | MEDIUM | Needs investigation |
| `e2fsprogs.txt` | MEDIUM | Needs investigation |
| `eclipse-openj9.LICENSE` | MEDIUM | Needs investigation |
| `eclipse-openj9_html.html` | MEDIUM | Needs investigation |
| `edl-1.0.txt` | MEDIUM | Related to PLAN-058 |
| `com-oreilly-servlet.txt` | LOW | Needs investigation |
| `complex.el` | LOW | Minor issue |
| `d-zlib_and_gfdl-1.2_and_gpl_and_gpl_and_other.txt` | LOW | Minor issue |
| `ecos-license.html` | LOW | Needs investigation |
