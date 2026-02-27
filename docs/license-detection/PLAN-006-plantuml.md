# PLAN-006: plantuml_license_notice.txt

## Status: OPEN - READY TO IMPLEMENT

## Test File
`testdata/license-golden/datadriven/lic4/plantuml_license_notice.txt`

## Issue
Expression wrapped in extra parentheses.

**Expected:** `["mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus"]`
**Actual:** `["(mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus)"]`

## Root Cause

The rule file `reference/scancode-toolkit/src/licensedcode/data/rules/plantuml_1.RULE` contains:
```yaml
license_expression: (mit OR apache-2.0 OR epl-2.0 OR lgpl-3.0-plus OR gpl-3.0-plus)
```

- **Rust** stores the expression as-is from the YAML file in `rule.license_expression`
- **Python** normalizes expressions during rule loading via `licensing.parse().render()`

## Previous Fix Attempt (Caused Regression)

Normalizing expression using parser at load time in `loader.rs`:
```rust
match parse_expression(&expr) {
    Ok(parsed) => expression_to_string(&parsed),
    Err(_) => expr,
}
```

**This caused a regression in `sencha-touch.txt`:**
- Expected: `(gpl-3.0 WITH sencha-app-floss-exception OR ...) AND (public-domain AND mit AND mit)`
- Actual: `(...) AND public-domain AND mit AND mit` (lost stylistic parens)

## Root Cause of Regression

The `expression_to_string()` function produces **semantically correct** output but loses **stylistic parentheses**:

| Expression | Trivial Outer Parens | Semantic/Grouping Parens |
|------------|---------------------|-------------------------|
| `(mit OR apache-2.0)` | YES - entire expression wrapped | NO |
| `(a OR b) AND (c OR d)` | NO | YES - required for grouping |
| `(gpl-3.0 WITH exception) OR mit` | NO | NO - stylistic only |

The parser's `expression_to_string()` correctly:
- Removes trivial outer parens: `(mit OR apache)` → `mit OR apache`
- Preserves semantic grouping: `(a OR b) AND c` → `(a OR b) AND c`

But it loses **stylistic parens** that aren't semantically required:
- `(gpl-3.0 WITH exception) OR mit` → `gpl-3.0 WITH exception OR mit`
- `(public-domain AND mit AND mit)` → `public-domain AND mit AND mit`

## The Correct Fix

**Key Insight:** Use a **string-level heuristic** to detect and remove ONLY trivial outer parentheses. This preserves ALL other expressions unchanged, including those with stylistic parens.

### Algorithm: `has_trivial_outer_parens(expr: &str) -> bool`

Returns `true` ONLY when the entire expression is wrapped in a single pair of parens:

```rust
fn has_trivial_outer_parens(s: &str) -> bool {
    let trimmed = s.trim();
    if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
        return false;
    }
    let mut depth = 0;
    let chars: Vec<char> = trimmed.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if *c == '(' {
            depth += 1;
        } else if *c == ')' {
            depth -= 1;
            // If we close the outer parens before the end, 
            // there are multiple top-level expressions
            if depth == 0 && i < chars.len() - 1 {
                return false;
            }
        }
    }
    depth == 0
}
```

### Exact Implementation Location

**File:** `src/license_detection/rules/loader.rs`
**Line:** 286-295 (the `license_expression` assignment)

**Current code:**
```rust
let license_expression = match fm.license_expression {
    Some(expr) => expr,
    None if is_false_positive => "unknown".to_string(),
    None => {
        return Err(anyhow!(
            "Rule file missing required field 'license_expression': {}",
            path.display()
        ));
    }
};
```

**New code:**
```rust
let license_expression = match fm.license_expression {
    Some(expr) => normalize_trivial_outer_parens(&expr),
    None if is_false_positive => "unknown".to_string(),
    None => {
        return Err(anyhow!(
            "Rule file missing required field 'license_expression': {}",
            path.display()
        ));
    }
};
```

**Add these helper functions at the end of the file (before any test module):**

```rust
fn has_trivial_outer_parens(s: &str) -> bool {
    let trimmed = s.trim();
    if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
        return false;
    }
    let mut depth = 0;
    let chars: Vec<char> = trimmed.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if *c == '(' {
            depth += 1;
        } else if *c == ')' {
            depth -= 1;
            if depth == 0 && i < chars.len() - 1 {
                return false;
            }
        }
    }
    depth == 0
}

fn normalize_trivial_outer_parens(expr: &str) -> String {
    let trimmed = expr.trim();
    if has_trivial_outer_parens(trimmed) {
        let inner = &trimmed[1..trimmed.len()-1];
        normalize_trivial_outer_parens(inner)
    } else {
        expr.to_string()
    }
}
```

### Why This Works (No Regression)

| Expression | `has_trivial_outer_parens` | Result |
|------------|---------------------------|--------|
| `(mit OR apache-2.0)` | `true` | `mit OR apache-2.0` |
| `((mit OR apache-2.0))` | `true` (recursive) | `mit OR apache-2.0` |
| `(a OR b) AND (c OR d)` | `false` | unchanged |
| `(gpl-3.0 WITH exception) OR mit` | `false` | unchanged |
| `(gpl-3.0...sencha-commercial) AND (public-domain AND mit AND mit)` | `false` | unchanged |

**Sencha expression analysis:**
- Input: `(gpl-3.0 WITH sencha-app-floss-exception OR gpl-3.0 WITH sencha-dev-floss-exception OR sencha-commercial) AND (public-domain AND mit AND mit)`
- First `(` closes after `sencha-commercial)` at position 98
- This is NOT the last position (still have ` AND (public-domain AND mit AND mit)`)
- So `has_trivial_outer_parens` returns `false`
- Expression remains **unchanged** - no regression!

## Investigation Tests

`src/license_detection/investigation/plantuml_test.rs`

### Passing Tests (Current Behavior Correct)

- `test_expression_parse_normalizes_outer_parens` - Parser removes trivial parens
- `test_semantically_required_grouping_preserved` - `(a OR b) AND c` preserved
- `test_stylistic_parens_lost_by_parser` - Documents current limitation
- `test_is_trivial_outer_parens_heuristic` - Validates the heuristic

### Failing Tests (Will Pass After Fix)

- `test_plantuml_expression_no_extra_parens` - End-to-end test for plantuml
- `test_plantuml_rule_expression_has_extra_parens` - Rule loading test

## Key Files

- `src/license_detection/rules/loader.rs:286-295` - Rule loading (fix location)
- `src/license_detection/expression.rs` - Expression parsing (DO NOT MODIFY for this fix)
- `src/license_detection/investigation/plantuml_test.rs` - Tests

## Verification Steps

1. **Run plantuml tests to verify fix:**
   ```bash
   cargo test plantuml --lib
   ```
   Expected: All 11 tests pass (currently 2 fail)

2. **Run sencha tests to verify no regression:**
   ```bash
   cargo test sencha --lib
   ```
   Expected: All tests pass with same behavior

3. **Run golden test for plantuml:**
   ```bash
   cargo test plantuml_license_notice --lib
   ```

4. **Run full license detection tests:**
   ```bash
   cargo test license_detection --lib
   ```

## Potential Regressions and Mitigations

### Risk 1: Expressions with nested parens at end

**Example:** `(mit AND (apache OR gpl))`
**Analysis:** First `(` closes at position 20, NOT at end → returns `false` → unchanged

### Risk 2: Empty or malformed expressions

**Mitigation:** The function handles:
- Empty strings: `starts_with('(')` returns `false`
- Unbalanced parens: Final `depth == 0` check fails

### Risk 3: Multi-line YAML expressions

**Analysis:** YAML parser joins multi-line values before our function sees them. The `trim()` handles leading/trailing whitespace.

## Implementation Checklist

- [ ] Add `has_trivial_outer_parens()` function to `loader.rs`
- [ ] Add `normalize_trivial_outer_parens()` function to `loader.rs`
- [ ] Apply normalization when loading `license_expression` from YAML
- [ ] Run `cargo test plantuml --lib` - all 11 tests pass
- [ ] Run `cargo test sencha --lib` - no regression
- [ ] Run `cargo test license_detection --lib` - all pass
- [ ] Run `cargo clippy --all-targets` - no warnings
