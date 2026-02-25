# PLAN-063: Missing Detections Investigation

## Status: NEEDS INVESTIGATION

## Problem Statement

Expected license expressions are not being detected. Actual count significantly less than expected.

### Representative Test Case

**File**: `testdata/license-golden/datadriven/lic1/e2fsprogs.txt`

| Expected | Actual |
|----------|--------|
| `["gpl-2.0 AND lgpl-2.0 AND bsd-new AND mit-old-style-no-advert", "bsd-new", "lgpl-2.1-plus", "lgpl-2.1-plus", "lgpl-2.1-plus"]` (5) | `["gpl-2.0 AND lgpl-2.0 AND bsd-new AND mit-old-style-no-advert", "bsd-new", "lgpl-2.1-plus", "lgpl-2.1-plus"]` (4) |

Missing: 1 `lgpl-2.1-plus` detection

---

## Investigation Instructions

### Step 1: Create Investigation Test File

Create `src/license_detection/missing_detection_parity_investigation_test.rs`:

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
    fn test_e2fsprogs_parity() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file("e2fsprogs.txt") else { return };

        // TODO: Add step-by-step assertions here
        
        let detections = engine.detect(&text).expect("Detection should succeed");
        
        // Final assertion - expect 5 detections
        assert_eq!(detections.len(), 5);
    }
}
```

### Step 2: Run Python Reference to Get Baseline Data

Use the playground:
```bash
cd reference/scancode-playground && venv/bin/python src/scancode/cli.py
```

Extract intermediate data:
1. All matches created - which lgpl-2.1-plus matches exist?
2. At what position (line numbers) are the 3 lgpl-2.1-plus matches?
3. Which match is being lost in Rust?
4. At what pipeline stage is it lost?

### Step 3: Add Step-by-Step Assertions

```rust
// Example: Verify all lgpl-2.1-plus matches are created
#[test]
fn test_e2fsprogs_lgpl_matches() {
    // ...setup...
    let matches = /* get all matches */;
    
    // From Python: expect 3 lgpl-2.1-plus matches at different positions
    let lgpl_matches: Vec<_> = matches.iter()
        .filter(|m| m.license_expression.contains("lgpl-2.1-plus"))
        .collect();
    assert_eq!(lgpl_matches.len(), 3);
}
```

### Step 4: Find Divergence Point

The first failing assertion identifies where the detection is lost.

### Step 5: Document Findings

---

## Key Questions

1. Is the missing lgpl-2.1-plus match created initially?
2. Is it merged with another match?
3. Is it filtered out? Why?
4. Does Python handle this position differently?

---

## Key Files

| Rust File | Python File | Purpose |
|-----------|-------------|---------|
| `src/license_detection/match_refine.rs` | `licensedcode/match.py` | Filter logic |
| `src/license_detection/detection.rs` | `licensedcode/detection.py` | Detection creation |

---

## Success Criteria

1. Identify where detection is lost
2. Document root cause
3. Implement fix
4. All 12 missing detection tests pass
