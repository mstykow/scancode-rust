# PLAN-064: Wrong Detection Investigation

## Status: NEEDS INVESTIGATION

## Problem Statement

Completely different license expression is detected instead of the expected one.

### Representative Test Case

**File**: `testdata/license-golden/datadriven/lic1/cpl-1.0_in_html.html`

| Expected | Actual |
|----------|--------|
| `["cpl-1.0"]` | `["unknown-license-reference"]` |

---

## Investigation Instructions

### Step 1: Create Investigation Test File

Create `src/license_detection/wrong_detection_parity_investigation_test.rs`:

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
    fn test_cpl_10_html_parity() {
        let Some(engine) = get_engine() else { return };
        let Some(text) = read_test_file("cpl-1.0_in_html.html") else { return };

        // TODO: Add step-by-step assertions here
        
        let detections = engine.detect(&text).expect("Detection should succeed");
        
        // Final assertion
        assert_eq!(detections.len(), 1);
        assert_eq!(
            detections[0].license_expression,
            Some("cpl-1.0".to_string())
        );
    }
}
```

### Step 2: Run Python Reference to Get Baseline Data

Use the playground:
```bash
cd reference/scancode-playground && venv/bin/python src/scancode/cli.py
```

Extract intermediate data:
1. Is cpl-1.0 match created in Python?
2. Is cpl-1.0 match created in Rust?
3. If not, why not? Token mismatch? Rule not matching?
4. Is there an HTML-specific preprocessing difference?

### Step 3: Add Step-by-Step Assertions

```rust
// Example: Verify cpl-1.0 match is created
#[test]
fn test_cpl_10_match_created() {
    // ...setup...
    let matches = /* get all matches */;
    
    // Verify cpl-1.0 rule matches
    let cpl_matches: Vec<_> = matches.iter()
        .filter(|m| m.license_expression.contains("cpl-1.0"))
        .collect();
    assert!(!cpl_matches.is_empty(), "cpl-1.0 match should be created");
}
```

### Step 4: Find Divergence Point

- If cpl-1.0 match is NOT created: investigate matching/tokenization
- If cpl-1.0 match IS created but not in final: investigate filtering/detection

### Step 5: Document Findings

---

## Key Questions

1. Is the CPL 1.0 rule text in the HTML file?
2. Is there HTML-specific tokenization or preprocessing?
3. Why does Rust detect `unknown-license-reference` instead?
4. Is this an HTML parsing issue?

---

## Key Files

| Rust File | Python File | Purpose |
|-----------|-------------|---------|
| `src/license_detection/query.rs` | `licensedcode/index.py` | Query construction |
| `src/utils/text.rs` | `textcode/analysis.py` | HTML handling |

---

## Success Criteria

1. Identify why cpl-1.0 is not detected
2. Document root cause
3. Implement fix
4. All 6 wrong detection tests pass
