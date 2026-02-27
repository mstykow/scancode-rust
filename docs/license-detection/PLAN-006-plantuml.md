# PLAN-006: plantuml_license_notice.txt

## Status: FOUND_ROOT_CAUSE

## Test File
`testdata/license-golden/datadriven/lic4/plantuml_license_notice.txt`

## Issue
Expression wrapped in extra parentheses.

**Expected:** `["mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus"]`
**Actual:** `["(mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus)"]`

## Root Cause

The divergence occurs in the **rule loading phase**:

1. **Rule file** `plantuml_1.RULE` contains the expression with outer parentheses:
   ```yaml
   license_expression: (mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus)
   ```

2. **Rust** stores the expression as-is from the YAML file in `rule.license_expression`

3. **Python** normalizes the expression during rule loading, removing unnecessary outer parentheses

4. When the rule matches via hash, the match's `license_expression` is copied directly from the rule

## Divergence Point

**Location:** `src/license_detection/rules/loader.rs:286-295`

```rust
let license_expression = match fm.license_expression {
    Some(expr) => expr,  // <-- NO NORMALIZATION HAPPENS HERE
    None if is_false_positive => "unknown".to_string(),
    None => {
        return Err(anyhow!(
            "Rule file missing required field 'license_expression': {}",
            path.display()
        ));
    }
};
```

## Fix Required

Normalize the `license_expression` when loading rules by parsing and re-rendering:

```rust
use crate::license_detection::expression::{parse_expression, expression_to_string};

let license_expression = match fm.license_expression {
    Some(expr) => {
        // Normalize expression (remove unnecessary outer parentheses)
        match parse_expression(&expr) {
            Ok(parsed) => expression_to_string(&parsed),
            Err(_) => expr, // Fall back to original if parse fails
        }
    }
    // ...
};
```

## Investigation Tests

Located at: `src/license_detection/investigation/plantuml_test.rs`

Key failing tests:
- `test_plantuml_rule_expression_has_extra_parens` - Shows the rule has parens
- `test_expression_parse_normalizes_outer_parens` - Shows parser DOES normalize

The parser already handles normalization correctly. The issue is that the rule's 
`license_expression` field is never passed through the parser during loading.

## Related Code

- `src/license_detection/rules/loader.rs:286-295` - Rule loading (NEEDS FIX)
- `src/license_detection/expression.rs:548-592` - Expression normalization (WORKS)
- `src/license_detection/hash_match.rs` - Copies expression from rule to match
