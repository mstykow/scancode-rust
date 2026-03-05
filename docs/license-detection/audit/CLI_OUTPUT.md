# CLI Integration and Output Format Comparison

This document compares the CLI integration and output format between Python ScanCode Toolkit and Rust scancode-rust for license detection.

## 1. CLI Entry Point

### Python

**File**: `reference/scancode-toolkit/src/licensedcode/plugin_license.py`

**Entry Point**: Plugin-based architecture via `ScanPlugin` class.

```python
# plugin_license.py:52-82
@scan_impl
class LicenseScanner(ScanPlugin):
    """Scan a Resource for licenses."""
    
    resource_attributes = dict([
        ('detected_license_expression', attr.ib(default=None)),
        ('detected_license_expression_spdx', attr.ib(default=None)),
        ('license_detections', attr.ib(default=attr.Factory(list))),
        ('license_clues', attr.ib(default=attr.Factory(list))),
        ('percentage_of_license_text', attr.ib(default=0)),
    ])
    
    codebase_attributes = dict(
        license_detections=attr.ib(default=attr.Factory(list)),
    )
    
    run_order = 4
    sort_order = 5
```

**CLI Options** (plugin_license.py:83-145):
- `--license` - Enable license scanning
- `--license-score` - Minimum score threshold (0-100)
- `--license-text` - Include matched text in output
- `--license-text-diagnostics` - Include diagnostic highlights
- `--license-diagnostics` - Include detection post-processing details
- `--license-url-template` - Template for license reference URLs
- `--unknown-licenses` - Experimental unknown license detection

**Invocation Flow**:
1. `setup()` - Cache warmup for child processes (plugin_license.py:150-159)
2. `get_scanner()` - Returns partial function with detection parameters (plugin_license.py:161-180)
3. `process_codebase()` - Post-process for referenced filenames and collect unique detections (plugin_license.py:182-256)

### Rust

**File**: `src/main.rs`

**Entry Point**: Direct CLI via `clap` parser.

```rust
// main.rs:30-86
fn main() -> std::io::Result<()> {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
    Ok(())
}

fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    // ... initialization ...
    let license_engine = init_license_engine(&cli.license_rules_path);
    let mut scan_result = process(
        &cli.dir_path,
        cli.max_depth,
        Arc::clone(&progress_bar),
        &exclude_patterns,
        license_engine.clone(),
        cli.include_text,
    )?;
    // ... assembly and output ...
}
```

**CLI Options** (`src/cli.rs:3-35`):
- `dir_path` - Directory path to scan (required)
- `-o, --output-file` - Output JSON file path (default: "output.json")
- `--max-depth` - Maximum recursion depth (default: 50)
- `--exclude` - Glob exclusion patterns
- `--no-assemble` - Disable package assembly
- `--license-rules-path` - Path to license rules directory
- `--include-text` - Include matched text in license detection output

### Key Differences

| Aspect | Python | Rust |
|--------|--------|------|
| Architecture | Plugin-based, runs at `run_order=4` | Direct integration, always runs |
| CLI Options | 7 license-specific options | 1 include-text option, rules path |
| License Scanning | Opt-in via `--license` flag | Always enabled if rules path exists |
| Threshold Control | `--license-score` (0-100) | No CLI threshold (hardcoded in engine) |
| Diagnostics | Multiple diagnostic flags | No diagnostic options |
| Unknown Licenses | Experimental `--unknown-licenses` | Not implemented |

**Compatibility Issues**:
1. **Missing CLI options**: Rust lacks `--license-score`, `--license-text-diagnostics`, `--license-diagnostics`
2. **Always-on behavior**: Rust runs license detection by default; Python requires `--license` flag
3. **No opt-out**: No `--no-license` option in Rust to disable scanning

---

## 2. Output JSON Structure

### Python

**Top-Level Output Structure**:

```json
{
  "headers": [...],
  "packages": [...],
  "dependencies": [...],
  "files": [...],
  "license_detections": [...],  // Top-level unique detections
  ...
}
```

**Resource-Level Attributes** (plugin_license.py:58-74):
- `detected_license_expression` - Combined license expression (ScanCode keys)
- `detected_license_expression_spdx` - SPDX license expression
- `license_detections` - List of license detection objects
- `license_clues` - Low-quality matches not in detections
- `percentage_of_license_text` - Percentage of file words detected as license

### Rust

**Top-Level Output Structure** (`src/models/output.rs:6-14`):

```rust
pub struct Output {
    pub headers: Vec<Header>,
    pub packages: Vec<Package>,
    pub dependencies: Vec<TopLevelDependency>,
    pub files: Vec<FileInfo>,
    pub license_references: Vec<LicenseReference>,      // TODO: implement
    pub license_rule_references: Vec<LicenseRuleReference>, // TODO: implement
}
```

**File-Level Attributes** (`src/models/file_info.rs:9-46`):
```rust
pub struct FileInfo {
    pub name: String,
    pub base_name: String,
    pub extension: String,
    pub path: String,
    pub file_type: FileType,
    pub mime_type: Option<String>,
    pub size: u64,
    pub date: Option<String>,
    pub sha1: Option<String>,
    pub md5: Option<String>,
    pub sha256: Option<String>,
    pub programming_language: Option<String>,
    pub package_data: Vec<PackageData>,
    pub license_expression: Option<String>,  // Serialized as "detected_license_expression_spdx"
    pub license_detections: Vec<LicenseDetection>,
    pub copyrights: Vec<Copyright>,
    pub urls: Vec<OutputURL>,
    pub for_packages: Vec<String>,
    pub scan_errors: Vec<String>,
}
```

### Key Differences

| Aspect | Python | Rust |
|--------|--------|------|
| Top-level `license_detections` | Present (unique detections across codebase) | Missing |
| `license_clues` | Present (low-quality matches) | Missing |
| `percentage_of_license_text` | Present | Missing |
| `detected_license_expression` | ScanCode keys | Missing (only SPDX version) |
| `license_references` | Present | TODO placeholder |
| `license_rule_references` | Present | TODO placeholder |

**Compatibility Issues**:
1. **Missing top-level `license_detections`**: Python collects unique detections at codebase level; Rust only has per-file detections
2. **Missing `license_clues`**: Low-quality matches are not tracked separately
3. **Missing `percentage_of_license_text`**: File-level license coverage metric not computed
4. **Missing `detected_license_expression`**: Only SPDX expression is serialized (as `detected_license_expression_spdx`)
5. **Missing `license_references` and `license_rule_references`**: Not yet implemented

---

## 3. Detection Output Format

### Python

**LicenseDetection Structure** (`detection.py:164-217`):

```python
@attr.s(slots=True, eq=False, order=False)
class LicenseDetection:
    license_expression = attr.ib(default=None)  # ScanCode keys
    license_expression_spdx = attr.ib(default=None)  # SPDX keys
    matches = attr.ib(default=attr.Factory(list))  # List of LicenseMatch
    detection_log = attr.ib(default=attr.Factory(list))  # DetectionRule entries
    identifier = attr.ib(default=None)  # Unique ID: "{expression}-{uuid}"
    file_region = attr.ib(default=attr.Factory(dict))  # FileRegion with path/lines
```

**to_dict() Method** (detection.py:476-510):
- Excludes `file_region` from output
- Optionally excludes `detection_log` (unless `license_diagnostics=True`)
- Converts each match via `match.to_dict()`

### Rust

**LicenseDetection Structure** (`src/models/file_info.rs:261-267`):

```rust
pub struct LicenseDetection {
    pub license_expression: String,
    pub license_expression_spdx: String,
    pub matches: Vec<Match>,
    pub identifier: Option<String>,
}
```

**Internal Detection Structure** (`src/license_detection/detection/types.rs:38-58`):
```rust
pub struct LicenseDetection {
    pub license_expression: Option<String>,
    pub license_expression_spdx: Option<String>,
    pub matches: Vec<LicenseMatch>,
    pub detection_log: Vec<String>,
    pub identifier: Option<String>,
    pub file_region: Option<FileRegion>,
}
```

### Key Differences

| Field | Python | Rust (Output) | Rust (Internal) |
|-------|--------|---------------|-----------------|
| `license_expression` | Always string or None | Required String | Option<String> |
| `license_expression_spdx` | Always string or None | Required String | Option<String> |
| `matches` | List of dicts | Vec<Match> | Vec<LicenseMatch> |
| `detection_log` | List of DetectionRule strings | **Missing** | Vec<String> (not serialized) |
| `identifier` | String "{expr}-{uuid}" | Option<String> | Option<String> |
| `file_region` | Present (excluded from output) | **Missing** | Option<FileRegion> (not serialized) |

**Compatibility Issues**:
1. **Missing `detection_log` in output**: Internal tracking exists but not serialized to JSON
2. **Missing `file_region`**: Internal structure exists but not exposed in output model
3. **Required vs Optional**: Rust model uses `String` (required), Python uses `None` for failed detections

---

## 4. Match Output Format

### Python

**LicenseMatch to_dict()** (`match.py:797-840`):

```python
def to_dict(self, ...):
    result = {}
    result['license_expression'] = self.rule.license_expression
    result['license_expression_spdx'] = self.rule.spdx_license_expression()
    result['from_file'] = file_path
    result['start_line'] = self.start_line
    result['end_line'] = self.end_line
    result['matcher'] = self.matcher
    result['score'] = self.score()  # 0-100
    result['matched_length'] = self.len()
    result['match_coverage'] = self.coverage()  # 0-100
    result['rule_relevance'] = self.rule.relevance
    result['rule_identifier'] = self.rule.identifier
    result['rule_url'] = self.rule.rule_url
    if include_text:
        result['matched_text'] = matched_text
    if license_text_diagnostics:
        result['matched_text_diagnostics'] = matched_text_diagnostics
    return result
```

**LicenseMatchFromResult to_dict()** (`detection.py:680-718`):
Additional fields when `rule_details=True`:
- `rule_length`
- `rule_notes`
- `referenced_filenames`
- `rule_text`

### Rust

**Match Structure** (`src/models/file_info.rs:273-295`):

```rust
pub struct Match {
    pub license_expression: String,
    pub license_expression_spdx: String,
    pub from_file: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub matcher: Option<String>,
    pub score: f64,  // 0-100
    pub matched_length: Option<usize>,
    pub match_coverage: Option<f64>,  // 0-100
    pub rule_relevance: Option<usize>,
    pub rule_identifier: Option<String>,
    pub rule_url: Option<String>,
    pub matched_text: Option<String>,
}
```

**LicenseMatch Internal Structure** (`src/license_detection/models/license_match.rs:12-154`):
```rust
pub struct LicenseMatch {
    pub rid: usize,  // #[serde(skip)] - not serialized
    pub license_expression: String,
    pub license_expression_spdx: String,
    pub from_file: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub start_token: usize,  // #[serde(default)]
    pub end_token: usize,    // #[serde(default)]
    pub matcher: String,
    pub score: f32,  // 0.0-1.0 internally, converted to 0-100 for output
    pub matched_length: usize,
    pub rule_length: usize,  // #[serde(default)]
    pub match_coverage: f32,  // 0.0-100.0
    pub rule_relevance: u8,   // 0-100
    pub rule_identifier: String,
    pub rule_url: String,
    pub matched_text: Option<String>,
    pub referenced_filenames: Option<Vec<String>>,
    pub is_license_intro: bool,
    pub is_license_clue: bool,
    pub is_license_reference: bool,  // #[serde(default)]
    pub is_license_tag: bool,        // #[serde(default)]
    pub is_license_text: bool,       // #[serde(default)]
    pub is_from_license: bool,       // #[serde(default)]
    // ... internal fields with #[serde(skip)] ...
}
```

### Key Differences

| Field | Python | Rust (Output) | Rust (Internal) |
|-------|--------|---------------|-----------------|
| `score` | 0-100 (computed) | f64 (0-100) | f32 (0.0-1.0), converted |
| `matcher` | String | Option<String> | String |
| `matched_length` | Always present | Option<usize> | usize |
| `match_coverage` | Always present | Option<f64> | f32 |
| `rule_relevance` | Always present | Option<usize> | u8 |
| `rule_identifier` | Always present | Option<String> | String |
| `rule_url` | Always present | Option<String> | String |
| `matched_text_diagnostics` | Present | **Missing** | N/A |
| `rule_length` | Optional (rule_details) | **Missing** | usize (not serialized) |
| `referenced_filenames` | Optional (rule_details) | **Missing** | Option<Vec<String>> (not serialized) |
| `is_license_*` flags | Not in output | **Missing** | Present (not serialized) |

**Conversion** (`src/scanner/process.rs:235-253`):
```rust
Match {
    license_expression: m.license_expression,
    license_expression_spdx: m.license_expression_spdx,
    from_file: m.from_file,
    start_line: m.start_line,
    end_line: m.end_line,
    matcher: Some(m.matcher),
    score: m.score as f64,  // Note: score is f32 0.0-1.0, converted wrong
    matched_length: Some(m.matched_length),
    match_coverage: Some(m.match_coverage as f64),
    rule_relevance: Some(m.rule_relevance as usize),
    rule_identifier: Some(m.rule_identifier),
    rule_url: Some(m.rule_url),
    matched_text: if include_text { m.matched_text } else { None },
}
```

**Compatibility Issues**:
1. **Score conversion bug**: Internal `score` is 0.0-1.0, but conversion casts directly to f64 without multiplying by 100
2. **Missing `matched_text_diagnostics`**: No diagnostic highlighting support
3. **Missing `rule_length`**: Not exposed in output
4. **Missing `referenced_filenames`**: Not exposed in output
5. **Optional vs Required**: Many fields use `Option<T>` where Python always includes them

---

## 5. Error Handling

### Python

**Detection Errors**:
- Failed detections return `None` (detection.py:254-258)
- Detection log contains `DetectionRule` entries explaining failures
- No exceptions thrown for detection failures

**File Processing Errors**:
- Tracked in resource attributes
- Logged with `logger.warn()` in plugin

### Rust

**Detection Errors** (`src/scanner/process.rs:196-223`):
```rust
match engine.detect(&text_content, false) {
    Ok(detections) => {
        // Process detections
    }
    Err(e) => {
        warn!("License detection failed: {}", e);
    }
}
```

**File Processing Errors** (`src/scanner/process.rs:108-115`):
```rust
let mut scan_errors: Vec<String> = vec![];

if let Err(e) = extract_information_from_content(...) {
    scan_errors.push(e.to_string());
};
```

### Key Differences

| Aspect | Python | Rust |
|--------|--------|------|
| Detection Failure | Returns None, logs in detection_log | Logs warning, continues |
| File Errors | Resource-level tracking | FileInfo.scan_errors |
| Exception Handling | Graceful degradation with None | Error propagation with Result |

---

## 6. File Region/Path Handling

### Python

**FileRegion Structure** (`detection.py:150-162`):
```python
@attr.s
class FileRegion:
    path = attr.ib(type=str)
    start_line = attr.ib(type=int)
    end_line = attr.ib(type=int)
```

**Usage**:
- `from_file` attribute on each LicenseMatch tracks origin file
- `file_region` on LicenseDetection for location tracking
- Populated during `collect_license_detections()` (detection.py:759-763)
- Used for cross-file reference resolution

### Rust

**FileRegion Structure** (`src/license_detection/detection/types.rs:63-71`):
```rust
pub struct FileRegion {
    pub path: String,
    pub start_line: usize,  // 1-indexed
    pub end_line: usize,    // 1-indexed
}
```

**Usage**:
- `from_file` on LicenseMatch is `Option<String>` (not always set)
- `file_region` on LicenseDetection is `Option<FileRegion>`
- Not serialized to output model

**Conversion** (`src/scanner/process.rs:241-242`):
```rust
from_file: m.from_file,  // Direct copy from internal model
```

### Key Differences

| Aspect | Python | Rust |
|--------|--------|------|
| `from_file` population | Always populated via `populate_matches_with_path()` | Optional, may be None |
| `file_region` serialization | Excluded from output | Not included in output model |
| Cross-file references | Fully supported | Not implemented |

---

## Summary of Compatibility Issues

### Critical Issues

1. **Missing top-level `license_detections`**: Python outputs unique detections at codebase level; Rust only has per-file detections
2. **Score conversion bug**: Internal 0.0-1.0 score not multiplied by 100 before output
3. **Missing CLI options**: `--license`, `--license-score`, `--license-text-diagnostics`, `--license-diagnostics`, `--unknown-licenses`
4. **Always-on behavior**: No way to disable license scanning in Rust

### High Priority Issues

5. **Missing `license_clues`**: Low-quality matches are discarded, not tracked
6. **Missing `percentage_of_license_text`**: File coverage metric not computed
7. **Missing `detected_license_expression`**: Only SPDX version exposed (renamed to `detected_license_expression_spdx`)
8. **Missing `detection_log`**: Internal tracking exists but not serialized
9. **Missing `matched_text_diagnostics`**: No diagnostic highlighting support

### Medium Priority Issues

10. **Missing `license_references` and `license_rule_references`**: TODO placeholders in output
11. **Missing `referenced_filenames`**: Cross-file reference tracking not exposed
12. **Missing `rule_length`**: Match quality indicator not in output
13. **Optional vs Required fields**: Many Python-required fields are Optional in Rust

### Low Priority Issues

14. **Missing `is_license_*` flags**: Rule type indicators not exposed
15. **No opt-out mechanism**: Cannot disable license scanning per-run
16. **No threshold control**: Score threshold hardcoded, not configurable via CLI

---

## Recommended Actions

### Immediate Fixes Required

1. **Fix score conversion**: Multiply internal score by 100 in `convert_detection_to_model()`
2. **Add top-level `license_detections`**: Implement `UniqueDetection` collection across codebase
3. **Add `detection_log` to output**: Serialize internal `detection_log` field

### Short-term Additions

4. **Add `--license` flag**: Make license scanning opt-in for parity
5. **Add `--license-score` option**: Allow threshold configuration
6. **Add `license_clues` tracking**: Preserve low-quality matches separately
7. **Populate `from_file` consistently**: Ensure all matches have origin file path

### Long-term Improvements

8. **Implement `percentage_of_license_text`**: Add file coverage metric
9. **Add `license_references` collection**: Output all referenced licenses
10. **Add diagnostic options**: `--license-text-diagnostics`, `--license-diagnostics`
11. **Implement cross-file references**: Support `referenced_filenames` resolution
