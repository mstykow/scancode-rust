# Rule Engine Audit: Python vs Rust Comparison

This document compares the rule loading and threshold handling between Python ScanCode Toolkit and the Rust implementation.

**Python Reference**: `reference/scancode-toolkit/src/licensedcode/models.py`
**Rust Implementation**: `src/license_detection/rules/`, `src/license_detection/models/rule.rs`

---

## 1. Rule File Format (.RULE and .LICENSE Files)

### Python Implementation

**Location**: `reference/scancode-toolkit/src/licensedcode/frontmatter.py`

**Frontmatter Parsing**:
- Uses `saneyaml` for YAML parsing
- Delimiter: `---` (3 or more dashes matched by regex `r"^-{3,}\s*$"`)
- Split into 3 parts: before first `---`, YAML content, text content
- Line 49-54: `split()` method uses `FM_BOUNDARY.split(text, 2)`

**File Structure**:
```
---
key: mit                    # YAML frontmatter
short_name: MIT License
name: MIT License
---
License text here           # Plain text content
```

**Code Reference**: `frontmatter.py:49-54`

### Rust Implementation

**Location**: `src/license_detection/rules/loader.rs`

**Frontmatter Parsing**:
- Uses `serde_yaml` for YAML parsing
- Delimiter regex: `r"(?m)^-{3,}\s*$"` (multi-line, 3+ dashes)
- Split into 3 parts using `splitn(&content, 3)`
- Lines 237-261: Parses YAML and text content separately

**Key Differences**:
1. **Error Handling**: Rust returns `Result<Rule>` with detailed error messages; Python raises exceptions
2. **Type Safety**: Rust uses strongly-typed `RuleFrontmatter` struct (lines 150-221); Python uses dynamic attribute assignment
3. **Boolean Parsing**: Rust supports `yes/no/true/false/1/0` via `deserialize_yes_no_bool` (lines 16-40); Python uses saneyaml's native handling

**Code Reference**: `loader.rs:223-346` (`parse_rule_file`)

---

## 2. Rule Attributes (is_license_text, is_license_notice, etc.)

### Python Implementation

**Location**: `reference/scancode-toolkit/src/licensedcode/models.py:1309-1453`

**Attributes** (BasicRule class):
| Attribute | Default | Description |
|-----------|---------|-------------|
| `is_license_text` | `False` | Full license text (highest confidence) |
| `is_license_notice` | `False` | Explicit notice like "Licensed under MIT" |
| `is_license_reference` | `False` | Reference like bare name or URL |
| `is_license_tag` | `False` | Structured tag (SPDX identifier) |
| `is_license_intro` | `False` | Introductory statement before license |
| `is_license_clue` | `False` | Weak clue, not proper detection |
| `is_false_positive` | `False` | Exact matches are false positives |
| `is_required_phrase` | `False` | Required phrase marker |

**Mutual Exclusivity**: `models.py:1360-1363` documents that `is_license_*` flags are mutually exclusive

**Validation**: `models.py:1970-1974` checks only one `is_license_*` flag can be true

### Rust Implementation

**Location**: `src/license_detection/models/rule.rs:24-48`

**Attributes**: Same boolean flags with identical semantics

**Key Differences**:
1. **Type Safety**: Rust uses `bool` type; Python uses `attr.ib(default=False)`
2. **No Validation at Load Time**: Rust does not validate mutual exclusivity during parsing; Python validates in `validate()` method
3. **Field Ordering**: Rust struct has explicit field order for sorting compatibility

---

## 3. Threshold Computation (minimum_coverage, relevance)

### Python Implementation

**Location**: `reference/scancode-toolkit/src/licensedcode/models.py:2369-2711`

**Constants** (from `licensedcode/__init__.py`):
- `MIN_MATCH_LENGTH = 4`
- `MIN_MATCH_HIGH_LENGTH = 3`
- `SMALL_RULE = 15`
- `TINY_RULE = 6`

**`compute_thresholds_occurrences()`** (lines 2628-2668):
```python
if minimum_coverage == 100:
    return minimum_coverage, length, high_length

if length < 3:
    # Tiny: 100% coverage required
    min_matched_length = length
    min_high_matched_length = high_length
    minimum_coverage = 100

elif length < 10:
    # Small: 80% coverage
    min_matched_length = length
    min_high_matched_length = high_length
    minimum_coverage = 80

elif length < 30:
    # Medium: 50% coverage
    min_matched_length = length // 2
    min_high_matched_length = min(high_length, MIN_MATCH_HIGH_LENGTH)
    minimum_coverage = 50

elif length < 200:
    # Large: use MIN_MATCH_LENGTH
    min_matched_length = MIN_MATCH_LENGTH
    min_high_matched_length = min(high_length, MIN_MATCH_HIGH_LENGTH)

else:  # length >= 200
    # Very large: 10% coverage
    min_matched_length = length // 10
    min_high_matched_length = high_length // 10
```

**`compute_relevance()`** (lines 2573-2625):
```python
if length > 18:
    return 100
return {
    0: 0, 1: 5, 2: 11, 3: 16, 4: 22, 5: 27,
    6: 33, 7: 38, 8: 44, 9: 50, 10: 55, 11: 61,
    12: 66, 13: 72, 14: 77, 15: 83, 16: 88,
    17: 94, 18: 100,
}[length]
```

**`compute_thresholds_unique()`** (lines 2671-2711):
```python
if minimum_coverage == 100:
    return length_unique, high_length_unique

if length > 200:
    min_matched_length_unique = length // 10
    min_high_matched_length_unique = high_length_unique // 10

elif length < 5:
    min_matched_length_unique = length_unique
    min_high_matched_length_unique = high_length_unique

elif length < 10:
    if length_unique < 2:
        min_matched_length_unique = length_unique
    else:
        min_matched_length_unique = length_unique - 1
    min_high_matched_length_unique = high_length_unique

elif length < 20:
    min_matched_length_unique = high_length_unique
    min_high_matched_length_unique = high_length_unique

else:
    min_matched_length_unique = MIN_MATCH_LENGTH
    highu = (int(high_length_unique // 2)) or high_length_unique
    min_high_matched_length_unique = min(highu, MIN_MATCH_HIGH_LENGTH)
```

### Rust Implementation

**Location**: `src/license_detection/rules/thresholds.rs`

**Constants** (lines 4-13):
```rust
pub const MIN_MATCH_LENGTH: usize = 4;
pub const MIN_MATCH_HIGH_LENGTH: usize = 3;
pub const SMALL_RULE: usize = 15;
pub const TINY_RULE: usize = 6;
```

**`compute_thresholds_occurrences()`** (lines 29-59):
- **IDENTICAL** logic to Python
- Uses `Option<u8>` for minimum_coverage instead of `int`
- Returns `(Option<u8>, usize, usize)` tuple

**Key Differences**:
1. **No `compute_relevance()`**: Rust does not implement the relevance computation function
2. **No unique threshold computation integration**: `compute_thresholds_unique()` exists but is not called during rule loading
3. **Type safety**: Uses `Option<u8>` instead of int for coverage

---

## 4. Required Phrases ({{...}} Marker Handling)

### Python Implementation

**Location**: `reference/scancode-toolkit/src/licensedcode/tokenize.py:90-213`

**Pattern**:
- Required phrase pattern: `{{...}}` (double curly braces)
- Tokenizer: `required_phrase_tokenizer()` yields `{{` and `}}` as separate tokens
- Span extraction: `get_existing_required_phrase_spans()` returns list of `Span` objects

**Example Rule** (`zlib_4.RULE`):
```
{{zlib/libpng license}}

This software is provided 'as-is'...
```

**Span Calculation** (`tokenize.py:122-174`):
```python
def get_existing_required_phrase_spans(text):
    # Yields Span objects for each {{phrase}}
    # Position tracking excludes {{ and }} markers
    # Raises InvalidRuleRequiredPhrase for malformed markers
```

**Validation**:
- Nested `{{` is an error: `InvalidRuleRequiredPhrase`
- Empty `{{}}` is an error
- Unclosed `{{` is an error

**Usage in Rule** (`models.py:2356-2367`):
```python
def build_required_phrase_spans(self):
    if self.is_from_license:
        return []
    return get_existing_required_phrase_spans(self.text)

def _set_continuous(self):
    # If entire rule is one required phrase, set is_continuous=True
    if (not self.is_continuous
        and self.required_phrase_spans
        and len(self.required_phrase_spans) == 1
        and len(self.required_phrase_spans[0]) == self.length):
        self.is_continuous = True
```

### Rust Implementation

**Location**: `src/license_detection/models/rule.rs:62-68`

**Fields**:
```rust
pub required_phrase_spans: Vec<Range<usize>>,
pub stopwords_by_pos: HashMap<usize, usize>,
```

**Status**: 
- **NOT IMPLEMENTED** - Field exists but is always initialized as empty `vec![]`
- No tokenizer for `{{...}}` markers
- No span extraction logic

**Potential Behavioral Differences**:
1. Rules with required phrases will not have `is_continuous` set automatically
2. Required phrase validation is not performed during loading
3. Match quality may differ for rules with `{{...}}` markers

---

## 5. Rule Flags (is_false_positive, is_license_clue, etc.)

### Python Implementation

**Location**: `reference/scancode-toolkit/src/licensedcode/models.py:1441-1453`

**`is_false_positive` Rules**:
- Must have `notes` field (validation at line 1933)
- Cannot have `is_license_*` flags (line 1939)
- Cannot have `license_expression` (line 1942)
- Cannot have `referenced_filenames` (line 1948)
- Cannot have `ignorable_*` attributes (line 1951)
- Always has `relevance = 100` (line 2528)

**`is_license_clue` Rules**:
- Not included in license expression summaries
- Used for detecting licensing hints but not actual licenses

### Rust Implementation

**Location**: `src/license_detection/rules/loader.rs:284-295`

**Handling**:
```rust
let is_false_positive = fm.is_false_positive.unwrap_or(false);

let license_expression = match fm.license_expression {
    Some(expr) => normalize_trivial_outer_parens(&expr),
    None if is_false_positive => "unknown".to_string(),
    None => return Err(anyhow!("Missing license_expression")),
};
```

**Validation**: `loader.rs:560-583` (`validate_rules`):
```rust
fn validate_rules(rules: &[Rule]) {
    // Warns on duplicate rule texts
    // Warns on empty license_expression for non-false-positive
}
```

**Key Differences**:
1. **False Positive Expression**: Rust assigns `"unknown"` as expression for false positives; Python requires `None`
2. **Validation Depth**: Python has comprehensive validation in `validate()` method; Rust only has basic checks
3. **No Runtime Checks**: Python validates during `setup()`; Rust only warns at load time

---

## 6. Frontmatter Parsing (YAML Header Extraction)

### Python Implementation

**Location**: `reference/scancode-toolkit/src/licensedcode/frontmatter.py:25-84`

**Handler**: `SaneYAMLHandler` class
- Uses `saneyaml.load(fm, allow_duplicate_keys=False)` 
- Preserves key order
- Returns `dict` for metadata

**Line Normalization** (`frontmatter.py:96-97`):
```python
text = text.replace("\r\n", "\n")
```

### Rust Implementation

**Location**: `src/license_detection/rules/loader.rs:13-66`

**Parser**: `serde_yaml`
- Strongly-typed deserialization into `RuleFrontmatter` / `LicenseFrontmatter`
- No key order preservation (uses struct field order)
- Returns `Result<Rule>` with detailed errors

**Boolean Parsing** (`loader.rs:16-40`):
```rust
enum YesNoOrBool {
    String(String),
    Bool(bool),
}
// Accepts: "yes", "no", "true", "false", "1", "0"
```

**Number Parsing** (`loader.rs:42-66`):
```rust
fn as_u8(&self) -> Option<u8> {
    // Handles both integer and float YAML numbers
    // Returns None if out of range
}
```

**Key Differences**:
1. **Duplicate Keys**: Python explicitly disallows duplicate keys; Rust's serde_yaml behavior may differ
2. **Line Endings**: Python normalizes `\r\n` to `\n`; Rust reads file as-is
3. **Error Context**: Rust provides YAML content in error messages; Python prints stack trace

---

## 7. Additional Fields Comparison

### Python Fields Not in Rust

| Field | Python Location | Description | Status |
|-------|-----------------|-------------|--------|
| `rid` | `models.py:1317` | Internal rule ID | Not needed (uses Vec index) |
| `license_expression_object` | `models.py:1343` | Parsed expression object | Not implemented |
| `has_stored_relevance` | `models.py:1542` | Track if relevance was stored | Not implemented |
| `has_stored_minimum_coverage` | `models.py:1500` | Track if coverage was stored | Not implemented |
| `_minimum_containment` | `models.py:1509` | Cached coverage / 100 | Not implemented |
| `skip_for_required_phrase_generation` | `models.py:1466` | Skip phrase collection | Not implemented |
| `source` | `models.py:1686` | Rule source identifier | Not implemented |
| `has_computed_thresholds` | `models.py:1779` | Track threshold computation | Not implemented |

### Rust Fields Not in Python

| Field | Rust Location | Description |
|-------|---------------|-------------|
| `tokens` | `rule.rs:22` | Token IDs (assigned during indexing) |
| `spdx_license_key` | `rule.rs:131` | SPDX identifier (from License, not Rule) |

---

## 8. Summary of Potential Behavioral Differences

### High Priority

1. **Required Phrase Handling** (CRITICAL)
   - Python: Extracts `{{...}}` markers and sets `is_continuous` automatically
   - Rust: Not implemented, `required_phrase_spans` always empty
   - **Impact**: Rules with required phrases may match incorrectly

2. **Relevance Computation** (HIGH)
   - Python: `compute_relevance()` assigns 0-100 based on rule length
   - Rust: Not implemented, always returns stored or default value
   - **Impact**: Match scoring and ranking may differ

3. **False Positive Expression** (MEDIUM)
   - Python: False positives must NOT have `license_expression`
   - Rust: False positives get `"unknown"` as expression
   - **Impact**: Downstream handling of false positives may differ

### Medium Priority

4. **Validation Depth** (MEDIUM)
   - Python: Comprehensive validation including mutual exclusivity checks
   - Rust: Only basic validation (duplicate texts, empty expression)
   - **Impact**: Invalid rules may load successfully in Rust

5. **Threshold Integration** (MEDIUM)
   - Python: Calls both `compute_thresholds_occurrences` and `compute_thresholds_unique`
   - Rust: Only `compute_thresholds_occurrences` is implemented
   - **Impact**: Unique token thresholds not used

### Low Priority

6. **Line Ending Normalization** (LOW)
   - Python: Normalizes `\r\n` to `\n`
   - Rust: No normalization
   - **Impact**: May affect cross-platform behavior

7. **Key Order Preservation** (LOW)
   - Python: Preserves YAML key order via saneyaml
   - Rust: Uses struct field order
   - **Impact**: Serialization output order differs, semantic equivalence preserved

---

## 9. Recommendations

### Must Implement

1. **Required Phrase Tokenizer**: Port `get_existing_required_phrase_spans()` from `tokenize.py:122-174`
2. **Relevance Computation**: Port `compute_relevance()` from `models.py:2573-2625`
3. **Rule Validation**: Add comprehensive validation matching Python's `validate()` method

### Should Implement

4. **Unique Threshold Integration**: Integrate `compute_thresholds_unique()` into rule loading
5. **Continuous Flag Logic**: Port `_set_continuous()` from `models.py:2343-2354`

### Consider

6. **False Positive Expression Handling**: Align with Python behavior (no expression for false positives)
7. **Line Ending Normalization**: Add `\r\n` to `\n` normalization for cross-platform consistency

---

## 10. File Reference Summary

| Component | Python File | Rust File |
|-----------|-------------|-----------|
| Rule Model | `licensedcode/models.py:2261-2513` | `license_detection/models/rule.rs` |
| Frontmatter | `licensedcode/frontmatter.py` | `license_detection/rules/loader.rs:13-66` |
| Rule Loader | `licensedcode/models.py:2424-2512` | `license_detection/rules/loader.rs:223-346` |
| Thresholds | `licensedcode/models.py:2369-2711` | `license_detection/rules/thresholds.rs` |
| Required Phrases | `licensedcode/tokenize.py:90-213` | **NOT IMPLEMENTED** |
| Legalese | `licensedcode/legalese.py` | `license_detection/rules/legalese.rs` |
| Index | `licensedcode/index.py` | `license_detection/index/mod.rs` |
