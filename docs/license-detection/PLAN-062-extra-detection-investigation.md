# PLAN-062: Extra Detections Investigation

## Status: NEEDS INVESTIGATION

## Problem Statement

Additional unexpected license expressions are detected. Expected N expressions, got N+1 or more.

### Representative Test Case

**File**: `testdata/license-golden/datadriven/lic1/gfdl-1.1-en_gnome_1.RULE`

| Expected | Actual |
|----------|--------|
| `["gfdl-1.1", "gfdl-1.1-plus"]` | `["gfdl-1.1", "other-copyleft", "other-copyleft", "gfdl-1.1-plus", "gfdl-1.1", "gfdl-1.1", "gfdl-1.1-plus", "gfdl-1.1-plus", "gfdl-1.3-no-invariants-only", ...]` (12 total) |

---

## Investigation Instructions

### Step 1: Create Investigation Test File

Create `src/license_detection/extra_detection_parity_investigation_test.rs`:

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
    fn test_gfdl_11_gnome_parity() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file("gfdl-1.1-en_gnome_1.RULE") else { return };

        // TODO: Add step-by-step assertions here
        
        let detections = engine.detect(&text).expect("Detection should succeed");
        
        // Final assertion - expect only 2 detections
        assert_eq!(detections.len(), 2);
    }
}
```

### Step 2: Run Python Reference to Get Baseline Data

Use the playground:
```bash
cd reference/scancode-playground && venv/bin/python src/scancode/cli.py
```

Extract intermediate data:
1. All matches created - which rule identifiers?
2. Which matches survive filtering?
3. How are matches grouped into detections?
4. What prevents extra matches in Python?

### Step 3: Add Step-by-Step Assertions

```rust
// Example: Verify match count after refine
#[test]
fn test_gfdl_11_refined_matches() {
    // ...setup...
    let refined = /* get refined matches */;
    
    // From Python: expect exactly 2 matches
    assert_eq!(refined.len(), 2, "Should have exactly 2 matches after refine");
}
```

### Step 4: Find Divergence Point

The first failing assertion identifies where Rust creates extra matches that Python filters out.

### Step 5: Document Findings

---

## Key Questions

1. Are extra matches created from the start, or created by incorrect merging?
2. Does Python have additional filtering for GFDL rules?
3. Are `other-copyleft` and `gfdl-1.3-no-invariants-only` matching incorrectly?

---

## Key Files

| Rust File | Python File | Purpose |
|-----------|-------------|---------|
| `src/license_detection/match_refine.rs` | `licensedcode/match.py` | Filter logic |
| `src/license_detection/seq_match.rs` | `licensedcode/index.py` | Match creation |

---

## Success Criteria

1. Identify where extra matches are created or not filtered
2. Document root cause
3. Implement fix
4. All 8 extra detection tests pass
