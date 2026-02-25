# PLAN-060: CDDL Rule Selection Parity Investigation

## Status: NEEDS INVESTIGATION

## Problem Statement

CDDL 1.0 test files are incorrectly matching CDDL 1.1 rules. Rust diverges from Python in CDDL rule selection.

### Representative Test Case

**File**: `testdata/license-golden/datadriven/lic1/cddl-1.0_or_gpl-2.0-glassfish.txt`

| Expected | Actual |
|----------|--------|
| `["cddl-1.0 OR gpl-2.0"]` | `["cddl-1.1 OR gpl-2.0 WITH classpath-exception-2.0"]` |

---

## Investigation Instructions

### Step 1: Create Investigation Test File

Create `src/license_detection/cddl_parity_investigation_test.rs`:

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
    fn test_cddl_10_glassfish_parity() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file("cddl-1.0_or_gpl-2.0-glassfish.txt") else { return };

        // TODO: Add step-by-step assertions here
        // Each assertion should verify intermediate data matches Python
        
        let detections = engine.detect(&text).expect("Detection should succeed");
        
        // Final assertion
        assert_eq!(
            detections.first().and_then(|d| d.license_expression.as_ref()),
            Some(&"cddl-1.0 OR gpl-2.0".to_string())
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
1. Token sequences
2. Phase 1 (hash) matches
3. Phase 2 (near-duplicate) matches  
4. Phase 3 (seq) matches
5. After merge_overlapping_matches()
6. After filter_contained_matches()
7. After filter_overlapping_matches()
8. After refine_matches()
9. Final detections

### Step 3: Add Step-by-Step Assertions

For each pipeline step, add an assertion to the test that verifies Rust's intermediate data matches Python's:

```rust
// Example: After merge_overlapping_matches
#[test]
fn test_cddl_10_after_merge() {
    // ...setup...
    let matches_after_merge = /* get matches after merge */;
    
    // Expected data from Python
    let expected_match_count = 42;  // From Python run
    let expected_cddl10_rid = Some(1234);  // From Python run
    
    assert_eq!(matches_after_merge.len(), expected_match_count);
    assert!(matches_after_merge.iter().any(|m| m.rid == expected_cddl10_rid));
}
```

### Step 4: Find Divergence Point

Run each test incrementally until the assertion fails. The first failing test is the divergence point.

### Step 5: Document Findings

Update this plan with:
- Exact file and line where divergence occurs
- What Python does differently
- Proposed fix

---

## Key Files to Investigate

| Rust File | Python File | Purpose |
|-----------|-------------|---------|
| `src/license_detection/mod.rs` | `licensedcode/index.py` | Main detection pipeline |
| `src/license_detection/match_refine.rs` | `licensedcode/match.py` | Match refinement |
| `src/license_detection/models.rs` | `licensedcode/models.py` | Match data structures |

---

## Success Criteria

1. Identify exact divergence point between Rust and Python
2. Document root cause
3. Implement fix that achieves parity
4. All 8 CDDL tests pass
