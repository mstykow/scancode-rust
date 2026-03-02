# PLAN-019: Implement UniqueDetection for Output Formatting

## Status: READY TO IMPLEMENT

## Summary

Implement `UniqueDetection` to group detections by identifier while preserving all file regions, matching Python's output format for license detections.

## Problem Statement

### Current State

The Rust implementation has two dead-code functions:
- `remove_duplicate_detections()` at `detection.rs:934`
- `compute_detection_identifier()` at `detection.rs:1101`

These were intended to deduplicate detections but were removed from the pipeline because they incorrectly merged detections at different locations.

### The Issue

The identifier is computed from:
1. `license_expression` (e.g., "bsd-new")
2. `rule_identifier` (same rule for same license text)
3. `score` (typically 100% for hash matches)
4. `matched_text_tokens` (identical for same license text)

For a file like `bsd-new_and_mit.txt` with BSD-new license at two locations (lines 27-48 and 81-102), both detections have:
- Same `license_expression`: "bsd-new"
- Same `rule_identifier`: "bsd-new.RULE" or similar
- Same `score`: 100%
- Same `matched_text_tokens`: the BSD license text (copyright holder lines are variable)

This produces the **same identifier**, causing incorrect deduplication.

### What Python Does

Python has TWO separate concepts:

1. **`LicenseDetection`** - A detection at a single location
   - Has `file_region` (start_line, end_line)
   - Has `identifier` computed from expression + content hash
   - Multiple detections can have the same identifier if same license appears multiple times

2. **`UniqueDetection`** - An aggregated view for output
   - Groups all detections with the same identifier
   - Has `file_regions`: list of all locations where this license appears
   - Has `detection_count`: number of locations
   - Has single `license_expression`, `matches`, etc.

Python's `get_unique_detections()` does NOT remove detections - it **aggregates** them:

```python
class UniqueDetection:
    identifier = attr.ib(default=None)
    license_expression = attr.ib(default=None)
    license_expression_spdx = attr.ib(default=None)
    detection_count = attr.ib(default=None)      # Number of locations
    matches = attr.ib(default=attr.Factory(list))
    detection_log = attr.ib(default=attr.Factory(list))
    file_regions = attr.ib(factory=list)         # All locations
```

### Key Insight

The Python test infrastructure uses `idx.match()` which returns **raw matches** without any deduplication. The golden tests compare these raw matches.

For output formatting (ScanCode JSON output), Python uses `UniqueDetection.get_unique_detections()` to create the final output with aggregated `file_regions`.

## Implementation Plan

### Phase 1: Define UniqueDetection Struct

```rust
/// A unique license detection, aggregating all detections with the same identifier.
/// 
/// Multiple detections of the same license at different locations are grouped
/// into a single UniqueDetection with all file_regions preserved.
pub struct UniqueDetection {
    /// Unique identifier for this detection (license expression + content hash)
    pub identifier: String,
    /// License expression (ScanCode format)
    pub license_expression: Option<String>,
    /// License expression (SPDX format)
    pub license_expression_spdx: Option<String>,
    /// Number of locations where this license was detected
    pub detection_count: usize,
    /// All matches contributing to this detection
    pub matches: Vec<LicenseMatch>,
    /// Diagnostic log entries
    pub detection_log: Vec<String>,
    /// All file regions where this license appears
    pub file_regions: Vec<FileRegion>,
}
```

### Phase 2: Rename Existing Functions

1. Rename `remove_duplicate_detections` → `get_unique_detections`
2. Update to return `Vec<UniqueDetection>` instead of `Vec<LicenseDetection>`
3. Properly aggregate `file_regions` from all detections with same identifier

```rust
pub fn get_unique_detections(detections: Vec<LicenseDetection>) -> Vec<UniqueDetection> {
    let mut detections_by_id: HashMap<String, Vec<LicenseDetection>> = HashMap::new();
    
    // Group by identifier
    for detection in detections {
        let identifier = detection.identifier.clone()
            .unwrap_or_else(|| compute_detection_identifier(&detection));
        detections_by_id.entry(identifier).or_default().push(detection);
    }
    
    // Create UniqueDetection from each group
    detections_by_id.into_iter().map(|(identifier, group)| {
        let first = group.first().unwrap();
        let file_regions: Vec<FileRegion> = group.iter()
            .filter_map(|d| d.file_region.clone())
            .collect();
        
        UniqueDetection {
            identifier,
            license_expression: first.license_expression.clone(),
            license_expression_spdx: first.license_expression_spdx.clone(),
            detection_count: file_regions.len(),
            matches: first.matches.clone(),
            detection_log: first.detection_log.clone(),
            file_regions,
        }
    }).collect()
}
```

### Phase 3: Integration Points

The `get_unique_detections` function should be called:

1. **For JSON output** - In the CLI output formatter, convert `Vec<LicenseDetection>` to `Vec<UniqueDetection>` for the final JSON output
2. **NOT in `post_process_detections`** - Keep raw detections for golden tests and internal processing

### Phase 4: Output Format Compatibility

Update JSON output to match Python's format:
- `detection_count` field
- `file_regions` array instead of single `file_region`
- Serialization matches Python's `to_dict()` behavior

## Files to Modify

| File | Changes |
|------|---------|
| `src/license_detection/detection.rs` | Add `UniqueDetection` struct, rename/reimplement function |
| `src/license_detection/mod.rs` | Export `UniqueDetection` |
| `src/main.rs` or output module | Call `get_unique_detections` for JSON output |

## Verification

1. Golden tests should continue to pass (they test raw `LicenseDetection`)
2. JSON output should match Python's format with `file_regions` and `detection_count`
3. Clippy warnings for dead code should be resolved

## Success Criteria

- [ ] `UniqueDetection` struct defined with all required fields
- [ ] `get_unique_detections` properly aggregates `file_regions`
- [ ] JSON output matches Python format
- [ ] No clippy warnings for dead code
- [ ] All golden tests pass (131 baseline failures)
- [ ] Manual verification with debug scripts shows correct aggregation

## References

- Python `UniqueDetection`: `reference/scancode-toolkit/src/licensedcode/detection.py:896`
- Python `get_unique_detections`: `reference/scancode-toolkit/src/licensedcode/detection.py:918`
- Current Rust dead code: `src/license_detection/detection.rs:934, 1101`
