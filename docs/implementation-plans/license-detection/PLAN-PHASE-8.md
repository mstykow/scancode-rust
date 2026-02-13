# Phase 8: Comprehensive Testing and Validation

> **Status**: Not Started
> **Priority**: P0 — Critical Core Feature
> **Depends On**: Phases 0-7 (Complete license detection implementation)
> **Last Updated**: 2026-02-13

## Overview

Phase 8 ensures the license detection engine produces **identical output** to Python ScanCode across a wide range of inputs. This phase establishes confidence in correctness through golden tests, strategy-specific unit tests, and performance benchmarking.

## Goals

1. **Golden Test Suite**: Validate against Python ScanCode reference outputs
2. **Per-Strategy Tests**: Verify each matching strategy works correctly in isolation
3. **Performance Testing**: Benchmark and validate acceptable performance

---

## 8.1 Golden Test Suite Setup

### 8.1.1 Create Test Infrastructure

**What**: Set up the testing framework for license detection golden tests.

**Files to Create**:

- `src/license_detection_test.rs` — Unit tests for license detection components
- `src/license_detection_golden_test.rs` — Golden tests comparing against Python reference

**Implementation**:

```rust
#[cfg(test)]
mod golden_tests {
    use crate::license_detection::LicenseDetector;
    use crate::models::LicenseDetection;
    use std::path::PathBuf;

    fn compare_license_detections(
        actual: &[LicenseDetection],
        expected_path: &Path,
    ) -> Result<(), String> {
        let expected_content = fs::read_to_string(expected_path)?;
        let expected: Value = serde_json::from_str(&expected_content)?;
        let expected_detections = expected.get("license_detections");
        // Compare fields, ignoring dynamic values (identifiers, etc.)
    }
}
```

**Verification**:

- Run `cargo test license_detection_golden` — should compile and find test files
- Comparison helper functions work correctly

**Dependencies**: None (infrastructure setup)

---

### 8.1.2 Generate Python Reference Outputs

**What**: Create reference JSON outputs from Python ScanCode for test files.

**Source Test Data** (from `reference/scancode-toolkit/tests/licensedcode/data/`):

| Category | Source Directory | Test Files |
|----------|-----------------|------------|
| Single licenses | `detect/simple_detection/` | MIT, Apache-2.0, GPL variants |
| Multi-license | `plugin_license/scan/` | `ffmpeg-LICENSE.md` |
| SPDX-LID | `match_spdx/scan/` | SPDX identifier headers |
| Hash match | `hash/query.txt` | Exact whole-file matches |
| Sequence match | `match_seq/query/` | Modified/partial licenses |
| Unknown licenses | `match_unknown/` | Unknown license text |
| False positives | `false_positive/` | `false-positive-gpl3.txt` |
| License references | `plugin_license/license_reference/` | See COPYING, etc. |
| License intros | `plugin_license/unknown_intro/` | License introduction text |
| Truncated licenses | `detect/truncated/` | Partial license text |
| Overlapping matches | `detect/overlap/` | Multiple overlapping rules |

**Procedure**:

```bash
# For each test file, generate reference output
cd reference/scancode-toolkit

# Example: ffmpeg LICENSE
scancode --license --license-text \
    tests/licensedcode/data/plugin_license/scan/ffmpeg-LICENSE.md \
    --json tests/licensedcode/data/plugin_license/scan/ffmpeg-license.expected.json

# Example: SPDX identifiers
scancode --license \
    tests/licensedcode/data/match_spdx/scan/license \
    --json tests/licensedcode/data/match_spdx/scan-expected.json
```

**Target Directory Structure**:

```text
testdata/license-golden/
├── single-license/
│   ├── mit.txt
│   ├── mit.txt.expected
│   ├── apache-2.0.txt
│   ├── apache-2.0.txt.expected
│   └── ...
├── multi-license/
│   ├── ffmpeg-LICENSE.md
│   ├── ffmpeg-LICENSE.md.expected
│   └── ...
├── spdx-lid/
│   ├── license
│   ├── license.expected
│   └── ...
├── hash-match/
├── seq-match/
├── unknown/
├── false-positive/
└── reference/
    └── see-copying.txt
```

**Verification**:

- All `.expected` files are valid JSON
- Each expected file contains `license_detections` array
- Field structure matches `LicenseDetection` struct

**Dependencies**: 8.1.1 (test infrastructure)

---

### 8.1.3 Implement Comparison Logic

**What**: Create robust JSON comparison that handles intentional differences.

**Fields to Compare** (must match exactly):

- `license_expression` — ScanCode license key
- `license_expression_spdx` — SPDX identifier
- `detection_log` — Warning/info messages
- `matches[].license_expression`
- `matches[].matcher` — Strategy identifier (`1-hash`, `1-spdx-id`, `2-aho`, `3-seq`, `4-unknown`)
- `matches[].score` — Match score (0.0-100.0)
- `matches[].match_coverage` — Coverage percentage
- `matches[].rule_relevance` — Rule relevance (0-100)

**Fields to Skip/Normalize** (dynamic or implementation-specific):

- `identifier` — UUID, random per run
- `matched_text` — May differ in whitespace normalization
- `matched_text_diagnostics` — Diagnostic text
- `start_line` / `end_line` — May differ slightly (±1)
- `rule_identifier` — Hash suffix may differ
- `rule_url` — URL format may differ
- `from_file` — Path normalization

**Files to Modify**:

- `src/test_utils.rs` — Add `compare_license_detections()`

**Implementation**:

```rust
#[cfg(test)]
pub fn compare_license_detections(
    actual: &[LicenseDetection],
    expected_path: &Path,
) -> Result<(), String> {
    const SKIP_FIELDS: &[&str] = &[
        "identifier",
        "matched_text",
        "matched_text_diagnostics",
        "rule_identifier",
        "rule_url",
        "from_file",
    ];

    const NORMALIZE_FIELDS: &[&str] = &[
        "start_line",  // Allow ±1 difference
        "end_line",    // Allow ±1 difference
    ];

    // Compare license_expression, license_expression_spdx, matcher, score, etc.
}
```

**Verification**:

- Comparison correctly identifies matching outputs
- Comparison correctly identifies mismatching outputs with clear diff
- Tests pass on known-good data

**Dependencies**: 8.1.1, 8.1.2

---

### 8.1.4 Create Golden Test Cases

**What**: Implement individual golden tests organized by category.

**Test Cases** (in priority order):

#### Priority 1: Core Detection

| Test Name | Input File | Expected Behavior |
|-----------|------------|-------------------|
| `test_golden_single_mit` | Single MIT license text | Detect `mit` expression |
| `test_golden_single_apache` | Single Apache-2.0 text | Detect `apache-2.0` |
| `test_golden_single_gpl` | Single GPL-2.0 text | Detect `gpl-2.0` |
| `test_golden_spdx_id` | File with `SPDX-License-Identifier: MIT` | Detect via `1-spdx-id` matcher |
| `test_golden_hash_match` | Exact license file (hash match) | Detect via `1-hash` matcher |

#### Priority 2: Multi-License

| Test Name | Input File | Expected Behavior |
|-----------|------------|-------------------|
| `test_golden_ffmpeg` | `ffmpeg-LICENSE.md` | Multiple detections with different expressions |
| `test_golden_dual_license` | Dual-licensed file | Detect `mit OR apache-2.0` |
| `test_golden_license_stack` | Multiple licenses in one file | Detect all licenses |

#### Priority 3: Edge Cases

| Test Name | Input File | Expected Behavior |
|-----------|------------|-------------------|
| `test_golden_truncated` | Partial license text | Detect with `3-seq` matcher |
| `test_golden_false_positive` | `false-positive-gpl3.txt` | No false detections |
| `test_golden_unknown` | Unknown license text | Detect as `unknown-license-reference` |
| `test_golden_reference` | "See COPYING" text | Detect as license reference |
| `test_golden_intro` | License introduction | Detect with intro flag |
| `test_golden_no_license` | File without licenses | Empty detections array |

**Files to Create**:

- `src/license_detection_golden_test.rs`

**Implementation Pattern**:

```rust
#[test]
fn test_golden_single_mit() {
    let input = PathBuf::from("testdata/license-golden/single-license/mit.txt");
    let expected = PathBuf::from("testdata/license-golden/single-license/mit.txt.expected");

    let detections = LicenseDetector::detect_licenses(&input);
    compare_license_detections(&detections, &expected).unwrap();
}

#[test]
fn test_golden_ffmpeg() {
    let input = PathBuf::from("testdata/license-golden/multi-license/ffmpeg-LICENSE.md");
    let expected = PathBuf::from("testdata/license-golden/multi-license/ffmpeg-LICENSE.md.expected");

    let detections = LicenseDetector::detect_licenses(&input);
    compare_license_detections(&detections, &expected).unwrap();
}
```

**Verification**:

- All tests compile and run
- Each test validates against expected output
- Clear failure messages on mismatch

**Dependencies**: 8.1.1, 8.1.2, 8.1.3

---

### 8.1.5 Document Intentional Differences

**What**: Document any legitimate behavioral differences from Python.

**File to Create/Modify**:

- `docs/improvements/license-detection-differences.md`

**Categories of Differences**:

1. **Bug Fixes**: Python bugs we intentionally don't replicate
2. **Performance Optimizations**: Same output, faster execution
3. **Architectural Differences**: Different internal structure, same output
4. **Beyond Parity**: Features Python lacks

**Format**:

```markdown
# License Detection: Intentional Differences from Python ScanCode

## Bug Fixes

### Issue: [Description]
**Python Behavior**: [What Python does wrong]
**Rust Behavior**: [What we do correctly]
**Reference**: [Link to Python issue/commit]
**Test**: [Test that demonstrates the fix]

## Performance Optimizations

### [Optimization name]
**Description**: [What was optimized]
**Impact**: [Performance improvement]
**Output Equivalence**: [Proof that output is identical]

## Beyond Parity

### [Feature name]
**Description**: [Feature Python lacks]
**Implementation**: [How we implemented it]
**Test**: [Test demonstrating the feature]
```

**Verification**:

- All intentional differences documented
- Each difference has corresponding test
- Documentation reviewed for accuracy

**Dependencies**: 8.1.4 (all golden tests)

---

## 8.2 Per-Strategy Tests

### 8.2.1 Hash Match Strategy Tests

**What**: Test the `1-hash` matcher for exact whole-file license matching.

**Test Files** (from `reference/scancode-toolkit/tests/licensedcode/data/hash/`):

- `query.txt` — Input text
- `rules/` — Hash-indexed license texts

**Test Cases**:

| Test Name | Input | Expected |
|-----------|-------|----------|
| `test_hash_exact_mit` | Exact MIT license text | `mit`, matcher=`1-hash`, score=100 |
| `test_hash_exact_apache` | Exact Apache-2.0 text | `apache-2.0`, matcher=`1-hash` |
| `test_hash_no_match` | Modified license text | No hash match (falls through) |
| `test_hash_normalized` | Normalized whitespace | Matches after normalization |

**Files to Create**:

- `src/license_detection_test.rs` (add hash tests)

**Implementation**:

```rust
#[test]
fn test_hash_exact_mit() {
    let mit_text = include_str!("../testdata/license-golden/hash/mit-exact.txt");
    let result = HashMatcher::match_hash(mit_text);
    assert!(result.is_some());
    let match = result.unwrap();
    assert_eq!(match.license_expression, "mit");
    assert_eq!(match.matcher, "1-hash");
    assert_eq!(match.score, 100.0);
}
```

**Verification**:

- All hash tests pass
- Hash algorithm produces same results as Python (MD5 or SHA256)
- Normalization matches Python behavior

**Dependencies**: Phase 4 (hash matching implementation)

---

### 8.2.2 SPDX-License-Identifier Tests

**What**: Test the `1-spdx-id` matcher for SPDX identifier headers.

**Test Files** (from `reference/scancode-toolkit/tests/licensedcode/data/match_spdx/`):

- `scan/license` — Example file with SPDX identifiers
- `lines/*.txt` — Various SPDX format variations

**Test Cases**:

| Test Name | Input | Expected |
|-----------|-------|----------|
| `test_spdx_simple` | `SPDX-License-Identifier: MIT` | `mit`, matcher=`1-spdx-id` |
| `test_spdx_with_plus` | `SPDX-License-Identifier: GPL-2.0+` | `gpl-2.0-plus` |
| `test_spdx_with_or` | `SPDX-License-Identifier: MIT OR Apache-2.0` | `mit OR apache-2.0` |
| `test_spdx_with_and` | `SPDX-License-Identifier: GPL-2.0 AND LGPL-2.1` | Combined expression |
| `test_spdx_multiple` | Multiple SPDX lines | Multiple detections |
| `test_spdx_v3_format` | SPDX 2.2+ format with `()` | Complex expression |
| `test_spdx_in_comment` | `// SPDX-License-Identifier: MIT` | Detects in comment |
| `test_spdx_with_exception` | `SPDX-License-Identifier: GPL-2.0 WITH Classpath-exception-2.0` | Expression with exception |

**Files to Create**:

- `src/license_detection_test.rs` (add SPDX tests)

**Implementation**:

```rust
#[test]
fn test_spdx_simple() {
    let text = "SPDX-License-Identifier: MIT\nSome code here";
    let result = SpdxLidMatcher::match_spdx_lid(text);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].license_expression, "mit");
    assert_eq!(result[0].matcher, "1-spdx-id");
    assert_eq!(result[0].score, 100.0);
}

#[test]
fn test_spdx_with_or() {
    let text = "SPDX-License-Identifier: MIT OR Apache-2.0";
    let result = SpdxLidMatcher::match_spdx_lid(text);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].license_expression, "mit OR apache-2.0");
}
```

**Verification**:

- All SPDX tests pass
- Expression parsing matches Python's behavior
- Line positions correctly identified

**Dependencies**: Phase 4 (SPDX-LID implementation)

---

### 8.2.3 Aho-Corasick Exact Match Tests

**What**: Test the `2-aho` matcher for exact rule matches.

**Test Files** (from `reference/scancode-toolkit/tests/licensedcode/data/mach_aho/`):

- `rtos_exact/` — Exact match test cases

**Test Cases**:

| Test Name | Input | Expected |
|-----------|-------|----------|
| `test_aho_single_rule` | Text containing one rule pattern | Single match |
| `test_aho_multiple_rules` | Text matching multiple rules | Multiple matches |
| `test_aho_overlapping` | Overlapping rule matches | Best match selected |
| `test_aho_case_insensitive` | Case variations | Matches after normalization |
| `test_aho_with_templates` | Rule with `{template}` variables | Template expansion works |
| `test_aho_false_positive_filtered` | Text matching false-positive rule | Filtered out |

**Files to Create**:

- `src/license_detection_test.rs` (add Aho tests)

**Implementation**:

```rust
#[test]
fn test_aho_single_rule() {
    let text = "Licensed under the MIT License";
    let index = create_test_index_with_mit_rule();
    let result = AhoMatcher::match_aho(text, &index);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].license_expression, "mit");
    assert_eq!(result[0].matcher, "2-aho");
}

#[test]
fn test_aho_overlapping() {
    let text = "This is licensed under the MIT license which is similar to...";
    let result = AhoMatcher::match_aho(text, &full_index);
    // Verify overlapping matches are resolved correctly
    assert!(result.iter().all(|m| m.matcher == "2-aho"));
}
```

**Verification**:

- All Aho tests pass
- Automaton produces same matches as Python
- Overlap resolution matches Python logic

**Dependencies**: Phase 4 (Aho-Corasick implementation)

---

### 8.2.4 Sequence Alignment Tests

**What**: Test the `3-seq` matcher for approximate/partial license matching.

**Test Files** (from `reference/scancode-toolkit/tests/licensedcode/data/match_seq/`):

- `query/` — Input files for sequence matching
- `rules/` — Reference license texts

**Test Cases**:

| Test Name | Input | Expected |
|-----------|-------|----------|
| `test_seq_partial_license` | 80% of MIT license text | `mit` with score ~80 |
| `test_seq_modified_license` | MIT with extra words | `mit` with score < 100 |
| `test_seq_truncated_license` | First half of license | Detects with lower score |
| `test_seq_threshold` | Text below threshold (e.g., 60%) | No match or lower confidence |
| `test_seq_multiple_candidates` | Text similar to multiple licenses | Best match selected |
| `test_seq_coverage_calculation` | Various coverage levels | Correct coverage % |

**Files to Create**:

- `src/license_detection_test.rs` (add seq tests)

**Implementation**:

```rust
#[test]
fn test_seq_partial_license() {
    let partial_mit = include_str!("../testdata/license-golden/seq/mit-80percent.txt");
    let result = SeqMatcher::match_seq(partial_mit, &index);
    assert!(result.is_some());
    let m = result.unwrap();
    assert_eq!(m.license_expression, "mit");
    assert_eq!(m.matcher, "3-seq");
    assert!(m.score >= 75.0 && m.score <= 85.0);
}
```

**Verification**:

- All sequence tests pass
- Scores match Python within ±5%
- Coverage calculations are accurate

**Dependencies**: Phase 5 (sequence alignment implementation)

---

### 8.2.5 Unknown License Detection Tests

**What**: Test the `4-unknown` matcher for unrecognized license-like text.

**Test Files** (from `reference/scancode-toolkit/tests/licensedcode/data/match_unknown/`):

- `unknown.txt` — Unknown license text
- `unknown-license-expected.json` — Expected output

**Test Cases**:

| Test Name | Input | Expected |
|-----------|-------|----------|
| `test_unknown_proprietary` | Custom proprietary license | `unknown-license-reference` |
| `test_unknown_intro` | License introduction without full text | Unknown with intro flag |
| `test_unknown_with_clues` | Unknown text with license clues | Clues detected |
| `test_unknown_diagnostics` | Unknown text diagnostics | Diagnostic info populated |
| `test_unknown_not_license` | Non-license legal text | No detection or unknown |

**Files to Create**:

- `src/license_detection_test.rs` (add unknown tests)

**Implementation**:

```rust
#[test]
fn test_unknown_proprietary() {
    let text = "This software is proprietary and confidential...";
    let result = UnknownMatcher::detect_unknown(text);
    assert!(result.is_some());
    let m = result.unwrap();
    assert_eq!(m.license_expression, "unknown-license-reference");
    assert_eq!(m.matcher, "4-unknown");
}
```

**Verification**:

- Unknown detection matches Python behavior
- Clue detection works correctly
- Diagnostics are informative

**Dependencies**: Phase 5 (unknown detection implementation)

---

### 8.2.6 Match Merging and Detection Grouping Tests

**What**: Test the heuristics that combine matches into detections.

**Test Files** (from `reference/scancode-toolkit/tests/licensedcode/data/detect/`):

- `overlap/` — Overlapping match resolution
- `contained/` — Contained match handling
- `score/` — Score-based selection

**Test Cases**:

| Test Name | Input | Expected |
|-----------|-------|----------|
| `test_merge_adjacent_matches` | Adjacent matches for same license | Single detection |
| `test_resolve_overlap` | Overlapping matches for different licenses | Best match wins |
| `test_combine_expression` | Multiple licenses in file | Combined expression |
| `test_filter_false_positive` | Match with false-positive flag | Filtered out |
| `test_intro_plus_match` | Intro followed by full match | Combined detection |
| `test_clue_handling` | License clues separate from detections | Clues in separate array |

**Files to Create**:

- `src/license_detection_test.rs` (add grouping tests)

**Implementation**:

```rust
#[test]
fn test_merge_adjacent_matches() {
    let matches = vec![
        create_match("mit", 1, 10),
        create_match("mit", 11, 20),
    ];
    let detections = DetectionGrouping::group_matches(matches);
    assert_eq!(detections.len(), 1);
    assert_eq!(detections[0].license_expression, "mit");
}

#[test]
fn test_combine_expression() {
    let matches = vec![
        create_match("mit", 1, 10),
        create_match("apache-2.0", 15, 25),
    ];
    let detections = DetectionGrouping::group_matches(matches);
    assert_eq!(detections.len(), 1);
    assert!(detections[0].license_expression.contains("AND"));
}
```

**Verification**:

- Grouping heuristics match Python behavior
- Expression composition is correct
- False positives are filtered

**Dependencies**: Phase 6 (detection grouping implementation)

---

## 8.3 Performance Testing

### 8.3.1 Create Benchmark Infrastructure

**What**: Set up Rust benchmarking using `cargo bench` with criterion.

**Files to Create**:

- `benches/license_detection_bench.rs`

**Cargo.toml Modifications**:

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
tempfile = "3.25.0"

[[bench]]
name = "license_detection_bench"
harness = false
```

**Verification**:

- `cargo bench` compiles and runs
- Basic benchmark structure works

**Dependencies**: None

---

### 8.3.2 Index Construction Benchmarks

**What**: Measure time to build the license index from rules.

**Metrics to Collect**:

- Total index construction time
- Time per 1000 rules
- Memory usage during construction
- Aho-Corasick automaton build time
- Hash table population time

**Test Cases**:

| Benchmark | Description |
|-----------|-------------|
| `bench_index_full` | Build full index from all rules |
| `bench_index_subset` | Build index from 100 rules |
| `bench_aho_construction` | Build Aho-Corasick automaton |
| `bench_hash_table` | Populate hash table |

**Implementation**:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_index_construction(c: &mut Criterion) {
    c.bench_function("index_full", |b| {
        b.iter(|| {
            let index = LicenseIndex::from_path("resources/licenses/rules/");
            black_box(index)
        })
    });
}

criterion_group!(benches, bench_index_construction);
criterion_main!(benches);
```

**Target Performance**:

- Full index construction: < 5 seconds
- Aho-Corasick construction: < 2 seconds
- Memory usage: < 500 MB

**Verification**:

- Benchmarks run without errors
- Results are repeatable (±10%)
- HTML reports generated

**Dependencies**: 8.3.1, Phase 3 (index construction)

---

### 8.3.3 Per-File Detection Benchmarks

**What**: Measure detection time for various file types.

**Test Files**:

- Small files (< 1 KB): Single license text
- Medium files (1-100 KB): Source code with license headers
- Large files (> 1 MB): Large source files or concatenated licenses

**Metrics to Collect**:

- Detection time per file
- Time per strategy (hash, SPDX, Aho, seq)
- Memory usage during detection
- Throughput (files/second, MB/second)

**Test Cases**:

| Benchmark | File Size | Expected Time |
|-----------|-----------|---------------|
| `bench_small_file` | < 1 KB | < 1 ms |
| `bench_medium_file` | 10 KB | < 10 ms |
| `bench_large_file` | 1 MB | < 100 ms |
| `bench_spdx_only` | SPDX headers only | < 1 ms |
| `bench_complex_licenses` | Multiple overlapping licenses | < 50 ms |

**Implementation**:

```rust
fn bench_detection_small(c: &mut Criterion) {
    let index = LicenseIndex::from_path("resources/licenses/rules/").unwrap();
    let small_text = include_str!("../testdata/license-golden/single-license/mit.txt");

    c.bench_function("detect_small_file", |b| {
        b.iter(|| {
            let detections = LicenseDetector::detect(black_box(small_text), &index);
            black_box(detections)
        })
    });
}
```

**Target Performance**:

- Small files: < 1 ms
- Medium files: < 10 ms
- Large files: < 100 ms
- Throughput: > 10 MB/s

**Verification**:

- Benchmarks run without errors
- No memory leaks detected
- Performance meets targets

**Dependencies**: 8.3.1, 8.3.2, Phase 4-6 (detection pipeline)

---

### 8.3.4 Compare with Python Performance

**What**: Benchmark Python ScanCode on same test files for comparison.

**Procedure**:

```bash
# Time Python ScanCode on test files
cd reference/scancode-toolkit

time scancode --license testdata/license-golden/single-license/ --json /dev/null
time scancode --license testdata/license-golden/multi-license/ --json /dev/null
time scancode --license testdata/license-golden/large/ --json /dev/null
```

**Metrics to Collect**:

- Total scan time
- Files per second
- Memory usage (via `/usr/bin/time -v`)
- CPU utilization

**Comparison Table**:

| Metric | Python ScanCode | Rust (target) | Target Ratio |
|--------|-----------------|---------------|--------------|
| Small file detection | ~10 ms | < 1 ms | 10x faster |
| Medium file detection | ~100 ms | < 10 ms | 10x faster |
| Large file detection | ~1 s | < 100 ms | 10x faster |
| Index construction | ~10 s | < 5 s | 2x faster |
| Memory usage | ~1 GB | < 500 MB | 2x less |

**Files to Create**:

- `docs/performance/license-detection-benchmarks.md`

**Verification**:

- Rust implementation meets or exceeds targets
- Results documented with methodology

**Dependencies**: 8.3.2, 8.3.3

---

### 8.3.5 Memory Profiling

**What**: Profile memory usage to identify leaks and optimization opportunities.

**Tools**:

- `valgrind --tool=massif` for heap profiling
- `heaptrack` for allocation tracking
- Rust `#[global_allocator]` with custom tracker

**Test Scenarios**:

1. Index construction and retention
2. Single file detection (check for leaks)
3. Batch file processing (10,000 files)
4. Long-running process (repeated detection)

**Implementation**:

```rust
#[cfg(test)]
mod memory_tests {
    use std::alloc::{GlobalAlloc, System, Layout};

    #[global_allocator]
    static TRACKER: Tracker = Tracker;

    struct Tracker;

    unsafe impl GlobalAlloc for Tracker {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            // Track allocations
            System.alloc(layout)
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            System.dealloc(ptr, layout)
        }
    }
}
```

**Target Metrics**:

- No memory leaks (all allocations freed)
- Peak memory < 500 MB during normal operation
- Memory returns to baseline after detection

**Verification**:

- Run with valgrind/heaptrack
- No leaked bytes reported
- Memory profile documented

**Dependencies**: 8.3.3

---

### 8.3.6 Document Performance Results

**What**: Create comprehensive performance documentation.

**File to Create**:

- `docs/performance/license-detection-performance.md`

**Contents**:

1. Benchmark methodology
2. Hardware specifications used
3. Index construction results
4. Per-file detection results
5. Comparison with Python ScanCode
6. Memory profile analysis
7. Optimization recommendations

**Template**:

```markdown
# License Detection Performance Report

## Methodology
- Hardware: [CPU, RAM, SSD/HDD]
- Rust version: [version]
- Dataset: [number of rules, test files]

## Index Construction
| Metric | Value |
|--------|-------|
| Total rules loaded | X |
| Construction time | X s |
| Peak memory | X MB |

## Per-File Detection
| File Size | Avg Time | P50 | P95 | P99 |
|-----------|----------|-----|-----|-----|
| < 1 KB | X ms | ... | ... | ... |
| 1-10 KB | X ms | ... | ... | ... |
| 10-100 KB | X ms | ... | ... | ... |
| > 1 MB | X ms | ... | ... | ... |

## Comparison with Python
| Metric | Python | Rust | Improvement |
|--------|--------|------|-------------|
| ... | ... | ... | ... |
```

**Verification**:

- Documentation is complete and accurate
- All benchmark results included
- Recommendations are actionable

**Dependencies**: 8.3.2, 8.3.3, 8.3.4, 8.3.5

---

## Summary

### Task Dependencies

```text
8.1.1 (Infrastructure)
    ├── 8.1.2 (Reference Outputs)
    │       └── 8.1.3 (Comparison Logic)
    │               └── 8.1.4 (Golden Tests)
    │                       └── 8.1.5 (Document Differences)
    │
8.2.1 (Hash Tests) ────────────── Phase 4
8.2.2 (SPDX Tests) ────────────── Phase 4
8.2.3 (Aho Tests) ─────────────── Phase 4
8.2.4 (Seq Tests) ─────────────── Phase 5
8.2.5 (Unknown Tests) ─────────── Phase 5
8.2.6 (Grouping Tests) ────────── Phase 6
    │
8.3.1 (Benchmark Infrastructure)
    ├── 8.3.2 (Index Benchmarks) ───── Phase 3
    ├── 8.3.3 (Detection Benchmarks) ─ Phase 4-6
    │       ├── 8.3.4 (Python Comparison)
    │       └── 8.3.5 (Memory Profiling)
    │               └── 8.3.6 (Documentation)
```

### Files to Create

| File | Purpose |
|------|---------|
| `src/license_detection_test.rs` | Unit tests for strategies |
| `src/license_detection_golden_test.rs` | Golden tests vs Python reference |
| `benches/license_detection_bench.rs` | Performance benchmarks |
| `testdata/license-golden/**` | Test input files and expected outputs |
| `docs/improvements/license-detection-differences.md` | Documented differences |
| `docs/performance/license-detection-performance.md` | Performance results |

### Success Criteria

- [ ] All golden tests pass against Python reference outputs
- [ ] Per-strategy tests cover each matcher comprehensively
- [ ] Performance meets targets (10x faster than Python for small files)
- [ ] No memory leaks detected
- [ ] All intentional differences documented
- [ ] `cargo test --all` passes with no failures
- [ ] `cargo bench` produces valid benchmark results

### Estimated Effort

| Section | Estimated Time |
|---------|---------------|
| 8.1 Golden Test Suite | 3-4 days |
| 8.2 Per-Strategy Tests | 2-3 days |
| 8.3 Performance Testing | 1-2 days |
| **Total** | **6-9 days** |
