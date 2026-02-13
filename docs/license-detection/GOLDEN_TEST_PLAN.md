# License Detection Golden Test Plan

## Overview

This document describes how to integrate Python ScanCode's license detection tests into scancode-rust.

## Python Test Structure Analysis

### Data-Driven Tests

Python scancode uses a data-driven testing approach with YAML expectation files:

```text
tests/licensedcode/data/datadriven/
├── lic1/          # 291 test pairs (582 files)
├── lic2/          # 340 test pairs (679 files)
├── lic3/          # 292 test pairs (584 files)
├── lic4/          # 345 test pairs (689 files)
├── external/      # External license references
├── unknown/       # Unknown license detection
└── unknown_about/ # ABOUT file tests
```text

**Total: ~1,268 test cases across all directories**

### Test File Format

Each test consists of:

1. **Source file**: The file to scan (e.g., `mit.c`, `apache.txt`)
2. **YAML expectation file**: Same name with `.yml` extension

**Example YAML (`mit.c.yml`):**

```yaml
license_expressions:
  - mit
```text

**Example with multiple licenses:**

```yaml
license_expressions:
  - apache-2.0
  - gpl-2.0
```text

### Test Execution (Python)

The `build_tests()` function in `licensedcode_test_utils.py`:

1. Scans test directory for `.yml`/file pairs
2. Dynamically creates test methods at module import time
3. Each test runs detection and compares `license_expression` results
4. On failure, outputs detailed diff with matched text

### Unit Tests

Additional focused tests in `tests/licensedcode/`:

- `test_detect.py` - Core detection mechanics
- `test_match.py` - Match scoring and combining
- `test_match_aho.py` - Aho-Corasick matcher
- `test_match_hash.py` - Hash matcher
- `test_match_seq.py` - Sequence matcher
- `test_match_spdx_lid.py` - SPDX-License-Identifier detection

## Proposed Rust Integration

### Phase 1: Core Infrastructure (Week 1)

#### 1.1 Create Test Data Directory Structure

```text
testdata/license-golden/
├── datadriven/
│   ├── lic1/           # Symlink or copy from reference
│   ├── lic2/
│   ├── lic3/
│   ├── lic4/
│   ├── external/
│   └── unknown/
├── detect/             # Unit test scenarios
└── README.md
```text

#### 1.2 Implement Data-Driven Test Framework

Create `src/license_detection_golden_test.rs` with:

```rust
/// Represents a single data-driven license test
struct LicenseGoldenTest {
    test_file: PathBuf,
    expected_expressions: Vec<String>,
    notes: Option<String>,
    expected_failure: bool,
}

impl LicenseGoldenTest {
    fn load_from_yaml(yaml_path: &Path) -> Result<Self, Error> {
        // Parse YAML file
        // Load expected_expressions
        // Handle notes and expected_failure
    }
    
    fn run(&self, engine: &LicenseDetectionEngine) -> Result<(), String> {
        // Read test file
        // Run detection
        // Compare results
    }
}
```text

#### 1.3 Test Discovery and Execution

```rust
/// Discover all golden tests in a directory
fn discover_golden_tests(dir: &Path) -> Vec<LicenseGoldenTest> {
    // Find all .yml files
    // Load each test
}

#[test]
fn test_golden_lic1() {
    let tests = discover_golden_tests("testdata/license-golden/datadriven/lic1");
    let engine = create_test_engine();
    
    for test in tests {
        if let Err(e) = test.run(&engine) {
            panic!("Test {:?} failed: {}", test.test_file, e);
        }
    }
}
```text

### Phase 2: Test Data Migration (Week 2)

#### 2.1 Option A: Symlink Reference Tests (Recommended)

```bash
# Create symlinks to reference test data
ln -s ../../../reference/scancode-toolkit/tests/licensedcode/data/datadriven/lic1 \
      testdata/license-golden/datadriven/lic1
```text

**Pros:**

- No duplication
- Automatically stays in sync with Python updates
- Smaller repo size

**Cons:**

- Requires reference submodule to be initialized
- May break if Python test structure changes

#### 2.2 Option B: Copy Selected Tests

```bash
# Copy a curated subset of tests
cp -r reference/scancode-toolkit/tests/licensedcode/data/datadriven/lic1/*.c \
      testdata/license-golden/datadriven/lic1/
```text

**Pros:**

- Full control over test selection
- No external dependencies

**Cons:**

- Must manually sync updates
- Larger repo size

#### 2.3 Recommendation: Hybrid Approach

1. **Symlink** for the main datadriven tests (lic1-lic4)
2. **Copy** specific unit test scenarios from `detect/` directory

### Phase 3: Comprehensive Coverage (Week 3)

#### 3.1 Test Categories to Cover

| Category | Python Location | Priority | Count |
|----------|----------------|----------|-------|
| Single licenses | lic1-lic4 | High | ~400 |
| Multi-license | lic1-lic4 | High | ~200 |
| SPDX-LID | lic1-lic4 | High | ~150 |
| Hash match | detect/mit | High | ~20 |
| Sequence match | detect/truncated | Medium | ~30 |
| Unknown licenses | unknown/ | Medium | ~10 |
| External refs | external/ | Medium | ~5 |
| Edge cases | detect/ | Low | ~50 |

#### 3.2 Test Implementation Strategy

```rust
#[test]
fn test_golden_single_licenses() {
    // Test files that should detect exactly one license
    run_golden_tests("testdata/license-golden/datadriven/lic1", |t| {
        t.expected_expressions.len() == 1
    });
}

#[test]
fn test_golden_multi_licenses() {
    // Test files with multiple license detections
    run_golden_tests("testdata/license-golden/datadriven/lic1", |t| {
        t.expected_expressions.len() > 1
    });
}

#[test]
fn test_golden_spdx_lid() {
    // Test SPDX-License-Identifier detection
    run_golden_tests("testdata/license-golden/datadriven/lic1", |t| {
        t.test_file.extension() == Some("rs".as_ref()) ||
        t.test_file.extension() == Some("py".as_ref())
    });
}
```text

### Phase 4: Continuous Integration (Week 4)

#### 4.1 CI Test Matrix

```yaml
# .github/workflows/license-tests.yml
- name: Run License Golden Tests
  run: cargo test golden --release
  
- name: Generate Test Report
  run: cargo test golden -- --format json > test-results.json
```text

#### 4.2 Expected Failure Handling

Some tests may fail initially. Use Rust's `#[ignore]` attribute:

```rust
#[test]
#[ignore = "Known difference in expression combination logic"]
fn test_golden_complex_expression() {
    // Test that differs from Python behavior
}
```text

## Implementation Steps

### Step 1: Create Test Infrastructure

1. Update `src/license_detection_golden_test.rs` with data-driven framework
2. Add YAML parsing dependency (`serde_yaml`)
3. Create test discovery functions

### Step 2: Set Up Test Data

```bash
# Create directory structure
mkdir -p testdata/license-golden/datadriven

# Symlink reference tests
cd testdata/license-golden/datadriven
ln -s ../../../../reference/scancode-toolkit/tests/licensedcode/data/datadriven/lic1 lic1
ln -s ../../../../reference/scancode-toolkit/tests/licensedcode/data/datadriven/lic2 lic2
ln -s ../../../../reference/scancode-toolkit/tests/licensedcode/data/datadriven/lic3 lic3
ln -s ../../../../reference/scancode-toolkit/tests/licensedcode/data/datadriven/lic4 lic4
ln -s ../../../../reference/scancode-toolkit/tests/licensedcode/data/datadriven/external external
ln -s ../../../../reference/scancode-toolkit/tests/licensedcode/data/datadriven/unknown unknown
```text

### Step 3: Implement Core Tests

```rust
// src/license_detection_golden_test.rs

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct LicenseTestYaml {
    license_expressions: Vec<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    expected_failure: bool,
    #[serde(default = "default_language")]
    language: String,
}

fn default_language() -> String { "en".to_string() }

struct LicenseGoldenTest {
    name: String,
    test_file: PathBuf,
    yaml: LicenseTestYaml,
}

impl LicenseGoldenTest {
    fn load(dir: &Path, yaml_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(yaml_path)?;
        let yaml: LicenseTestYaml = serde_yaml::from_str(&content)?;
        
        // Find corresponding test file (same name without .yml)
        let test_file = yaml_path.with_extension("");
        let name = test_file.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        Ok(Self { name, test_file, yaml })
    }
    
    fn run(&self, engine: &LicenseDetectionEngine) -> Result<(), String> {
        let text = std::fs::read_to_string(&self.test_file)
            .map_err(|e| format!("Failed to read {}: {}", self.test_file.display(), e))?;
        
        let detections = engine.detect(&text)
            .map_err(|e| format!("Detection failed: {:?}", e))?;
        
        let actual: Vec<&str> = detections.iter()
            .map(|d| d.license_expression.as_deref().unwrap_or(""))
            .collect();
        
        let expected: Vec<&str> = self.yaml.license_expressions.iter()
            .map(|s| s.as_str())
            .collect();
        
        if actual != expected {
            return Err(format!(
                "Expression mismatch for {}:\n  Expected: {:?}\n  Actual:   {:?}",
                self.name, expected, actual
            ));
        }
        
        Ok(())
    }
}

fn discover_tests(dir: &Path) -> Vec<LicenseGoldenTest> {
    let mut tests = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "yml").unwrap_or(false) {
                if let Ok(test) = LicenseGoldenTest::load(dir, &path) {
                    tests.push(test);
                }
            }
        }
    }
    
    tests.sort_by(|a, b| a.name.cmp(&b.name));
    tests
}

fn run_golden_suite(suite_name: &str, dir: &Path) {
    if skip_if_no_reference_data() {
        eprintln!("Skipping {}: reference data not available", suite_name);
        return;
    }
    
    let Some(engine) = create_test_engine() else {
        panic!("Failed to create license detection engine");
    };
    
    let tests = discover_tests(dir);
    let mut failures = Vec::new();
    
    for test in &tests {
        if test.yaml.expected_failure {
            continue; // Skip known failures
        }
        
        if let Err(e) = test.run(&engine) {
            failures.push((test.name.clone(), e));
        }
    }
    
    if !failures.is_empty() {
        panic!(
            "{} test failures in {}:\n{}",
            failures.len(),
            suite_name,
            failures.iter()
                .map(|(n, e)| format!("  {}: {}", n, e))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

#[test]
fn test_golden_lic1() {
    run_golden_suite(
        "lic1",
        &PathBuf::from("testdata/license-golden/datadriven/lic1")
    );
}

#[test]
fn test_golden_lic2() {
    run_golden_suite(
        "lic2",
        &PathBuf::from("testdata/license-golden/datadriven/lic2")
    );
}

#[test]
fn test_golden_lic3() {
    run_golden_suite(
        "lic3",
        &PathBuf::from("testdata/license-golden/datadriven/lic3")
    );
}

#[test]
fn test_golden_lic4() {
    run_golden_suite(
        "lic4",
        &PathBuf::from("testdata/license-golden/datadriven/lic4")
    );
}
```text

### Step 4: Add serde_yaml Dependency

```toml
# Cargo.toml
[dev-dependencies]
serde_yaml = "0.9"
```text

### Step 5: Create Symlinks

```bash
cd testdata/license-golden/datadriven
ln -s ../../../../reference/scancode-toolkit/tests/licensedcode/data/datadriven/lic1 lic1
ln -s ../../../../reference/scancode-toolkit/tests/licensedcode/data/datadriven/lic2 lic2
ln -s ../../../../reference/scancode-toolkit/tests/licensedcode/data/datadriven/lic3 lic3
ln -s ../../../../reference/scancode-toolkit/tests/licensedcode/data/datadriven/lic4 lic4
ln -s ../../../../reference/scancode-toolkit/tests/licensedcode/data/datadriven/external external
ln -s ../../../../reference/scancode-toolkit/tests/licensedcode/data/datadriven/unknown unknown
```text

## Success Criteria

1. **Coverage**: All 1,268 datadriven tests pass or are documented as expected failures
2. **Speed**: Full golden test suite runs in under 60 seconds
3. **Reporting**: Clear failure messages showing expected vs actual
4. **CI Integration**: Tests run on every PR
5. **Documentation**: Each expected failure documented with reason

## Next Steps

1. Implement Step 1-5 above
2. Run tests and identify failures
3. Debug and fix detection issues
4. Document intentional differences from Python behavior

## Related Documentation

- [TESTING_STRATEGY.md](../TESTING_STRATEGY.md) - Overall testing approach
- [ADR 0003](../adr/0003-golden-test-strategy.md) - Golden test philosophy
- [COMPARISON.md](../../COMPARISON.md) - Python vs Rust comparison results
