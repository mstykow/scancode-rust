# PLAN-011: Fix `remove_duplicate_detections` to Use Content-Based Identifier

## Status: DRAFT

## Summary

Fix the `remove_duplicate_detections` function in Rust to use a content-based identifier for deduplication, matching Python's behavior. Currently, Rust deduplicates by `license_expression` alone, causing ~10 tests to fail as separate detections at different file locations are incorrectly merged.

---

## 1. Problem Statement

### 1.1 Current Behavior (Incorrect)

Rust's `remove_duplicate_detections` function in `src/license_detection/detection.rs:793-815` deduplicates detections using only the `license_expression` string as the key:

```rust
pub fn remove_duplicate_detections(detections: Vec<LicenseDetection>) -> Vec<LicenseDetection> {
    let mut unique_detections: std::collections::HashMap<String, LicenseDetection> =
        std::collections::HashMap::new();

    for detection in detections {
        let expr = detection
            .license_expression
            .clone()
            .unwrap_or_else(String::new);

        let score = compute_detection_score(&detection.matches);
        let should_keep = unique_detections
            .get(&expr)
            .map(|existing| score >= compute_detection_score(&existing.matches))
            .unwrap_or(true);

        if should_keep {
            unique_detections.insert(expr, detection);
        }
    }

    unique_detections.into_values().collect()
}
```

### 1.2 Why This Is Wrong

When a file contains the same license expression at multiple locations (e.g., "GPL-2.0" appears in two separate sections), Rust incorrectly merges them into a single detection. This loses important information:

- Each detection should preserve its unique file location (`start_line`, `end_line`)
- Each detection may have different matched text content
- Users need to know WHERE in the file each license appears

### 1.3 Impact

Approximately 10 tests fail due to this issue:

- `datadriven/lic1/ecos-license.html`
- `datadriven/lic1/edl-1.0.txt`
- `datadriven/lic1/gpl-2.0_82.RULE`
- `datadriven/lic1/gpl-2.0_and_gpl-2.0-plus.txt`
- `datadriven/lic1/gpl_and_gpl_and_gpl_and_lgpl-2.0_and_other.txt`
- `datadriven/lic1/gpl_65.txt`
- And others

---

## 2. Python Reference Analysis

### 2.1 How Python Creates the Identifier

From `reference/scancode-toolkit/src/licensedcode/detection.py:305-341`:

```python
@property
def _identifier(self):
    """
    Return an unique identifier for a license detection, based on it's
    underlying license matches with the tokenized matched_text.
    """
    data = []
    for match in self.matches:

        matched_text = match.matched_text
        if isinstance(matched_text, typing.Callable):
            matched_text = matched_text()
            if matched_text is None:
                matched_text = ''
        if not isinstance(matched_text, str):
            matched_text = repr(matched_text)

        tokenized_matched_text = tuple(query_tokenizer(matched_text))

        identifier = (
            match.rule.identifier,
            match.score(),
            tokenized_matched_text,
        )
        data.append(identifier)

    # Return a uuid generated from the contents of the matches
    return get_uuid_on_content(content=data)

@property
def identifier_with_expression(self):
    """
    Return an identifer for a license detection with the license expression
    and an UUID created from the detection contents.
    """
    id_safe_expression = python_safe_name(s=str(self.license_expression))
    return "{}-{}".format(id_safe_expression, self._identifier)
```

### 2.2 UUID Generation from Content

From `reference/scancode-toolkit/src/licensedcode/detection.py:513-520`:

```python
def get_uuid_on_content(content):
    """
    Return an UUID based on the contents of a list, which should be
    a list of hashable elements.
    """
    identifier_string = repr(tuple(content))
    md_hash = sha1(identifier_string.encode('utf-8'))
    return str(uuid.UUID(hex=md_hash.hexdigest()[:32]))
```

### 2.3 How Python Groups Detections

From `reference/scancode-toolkit/src/licensedcode/detection.py:1017-1027`:

```python
def get_detections_by_id(license_detections):
    """
    Get a dict(hashmap) where each item is: {detection.identifier: all_detections} where
    `all_detections` is all detections in `license_detections` whose detection.identifier
    is the same.
    """
    detections_by_id = defaultdict(list)
    for detection in license_detections:
        detections_by_id[detection.identifier].append(detection)

    return detections_by_id
```

### 2.4 How Python Uses This for Unique Detections

From `reference/scancode-toolkit/src/licensedcode/detection.py:918-961`:

```python
@classmethod
def get_unique_detections(cls, license_detections):
    """
    Return all unique UniqueDetection from a ``license_detections`` list of
    LicenseDetection.
    """
    licensing = get_licensing()
    detections_by_id = get_detections_by_id(license_detections)
    unique_license_detections = []

    for all_detections in detections_by_id.values():
        file_regions = [
            detection.file_region
            for detection in all_detections
        ]
        detection = next(iter(all_detections))
        # ... creates UniqueDetection with detection_count and file_regions
```

**Key Insight:** Python groups detections by their `identifier` (expression + content hash), NOT by `license_expression` alone. This preserves location uniqueness.

---

## 3. Rust Code Analysis

### 3.1 LicenseDetection Struct

From `src/license_detection/detection.rs:98-119`:

```rust
pub struct LicenseDetection {
    pub license_expression: Option<String>,
    pub license_expression_spdx: Option<String>,
    pub matches: Vec<LicenseMatch>,
    pub detection_log: Vec<String>,
    pub identifier: Option<String>,           // Already exists!
    pub file_region: Option<FileRegion>,
}
```

**Note:** The `identifier` field already exists but is currently not used for deduplication.

### 3.2 LicenseMatch Struct

From `src/license_detection/models.rs:174-224`:

```rust
pub struct LicenseMatch {
    pub license_expression: String,
    pub license_expression_spdx: String,
    pub from_file: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub matcher: String,
    pub score: f32,
    pub matched_length: usize,
    pub match_coverage: f32,
    pub rule_relevance: u8,
    pub rule_identifier: String,              // Used in identifier
    pub rule_url: String,
    pub matched_text: Option<String>,         // Used in identifier
    pub referenced_filenames: Option<Vec<String>>,
    pub is_license_intro: bool,
    pub is_license_clue: bool,
}
```

### 3.3 FileRegion Struct

From `src/license_detection/detection.rs:121-132`:

```rust
pub struct FileRegion {
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
}
```

### 3.4 Current Test Coverage

The existing tests in `src/license_detection/detection.rs:2516-2612` verify:

1. `test_remove_duplicate_detections_different_expressions` - Passes (different expressions are kept separate)
2. `test_remove_duplicate_detections_same_expression_keeps_best` - **Incorrectly expects 1 result** (should keep both)
3. `test_remove_duplicate_detections_empty` - Passes

**Critical Issue:** The test `test_remove_duplicate_detections_same_expression_keeps_best` enforces the WRONG behavior. It expects deduplication by expression, but the fix requires keeping separate detections.

---

## 4. Proposed Changes

### 4.1 New Helper Function: `compute_detection_identifier`

Create a function that computes a content-based identifier similar to Python's approach:

**Location:** `src/license_detection/detection.rs`

```rust
use sha1::{Digest, Sha1};
use uuid::Uuid;

/// Compute a content-based identifier for a detection.
///
/// This creates a unique identifier based on:
/// 1. The license expression (sanitized)
/// 2. A UUID derived from match contents (rule_identifier, score, matched_text)
///
/// This matches Python's `identifier_with_expression` property.
pub fn compute_detection_identifier(detection: &LicenseDetection) -> String {
    let expression = detection
        .license_expression
        .as_ref()
        .map(|s| python_safe_name(s))
        .unwrap_or_default();

    let content_uuid = compute_content_uuid(&detection.matches);
    format!("{}-{}", expression, content_uuid)
}

/// Compute a UUID from match contents.
///
/// Uses the same algorithm as Python's `get_uuid_on_content`:
/// - Collects (rule_identifier, score, matched_text) tuples from matches
/// - Creates SHA1 hash of the tuple representation
/// - Returns a UUID from the first 32 hex characters
fn compute_content_uuid(matches: &[LicenseMatch]) -> String {
    let content: Vec<(&str, f32, &str)> = matches
        .iter()
        .map(|m| {
            let matched_text = m.matched_text.as_deref().unwrap_or("");
            (m.rule_identifier.as_str(), m.score, matched_text)
        })
        .collect();

    // Create deterministic string representation
    let content_repr = format!("{:?}", content);

    // SHA1 hash
    let mut hasher = Sha1::new();
    hasher.update(content_repr.as_bytes());
    let hash = hasher.finalize();

    // Take first 32 hex characters for UUID
    let hex_string = hex::encode(hash);
    let uuid_str = &hex_string[..32];

    // Parse as UUID and format
    Uuid::parse_str(uuid_str)
        .map(|u| u.to_string())
        .unwrap_or_else(|_| uuid_str.to_string())
}

/// Convert a string to a Python-safe identifier name.
///
/// Replaces non-alphanumeric characters with underscores.
fn python_safe_name(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .replace("__", "_")
        .trim_matches('_')
        .to_string()
}
```

**Dependencies to add to `Cargo.toml`:**
```toml
sha1 = "0.10"
hex = "0.4"
uuid = { version = "1.0", features = ["v4"] }
```

### 4.2 Modify `remove_duplicate_detections`

**Location:** `src/license_detection/detection.rs:793-815`

Replace the current implementation:

```rust
/// Remove duplicate detections.
///
/// Groups detections by their `identifier` (license expression + content hash).
/// Detections with the same identifier represent the same license at the same
/// location. Detections with the same expression but different identifiers
/// represent the same license at DIFFERENT locations and should be kept separate.
///
/// This matches Python's `get_detections_by_id` behavior.
pub fn remove_duplicate_detections(detections: Vec<LicenseDetection>) -> Vec<LicenseDetection> {
    let mut detections_by_id: std::collections::HashMap<String, LicenseDetection> =
        std::collections::HashMap::new();

    for detection in detections {
        // Use existing identifier or compute one
        let identifier = detection.identifier.clone().unwrap_or_else(|| {
            compute_detection_identifier(&detection)
        });

        // Only keep if this exact identifier hasn't been seen
        // (same expression + same content = true duplicate)
        if !detections_by_id.contains_key(&identifier) {
            let mut detection = detection;
            detection.identifier = Some(identifier.clone());
            detections_by_id.insert(identifier, detection);
        }
    }

    detections_by_id.into_values().collect()
}
```

### 4.3 Update `populate_detection_from_group`

Ensure the `identifier` field is populated when creating detections:

**Location:** `src/license_detection/detection.rs:652-681`

Add identifier computation at the end of the function:

```rust
pub fn populate_detection_from_group(detection: &mut LicenseDetection, group: &DetectionGroup) {
    // ... existing code ...

    // Compute and set identifier
    detection.identifier = Some(compute_detection_identifier(detection));

    if group.start_line > 0 {
        detection.file_region = Some(FileRegion {
            path: String::new(),
            start_line: group.start_line,
            end_line: group.end_line,
        });
    }
}
```

### 4.4 Add Tokenizer for Matched Text (Optional Enhancement)

For exact parity with Python, tokenize the matched text before hashing:

**Location:** `src/license_detection/tokenize.rs` (if it exists) or `src/license_detection/detection.rs`

```rust
/// Simple query tokenizer matching Python's behavior.
///
/// Lowercases text and splits on non-alphanumeric characters.
pub fn query_tokenize(text: &str) -> Vec<&str> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect()
}
```

Then use in `compute_content_uuid`:

```rust
fn compute_content_uuid(matches: &[LicenseMatch]) -> String {
    let content: Vec<(&str, f32, Vec<&str>)> = matches
        .iter()
        .map(|m| {
            let matched_text = m.matched_text.as_deref().unwrap_or("");
            let tokens = query_tokenize(matched_text);
            (m.rule_identifier.as_str(), m.score, tokens)
        })
        .collect();
    // ... rest of function
}
```

---

## 5. Testing Strategy

### 5.1 Update Existing Unit Tests

**File:** `src/license_detection/detection.rs`

Modify `test_remove_duplicate_detections_same_expression_keeps_best` to expect BOTH detections to be kept when they have different content:

```rust
#[test]
fn test_remove_duplicate_detections_same_expression_different_content() {
    // Two detections with same expression but different locations/content
    // should be kept as separate detections
    let detections = vec![
        LicenseDetection {
            license_expression: Some("gpl-2.0".to_string()),
            license_expression_spdx: None,
            matches: vec![create_test_match_with_params(
                "gpl-2.0",
                "1-hash",
                1,   // start_line
                10,  // end_line
                95.0,
                100,
                100.0,
                100,
                "gpl-2.0.LICENSE",
            )],
            detection_log: Vec::new(),
            identifier: None,
            file_region: Some(FileRegion { path: String::new(), start_line: 1, end_line: 10 }),
        },
        LicenseDetection {
            license_expression: Some("gpl-2.0".to_string()),
            license_expression_spdx: None,
            matches: vec![create_test_match_with_params(
                "gpl-2.0",
                "1-hash",
                100,  // Different start_line
                110,  // Different end_line
                95.0,
                100,
                100.0,
                100,
                "gpl-2.0.LICENSE",
            )],
            detection_log: Vec::new(),
            identifier: None,
            file_region: Some(FileRegion { path: String::new(), start_line: 100, end_line: 110 }),
        },
    ];

    let result = remove_duplicate_detections(detections);
    assert_eq!(result.len(), 2, "Two detections with same expression but different locations should both be kept");
}

#[test]
fn test_remove_duplicate_detections_same_identifier_removed() {
    // Two detections with the SAME identifier (same content) should be deduplicated
    let identifier = "gpl_2_0-abc123".to_string();
    let detections = vec![
        LicenseDetection {
            license_expression: Some("gpl-2.0".to_string()),
            license_expression_spdx: None,
            matches: vec![create_test_match_with_params(
                "gpl-2.0", "1-hash", 1, 10, 95.0, 100, 100.0, 100, "gpl-2.0.LICENSE",
            )],
            detection_log: Vec::new(),
            identifier: Some(identifier.clone()),
            file_region: None,
        },
        LicenseDetection {
            license_expression: Some("gpl-2.0".to_string()),
            license_expression_spdx: None,
            matches: vec![create_test_match_with_params(
                "gpl-2.0", "1-hash", 1, 10, 95.0, 100, 100.0, 100, "gpl-2.0.LICENSE",
            )],
            detection_log: Vec::new(),
            identifier: Some(identifier.clone()),
            file_region: None,
        },
    ];

    let result = remove_duplicate_detections(detections);
    assert_eq!(result.len(), 1, "Detections with identical identifier should be deduplicated");
}
```

### 5.2 Add New Unit Tests

```rust
#[test]
fn test_compute_detection_identifier_deterministic() {
    let detection = LicenseDetection {
        license_expression: Some("mit".to_string()),
        license_expression_spdx: None,
        matches: vec![create_test_match_with_params(
            "mit", "1-hash", 1, 10, 95.0, 100, 100.0, 100, "mit.LICENSE",
        )],
        detection_log: Vec::new(),
        identifier: None,
        file_region: None,
    };

    let id1 = compute_detection_identifier(&detection);
    let id2 = compute_detection_identifier(&detection);
    assert_eq!(id1, id2, "Identifier should be deterministic");
    assert!(id1.starts_with("mit-"), "Identifier should start with expression");
}

#[test]
fn test_compute_detection_identifier_different_content() {
    let detection1 = LicenseDetection {
        license_expression: Some("mit".to_string()),
        matches: vec![create_test_match_with_params(
            "mit", "1-hash", 1, 10, 95.0, 100, 100.0, 100, "mit.LICENSE",
        )],
        ..Default::default()
    };

    let detection2 = LicenseDetection {
        license_expression: Some("mit".to_string()),
        matches: vec![create_test_match_with_params(
            "mit", "1-hash", 100, 110, 95.0, 100, 100.0, 100, "mit.LICENSE",  // Different location
        )],
        ..Default::default()
    };

    let id1 = compute_detection_identifier(&detection1);
    let id2 = compute_detection_identifier(&detection2);
    assert_ne!(id1, id2, "Different content should produce different identifiers");
}
```

### 5.3 Golden Test Verification

After implementing the fix, run the golden tests for the affected files:

```bash
# Run specific failing tests
cargo test --test golden_test -- --test-threads=1 ecos-license.html
cargo test --test golden_test -- --test-threads=1 edl-1.0.txt
cargo test --test golden_test -- --test-threads=1 gpl-2.0_82.RULE
cargo test --test golden_test -- --test-threads=1 gpl-2.0_and_gpl-2.0-plus.txt
cargo test --test golden_test -- --test-threads=1 gpl_and_gpl_and_gpl_and_lgpl-2.0_and_other.txt
cargo test --test golden_test -- --test-threads=1 gpl_65.txt
```

### 5.4 Integration Test

Create an integration test that verifies the expected output for a file with multiple instances of the same license:

```rust
#[test]
fn test_multiple_same_license_detections_preserved() {
    // File: testdata/lic1/gpl-2.0_and_gpl-2.0-plus.txt
    // This file should produce 2 detections:
    // 1. gpl-2.0 at one location
    // 2. gpl-2.0-plus at another location
    // Both should be preserved, not merged
}
```

---

## 6. Implementation Checklist

### Phase 1: Core Implementation

- [ ] Add `sha1`, `hex`, and `uuid` dependencies to `Cargo.toml`
- [ ] Implement `python_safe_name()` helper function
- [ ] Implement `compute_content_uuid()` function
- [ ] Implement `compute_detection_identifier()` function
- [ ] Update `remove_duplicate_detections()` to use identifier-based grouping
- [ ] Update `populate_detection_from_group()` to set identifier

### Phase 2: Optional Enhancement

- [ ] Implement `query_tokenize()` function for exact Python parity
- [ ] Update `compute_content_uuid()` to use tokenized matched text

### Phase 3: Testing

- [ ] Update `test_remove_duplicate_detections_same_expression_keeps_best` test
- [ ] Add `test_remove_duplicate_detections_same_expression_different_content` test
- [ ] Add `test_remove_duplicate_detections_same_identifier_removed` test
- [ ] Add `test_compute_detection_identifier_deterministic` test
- [ ] Add `test_compute_detection_identifier_different_content` test
- [ ] Run all unit tests: `cargo test --lib`
- [ ] Run golden tests for affected files
- [ ] Run full test suite: `cargo test`

### Phase 4: Verification

- [ ] Verify output matches Python for `ecos-license.html`
- [ ] Verify output matches Python for `edl-1.0.txt`
- [ ] Verify output matches Python for `gpl-2.0_82.RULE`
- [ ] Verify output matches Python for `gpl-2.0_and_gpl-2.0-plus.txt`
- [ ] Verify output matches Python for `gpl_and_gpl_and_gpl_and_lgpl-2.0_and_other.txt`
- [ ] Verify output matches Python for `gpl_65.txt`
- [ ] Run clippy: `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Run fmt: `cargo fmt --all -- --check`

---

## 7. Edge Cases to Consider

### 7.1 Empty matched_text

When `matched_text` is `None`, use empty string for hashing. This is consistent with Python:

```python
if matched_text is None:
    matched_text = ''
```

### 7.2 Detections Without Expression

When `license_expression` is `None`, use empty string for the identifier prefix. Python handles this in `python_safe_name`:

```rust
let expression = detection
    .license_expression
    .as_ref()
    .map(|s| python_safe_name(s))
    .unwrap_or_default();
```

### 7.3 Matched Text as Callable (Python-specific)

Python's `matched_text` can be a callable. In Rust, `LicenseMatch.matched_text` is always `Option<String>`, so this is not applicable.

### 7.4 UUID Format

Python generates a UUID from SHA1 hash. Ensure Rust's UUID format matches exactly:

```rust
// Python: str(uuid.UUID(hex=md_hash.hexdigest()[:32]))
// Rust equivalent:
Uuid::parse_str(&hex_string[..32])?.to_string()
```

---

## 8. References

- Python detection.py: `reference/scancode-toolkit/src/licensedcode/detection.py`
  - Lines 305-341: `_identifier` and `identifier_with_expression` properties
  - Lines 513-520: `get_uuid_on_content` function
  - Lines 1017-1027: `get_detections_by_id` function
  - Lines 918-961: `UniqueDetection.get_unique_detections` class method

- Python tokenize.py: `reference/scancode-toolkit/src/licensedcode/tokenize.py`
  - Lines 309-329: `query_tokenizer` function

- Rust implementation: `src/license_detection/detection.rs`
  - Lines 793-815: Current `remove_duplicate_detections` function
  - Lines 98-119: `LicenseDetection` struct
  - Lines 652-681: `populate_detection_from_group` function

- Rust models: `src/license_detection/models.rs`
  - Lines 174-224: `LicenseMatch` struct

---

## 9. Estimated Effort

- **Core Implementation:** 2-4 hours
- **Testing:** 1-2 hours
- **Verification & Debugging:** 1-2 hours
- **Total:** 4-8 hours
