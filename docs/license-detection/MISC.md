# Miscellaneous: License Flags Serialization

This is a low-priority task for future implementation.

## Part 2: License Flags in LicenseMatch

### 1. Complete Inventory of License Flags

Python's `license_flag_names` property defines **6 mutually exclusive flags**:

| Flag | Description | Used in Filtering |
|------|-------------|-------------------|
| `is_license_text` | Full license text (highest confidence) | ✅ Yes - subtract long matches |
| `is_license_notice` | Explicit notice like "Licensed under MIT" | ❌ No |
| `is_license_reference` | Reference like bare name or URL | ❌ No |
| `is_license_tag` | Structured tag (e.g., SPDX identifier) | ❌ No |
| `is_license_intro` | Intro before actual license text | ❌ No |
| `is_license_clue` | Clue but not proper detection | ❌ No |

### 2. Python Match.to_dict() JSON Output

**Critical finding**: Python's `Match.to_dict()` does **NOT** serialize any `is_license_*` flags to JSON.

Only these fields are output:

```
license_expression, license_expression_spdx, from_file, start_line, end_line,
matcher, score, matched_length, match_coverage, rule_relevance,
rule_identifier, rule_url, matched_text (optional)
```

### 3. Comparison Table

| Flag | Python Rule | Python JSON | Rust Rule | Rust LicenseMatch | Rust JSON | Gap |
|------|:-----------:|:-----------:|:---------:|:-----------------:|:---------:|:---:|
| `is_license_text` | ✅ | ❌ | ✅ | ✅ | ✅ | Extra serialization |
| `is_license_notice` | ✅ | ❌ | ✅ | ❌ | ❌ | Missing |
| `is_license_reference` | ✅ | ❌ | ✅ | ✅ | ✅ | Extra serialization |
| `is_license_tag` | ✅ | ❌ | ✅ | ✅ | ✅ | Extra serialization |
| `is_license_intro` | ✅ | ❌ | ✅ | ✅ | ✅ | Extra serialization |
| `is_license_clue` | ✅ | ❌ | ✅ | ✅ | ✅ | Extra serialization |

### 4. Recommended Changes

Add missing flags but **skip serialization** to match Python's behavior exactly.

#### Changes to `src/license_detection/models.rs`

**Add/modify fields in `LicenseMatch`:**

```rust
#[serde(skip)]
pub is_license_text: bool,

#[serde(skip)]
pub is_license_notice: bool,

#[serde(skip)]
pub is_license_intro: bool,

#[serde(skip)]
pub is_license_clue: bool,

#[serde(skip)]
pub is_license_reference: bool,

#[serde(skip)]
pub is_license_tag: bool,
```

### 5. Files to Modify

| File | Changes |
|------|---------|
| `src/license_detection/models.rs` | Add fields, update Default, update tests |
| `src/license_detection/match_refine.rs` | Update ~5 creation sites |
| `src/license_detection/spdx_lid.rs` | Update 1 creation site |
| `src/license_detection/seq_match.rs` | Update 2 creation sites |
| `src/license_detection/unknown_match.rs` | Update 3 creation sites |
| `src/license_detection/detection.rs` | Update ~15 creation sites |
| `src/license_detection/hash_match.rs` | Update 1 creation site |
| `src/license_detection/aho_match.rs` | Update 1 creation site |

### Implementation Checklist

- [ ] Add `is_license_notice` field with `#[serde(skip)]`
- [ ] Change all 6 flags to use `#[serde(skip)]`
- [ ] Update `Default` implementation
- [ ] Update all creation sites (~73 locations)
- [ ] Add serialization test to verify flags not in JSON
