# PLAN-077: Unknown Detection - citrix.txt

## Status: DETAILED IMPLEMENTATION PLAN READY

## Problem Statement

**File**: `testdata/license-golden/datadriven/unknown/citrix.txt`

| Expected | Actual |
|----------|--------|
| 5 detections (Python default) | 15 detections (Rust) |

**Issue**: Rust always runs unknown license matching, but Python only runs it when `--unknown-licenses` flag is provided.

## Root Cause Analysis

### Python Behavior

1. **Default behavior** (`unknown_licenses=False`): Python does NOT run `match_unknowns()` at all
   - Returns only Aho-Corasick matches (matcher="2-aho")
   - citrix.txt produces **11 matches** with default settings
   - These become **5 license_detections** after grouping

2. **With `--unknown-licenses` flag** (`unknown_licenses=True`): Python runs `match_unknowns()`
   - Returns Aho matches + unknown matches (matcher="6-unknown")
   - citrix.txt produces **9 matches** total (7 aho + 2 unknown)
   - These become **4 license_detections** after grouping

3. **Key Python code path** (index.py:1082-1116):
   ```python
   if unknown_licenses:  # <-- THIS IS FALSE BY DEFAULT!
       good_matches, weak_matches = match.split_weak_matches(matches)
       # ... compute unmatched regions ...
       for unspan in unmatched_qspan.subspans():
           unknown_match = match_unknown.match_unknowns(...)
   ```

### Rust Behavior

1. **Always runs unknown matching** (mod.rs:278-281):
   ```rust
   let unknown_matches = unknown_match(&self.index, &query, &all_matches);
   let filtered_unknown_matches =
       filter_invalid_contained_unknown_matches(&unknown_matches, &all_matches);
   all_matches.extend(filtered_unknown_matches);
   ```

2. **No configuration option** to disable unknown matching
   - Rust has no `unknown_licenses` parameter
   - No equivalent to Python's `--unknown-licenses` CLI flag

---

## Detailed Implementation Plan

### Overview

Add `unknown_licenses: bool` parameter throughout the call stack, defaulting to `false` to match Python's behavior.

### Parameter Flow

```
CLI (src/cli.rs)
    ↓ unknown_licenses: bool
main.rs (run function)
    ↓ unknown_licenses: bool
scanner/process.rs (process function)
    ↓ unknown_licenses: bool
license_detection/mod.rs (LicenseDetectionEngine::detect)
    ↓ gate condition
unknown_match() - ONLY called when unknown_licenses == true
```

---

### Step 1: Add CLI Flag

**File**: `src/cli.rs`

**Change**: Add `--unknown-licenses` flag to `Cli` struct

```rust
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    // ... existing fields ...

    /// [EXPERIMENTAL] Detect unknown licenses
    /// 
    /// When enabled, the license detector will analyze unmatched text regions
    /// to identify potential unknown licenses. Disabled by default.
    #[arg(long)]
    pub unknown_licenses: bool,
}
```

**Note**: 
- Default is `false` (flag not present = no unknown detection)
- Matches Python's `is_flag=True, default=False`

---

### Step 2: Pass Parameter Through main.rs

**File**: `src/main.rs`

**Change**: Thread `unknown_licenses` from CLI to scanner

Current code (lines 55-62):
```rust
let mut scan_result = process(
    &cli.dir_path,
    cli.max_depth,
    Arc::clone(&progress_bar),
    &exclude_patterns,
    license_engine.clone(),
    cli.include_text,
)?;
```

New code:
```rust
let mut scan_result = process(
    &cli.dir_path,
    cli.max_depth,
    Arc::clone(&progress_bar),
    &exclude_patterns,
    license_engine.clone(),
    cli.include_text,
    cli.unknown_licenses,  // NEW PARAMETER
)?;
```

---

### Step 3: Update scanner/process.rs Function Signature

**File**: `src/scanner/process.rs`

**Change 3a**: Update `process()` function signature (line 20)

Current:
```rust
pub fn process<P: AsRef<Path>>(
    path: P,
    max_depth: usize,
    progress_bar: Arc<ProgressBar>,
    exclude_patterns: &[Pattern],
    license_engine: Option<Arc<LicenseDetectionEngine>>,
    include_text: bool,
) -> Result<ProcessResult, Error>
```

New:
```rust
pub fn process<P: AsRef<Path>>(
    path: P,
    max_depth: usize,
    progress_bar: Arc<ProgressBar>,
    exclude_patterns: &[Pattern],
    license_engine: Option<Arc<LicenseDetectionEngine>>,
    include_text: bool,
    unknown_licenses: bool,  // NEW PARAMETER
) -> Result<ProcessResult, Error>
```

**Change 3b**: Update recursive call (line 79-86)

Current:
```rust
process(
    &path,
    max_depth - 1,
    progress_bar.clone(),
    exclude_patterns,
    license_engine.clone(),
    include_text,
)?
```

New:
```rust
process(
    &path,
    max_depth - 1,
    progress_bar.clone(),
    exclude_patterns,
    license_engine.clone(),
    include_text,
    unknown_licenses,  // NEW PARAMETER
)?
```

**Change 3c**: Update `process_file()` call (line 67)

Current:
```rust
let file_entry = process_file(path, metadata, license_engine.clone(), include_text);
```

New:
```rust
let file_entry = process_file(path, metadata, license_engine.clone(), include_text, unknown_licenses);
```

**Change 3d**: Update `process_file()` signature (line 102)

Current:
```rust
fn process_file(
    path: &Path,
    metadata: &fs::Metadata,
    license_engine: Option<Arc<LicenseDetectionEngine>>,
    include_text: bool,
) -> FileInfo
```

New:
```rust
fn process_file(
    path: &Path,
    metadata: &fs::Metadata,
    license_engine: Option<Arc<LicenseDetectionEngine>>,
    include_text: bool,
    unknown_licenses: bool,  // NEW PARAMETER
) -> FileInfo
```

**Change 3e**: Update `extract_information_from_content()` call (line 111-113)

Current:
```rust
extract_information_from_content(&mut file_info_builder, path, license_engine, include_text)
```

New:
```rust
extract_information_from_content(&mut file_info_builder, path, license_engine, include_text, unknown_licenses)
```

**Change 3f**: Update `extract_information_from_content()` signature (line 144)

Current:
```rust
fn extract_information_from_content(
    file_info_builder: &mut FileInfoBuilder,
    path: &Path,
    license_engine: Option<Arc<LicenseDetectionEngine>>,
    include_text: bool,
) -> Result<(), Error>
```

New:
```rust
fn extract_information_from_content(
    file_info_builder: &mut FileInfoBuilder,
    path: &Path,
    license_engine: Option<Arc<LicenseDetectionEngine>>,
    include_text: bool,
    unknown_licenses: bool,  // NEW PARAMETER
) -> Result<(), Error>
```

**Change 3g**: Update `extract_license_information()` call (line 169-174)

Current:
```rust
extract_license_information(
    file_info_builder,
    text_content,
    license_engine,
    include_text,
)
```

New:
```rust
extract_license_information(
    file_info_builder,
    text_content,
    license_engine,
    include_text,
    unknown_licenses,  // NEW PARAMETER
)
```

**Change 3h**: Update `extract_license_information()` signature (line 181)

Current:
```rust
fn extract_license_information(
    file_info_builder: &mut FileInfoBuilder,
    text_content: String,
    license_engine: Option<Arc<LicenseDetectionEngine>>,
    include_text: bool,
) -> Result<(), Error>
```

New:
```rust
fn extract_license_information(
    file_info_builder: &mut FileInfoBuilder,
    text_content: String,
    license_engine: Option<Arc<LicenseDetectionEngine>>,
    include_text: bool,
    unknown_licenses: bool,  // NEW PARAMETER
) -> Result<(), Error>
```

**Change 3i**: Update `engine.detect()` call (line 191)

Current:
```rust
match engine.detect(&text_content) {
```

New:
```rust
match engine.detect(&text_content, unknown_licenses) {
```

---

### Step 4: Update LicenseDetectionEngine::detect()

**File**: `src/license_detection/mod.rs`

**Change 4a**: Update method signature (line 131)

Current:
```rust
pub fn detect(&self, text: &str) -> Result<Vec<LicenseDetection>> {
```

New:
```rust
pub fn detect(&self, text: &str, unknown_licenses: bool) -> Result<Vec<LicenseDetection>> {
```

**Change 4b**: Update docstring (lines 126-130)

Add parameter documentation:
```rust
/// # Arguments
/// * `text` - The text to analyze
/// * `unknown_licenses` - Whether to detect unknown licenses in unmatched regions.
///                        Default is false (disabled) to match Python's behavior.
```

**Change 4c**: Gate the unknown_match call (lines 278-281)

Current:
```rust
let unknown_matches = unknown_match(&self.index, &query, &all_matches);
let filtered_unknown_matches =
    filter_invalid_contained_unknown_matches(&unknown_matches, &all_matches);
all_matches.extend(filtered_unknown_matches);
```

New:
```rust
if unknown_licenses {
    let unknown_matches = unknown_match(&self.index, &query, &all_matches);
    let filtered_unknown_matches =
        filter_invalid_contained_unknown_matches(&unknown_matches, &all_matches);
    all_matches.extend(filtered_unknown_matches);
}
```

---

### Step 5: Update Test Call Sites

**Files with test code that calls `detect()`**:

1. `src/license_detection/golden_test.rs` - Multiple calls to `engine.detect()`
2. `src/license_detection/extra_detection_investigation_test.rs`
3. `src/license_detection/wrong_detection_investigation_test.rs`
4. `src/license_detection/mod.rs` (internal tests)

**Change**: Add `false` as second argument to all `detect()` calls in tests

Example:
```rust
// Before
engine.detect(&text_content)

// After  
engine.detect(&text_content, false)  // Default behavior - no unknown detection
```

**Special test cases**: Tests specifically for unknown matching should pass `true`:
```rust
engine.detect(&text_content, true)  // Enable unknown detection
```

---

### Step 6: Golden Test Strategy

**Current Status**: The `unknown/` directory tests are failing because Rust always runs unknown matching.

**After Implementation**:

1. **Default behavior tests** (`unknown_licenses=false`):
   - Most golden tests will pass with `false` (matching Python default)
   - The `unknown/` test files will need to be verified

2. **Unknown-specific tests**: Consider creating a separate test configuration:
   - Option A: Add a test attribute/marker for files that require `unknown_licenses=true`
   - Option B: Create separate golden expectation files for unknown detection
   - Option C: Skip unknown tests until explicitly enabled

**Recommended**: Run all golden tests with `unknown_licenses=false` (Python default), and add specific tests for `unknown_licenses=true` behavior.

---

### Step 7: Verification Checklist

After implementation:

- [ ] `cargo build` succeeds
- [ ] `cargo test` passes (all tests updated with `false` parameter)
- [ ] `cargo clippy` passes with no warnings
- [ ] Manual test: `cargo run -- testdata/license-golden/datadriven/unknown/citrix.txt -o test.json` produces 5 detections
- [ ] Manual test: `cargo run -- --unknown-licenses testdata/license-golden/datadriven/unknown/citrix.txt -o test.json` produces unknown matches
- [ ] Golden tests in `unknown/` directory pass with `false` parameter

---

## Summary of Files to Modify

| File | Changes |
|------|---------|
| `src/cli.rs` | Add `--unknown-licenses` flag |
| `src/main.rs` | Pass flag to scanner |
| `src/scanner/process.rs` | Thread parameter through all functions |
| `src/license_detection/mod.rs` | Add parameter to `detect()`, gate unknown matching |
| `src/license_detection/golden_test.rs` | Update `detect()` calls |
| `src/license_detection/*_test.rs` | Update `detect()` calls |

---

## Additional Findings

### Detection Count Discrepancy

The "9 vs 36" in the original problem statement appears to be outdated:
- Current Python default: **5 detections**
- Current Rust: **15 detections**

### Unknown Match Implementation Differences

1. **Python merges ngrams into single match per region**:
   - Each unmatched region produces at most ONE unknown match
   - ngrams are merged using `Span().union(*qspans)`

2. **Rust creates multiple unknown matches**:
   - Each gap between known matches can produce an unknown match
   - More fragmented output

### Weak Match Splitting

Python splits matches into "good" and "weak" before computing unmatched regions:
- citrix.txt: 3 good matches, 8 weak matches
- Only "good" matches define the covered regions
- Rust may not be implementing this split correctly

**Note**: These differences are SEPARATE issues from the gate condition fix. After implementing the gate, if detection counts still differ, those issues should be investigated separately.

---

## Recommended Fix Priority

**HIGH** - This is a significant behavioral difference from Python.

## Estimated Effort

- Implementation: ~1 hour
- Testing: ~30 minutes
- Total: ~1.5 hours
