# PLAN-061: Duplicate Detections Merged Investigation

## Status: NEEDS INVESTIGATION

## Problem Statement

Multiple license instances in a file are being incorrectly merged into one detection. Expected N expressions, got N-1 or fewer.

### Representative Test Case

**File**: `testdata/license-golden/datadriven/lic1/edl-1.0.txt`

| Expected | Actual |
|----------|--------|
| `["bsd-new", "bsd-new"]` | `["bsd-new"]` |

---

## Investigation Instructions

### Step 1: Create Investigation Test File

Create `src/license_detection/duplicate_parity_investigation_test.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::license_detection::LicenseDetectionEngine;
    use std::path::PathBuf;

    fn get_engine() -> Option<LicenseDetectionEngine> {
        let data_path = PathBuf::from("reference/scancode-toolkit/src/licensedcode/data");
        if !data_path.exists() {
            return None;
        }
        LicenseDetectionEngine::new(&data_path).ok()
    }

    fn read_test_file(name: &str) -> Option<String> {
        let path = PathBuf::from("testdata/license-golden/datadriven/lic1").join(name);
        std::fs::read_to_string(&path).ok()
    }

    #[test]
    fn test_edl_10_duplicate_parity() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file("edl-1.0.txt") else { return };

        // TODO: Add step-by-step assertions here
        // Each assertion should verify intermediate data matches Python
        
        let detections = engine.detect(&text).expect("Detection should succeed");
        
        // Final assertion - expect 2 separate detections
        assert_eq!(detections.len(), 2);
        assert_eq!(
            detections[0].license_expression,
            Some("bsd-new".to_string())
        );
        assert_eq!(
            detections[1].license_expression,
            Some("bsd-new".to_string())
        );
    }
}
```

### Step 2: Run Python Reference to Get Baseline Data

Use the playground:
```bash
cd reference/scancode-playground && venv/bin/python src/scancode/cli.py
```

Modify the playground to extract and print intermediate data at each step:
1. Number of matches after each phase
2. Match positions (start_line, end_line)
3. Which matches are created for each BSD instance
4. After merge_overlapping_matches() - are both matches still there?
5. After filter_contained_matches() - is one removed?
6. After filter_overlapping_matches() - is one removed?
7. Detection creation - how many detections created?

### Step 3: Add Step-by-Step Assertions

For each pipeline step, add an assertion verifying Rust matches Python:

```rust
// Example: Verify both matches exist after seq_match
#[test]
fn test_edl_10_matches_created() {
    // ...setup...
    let seq_matches = /* get seq matches */;
    
    // From Python: expect 2 matches at different positions
    assert_eq!(seq_matches.len(), 2, "Should have 2 matches");
    assert_ne!(seq_matches[0].start_line, seq_matches[1].start_line);
}
```

### Step 4: Find Divergence Point

Run each test incrementally. The first failing test identifies where Rust differs from Python.

### Step 5: Document Findings

Update this plan with:
- Exact file and line where divergence occurs
- What Python does to keep duplicates separate
- Proposed fix

---

## Key Questions

1. Are both BSD instances detected as separate matches initially?
2. At what point are they merged/removed?
3. Does Python use position-based deduplication or something else?
4. Is the merge logic in `merge_overlapping_matches()` or `filter_contained_matches()`?

---

## Key Files to Investigate

| Rust File | Python File | Purpose |
|-----------|-------------|---------|
| `src/license_detection/match_refine.rs` | `licensedcode/match.py` | Merge/filter logic |
| `src/license_detection/detection.rs` | `licensedcode/detection.py` | Detection grouping |

---

## Success Criteria

1. Identify exact divergence point
2. Document root cause
3. Implement fix
4. All 16 duplicate merging tests pass
