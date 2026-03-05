# Detection Assembly and Grouping Logic Audit

**Comparison between Python and Rust implementations**

- **Python reference**: `reference/scancode-toolkit/src/licensedcode/detection.py`
- **Rust implementation**: `src/license_detection/detection/`

## Executive Summary

The Rust implementation closely follows the Python detection assembly logic with good structural parity. Key areas examined: match grouping, detection creation, license expression determination, classification, identifier generation, and file region tracking. Several behavioral differences and potential edge cases have been identified.

---

## 1. Match Grouping

### Python Logic

**File**: `reference/scancode-toolkit/src/licensedcode/detection.py:1820-1868`

```python
def group_matches(license_matches, lines_threshold=LINES_THRESHOLD):
    group_of_license_matches = []
    
    for license_match in license_matches:
        if not group_of_license_matches:
            group_of_license_matches.append(license_match)
            continue
        
        previous_match = group_of_license_matches[-1]
        is_in_group_by_threshold = license_match.start_line <= previous_match.end_line + lines_threshold
        
        if previous_match.rule.is_license_intro:
            group_of_license_matches.append(license_match)
        elif license_match.rule.is_license_intro:
            yield group_of_license_matches
            group_of_license_matches = [license_match]
        elif license_match.rule.is_license_clue:
            yield group_of_license_matches
            yield [license_match]
            group_of_license_matches = []
        elif is_in_group_by_threshold:
            group_of_license_matches.append(license_match)
        else:
            yield group_of_license_matches
            group_of_license_matches = [license_match]
    
    if group_of_license_matches:
        yield group_of_license_matches
```

**Key behaviors**:
1. Groups matches when `start_line <= prev_end_line + 4`
2. Previous intro match: always include next match (ignores threshold)
3. Current match is intro: create NEW group (ignores threshold)
4. Current match is license clue: yield TWO groups (current group + clue as single group)
5. Otherwise: use line threshold check

### Rust Logic

**File**: `src/license_detection/detection/grouping.rs:7-64`

```rust
pub fn group_matches_by_region_with_threshold(
    matches: &[LicenseMatch],
    proximity_threshold: usize,
) -> Vec<DetectionGroup> {
    let mut groups = Vec::new();
    let mut current_group: Vec<LicenseMatch> = Vec::new();
    
    for match_item in matches {
        if current_group.is_empty() {
            current_group.push(match_item.clone());
            continue;
        }
        
        let previous_match = current_group.last().unwrap();
        
        if previous_match.is_license_intro {
            current_group.push(match_item.clone());
        } else if match_item.is_license_intro {
            if !current_group.is_empty() {
                groups.push(DetectionGroup::new(current_group.clone()));
            }
            current_group = vec![match_item.clone()];
        } else if match_item.is_license_clue {
            if !current_group.is_empty() {
                groups.push(DetectionGroup::new(current_group.clone()));
            }
            groups.push(DetectionGroup::new(vec![match_item.clone()]));
            current_group = Vec::new();
        } else if should_group_together(previous_match, match_item, proximity_threshold) {
            current_group.push(match_item.clone());
        } else {
            if !current_group.is_empty() {
                groups.push(DetectionGroup::new(current_group.clone()));
            }
            current_group = vec![match_item.clone()];
        }
    }
    
    if !current_group.is_empty() {
        groups.push(DetectionGroup::new(current_group));
    }
    
    groups
}
```

**should_group_together** (line 76-83):
```rust
pub(super) fn should_group_together(
    prev: &LicenseMatch,
    cur: &LicenseMatch,
    threshold: usize,
) -> bool {
    let line_gap = cur.start_line.saturating_sub(prev.end_line);
    line_gap <= threshold
}
```

### Differences

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| Threshold check | `start_line <= prev_end_line + 4` | `start_line - prev_end_line <= 4` | **IDENTICAL** (same semantics) |
| Empty group check | Implicit (append first match) | Explicit `is_empty()` check | Minor style difference |
| Return type | Generator (`yield`) | `Vec<DetectionGroup>` | Memory tradeoff (Rust collects all at once) |

### Behavioral Verification

The threshold logic is equivalent:
- Python: `start_line <= prev_end_line + 4` means gap can be 0, 1, 2, 3, or 4
- Rust: `gap <= 4` where `gap = start_line - prev_end_line`

**Example**: If `prev.end_line=10` and `cur.start_line=14`:
- Python: `14 <= 10 + 4 = 14` → `14 <= 14` → True (grouped)
- Rust: `gap = 14 - 10 = 4` → `4 <= 4` → True (grouped)

**POTENTIAL ISSUE**: If `start_line < prev_end_line` (overlapping matches):
- Python: `start_line <= prev_end_line + 4` → Always True (groups overlapping)
- Rust: `gap = saturating_sub(start_line, prev_end_line)` → `0` → `0 <= 4` → True (groups overlapping)

Both handle overlapping matches correctly.

---

## 2. Detection Creation

### Python Logic

**File**: `reference/scancode-toolkit/src/licensedcode/detection.py:218-267`

```python
@classmethod
def from_matches(cls, matches, analysis=None, post_scan=False, package_license=False):
    if not matches:
        return
    
    if analysis is None:
        analysis = analyze_detection(license_matches=matches, package_license=package_license)
    
    detection_log, license_expression = get_detected_license_expression(
        analysis=analysis,
        license_matches=matches,
        post_scan=post_scan,
    )
    
    if license_expression == None:
        return cls(matches=matches, detection_log=detection_log)
    
    detection = cls(
        matches=matches,
        license_expression=str(license_expression),
        detection_log=detection_log,
    )
    detection.identifier = detection.identifier_with_expression
    detection.license_expression_spdx = detection.spdx_license_expression()
    return detection
```

**Key behaviors**:
1. Analyze detection to get category
2. Get license expression based on analysis (may filter matches)
3. Generate identifier if expression exists
4. Generate SPDX expression

### Rust Logic

**File**: `src/license_detection/detection/mod.rs:177-236`

```rust
pub fn create_detection_from_group(group: &DetectionGroup) -> LicenseDetection {
    let mut detection = LicenseDetection { /* fields */ };
    
    if group.matches.is_empty() {
        return detection;
    }
    
    let log_category = analyze_detection(&group.matches, false);
    
    let matches_for_expression = if log_category == DETECTION_LOG_UNKNOWN_INTRO_FOLLOWED_BY_MATCH {
        filter_license_intros(&group.matches)
    } else if log_category == DETECTION_LOG_UNKNOWN_REFERENCE_TO_LOCAL_FILE {
        filter_license_intros_and_references(&group.matches)
    } else {
        group.matches.clone()
    };
    
    // Store RAW matches in detection.matches (matching Python behavior)
    detection.matches = group.matches.clone();
    
    // Use FILTERED matches for expression computation
    if let Ok(expr) = determine_license_expression(&matches_for_expression) {
        detection.license_expression = Some(expr.clone());
        
        if let Ok(spdx_expr) = determine_spdx_expression(&matches_for_expression) {
            detection.license_expression_spdx = Some(spdx_expr);
        }
    }
    
    detection.detection_log.push(log_category.to_string());
    
    // Compute identifier
    if let Some(ref expr) = detection.license_expression {
        let id_safe_expression = python_safe_name(expr);
        let content_uuid = compute_content_identifier(&detection.matches);
        detection.identifier = Some(format!("{}-{}", id_safe_expression, content_uuid));
    }
    
    detection
}
```

### Differences

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| Analysis handling | `analyze_detection` returns category string | `analyze_detection` returns `&'static str` | **IDENTICAL** in practice |
| Expression filtering | Handled in `get_detected_license_expression` | Handled explicitly in `create_detection_from_group` | **SAME LOGIC**, different structure |
| Empty detection | Returns `None` if `license_expression == None` | Returns detection with `None` expression | **POTENTIAL DIFFERENCE** |

**POTENTIAL BEHAVIORAL DIFFERENCE**:
- Python: Returns `None` (no detection object) when expression is None
- Rust: Returns `LicenseDetection` with `license_expression: None`

This affects how "license clues" and "false positive" cases are handled - Python doesn't create detection objects for these cases in some scenarios.

---

## 3. License Expression Determination

### Python Logic

**File**: `reference/scancode-toolkit/src/licensedcode/detection.py:1468-1602`

```python
def get_detected_license_expression(analysis, license_matches=None, ...):
    detection_log = []
    matches_for_expression = None
    combined_expression = None
    
    if analysis == DetectionCategory.FALSE_POSITVE.value:
        detection_log.append(DetectionRule.FALSE_POSITIVE.value)
        return detection_log, combined_expression  # Returns None expression
    
    elif analysis == DetectionCategory.LICENSE_CLUES.value:
        detection_log.append(DetectionRule.LICENSE_CLUES.value)
        return detection_log, combined_expression  # Returns None expression
    
    elif analysis == DetectionCategory.LOW_QUALITY_MATCH_FRAGMENTS.value:
        detection_log.append(DetectionRule.LOW_QUALITY_MATCH_FRAGMENTS.value)
        return detection_log, combined_expression  # Returns None expression
    
    elif analysis == DetectionCategory.UNKNOWN_INTRO_BEFORE_DETECTION.value:
        matches_for_expression = filter_license_intros(license_matches)
        detection_log.append(DetectionRule.UNKNOWN_INTRO_FOLLOWED_BY_MATCH.value)
    
    # ... other cases ...
    
    else:
        matches_for_expression = license_matches
    
    combined_expression = combine_expressions(
        expressions=[match.rule.license_expression for match in matches_for_expression],
        licensing=get_licensing(),
    )
    
    return detection_log, str(combined_expression)
```

**Key behaviors**:
1. FALSE_POSITIVE, LICENSE_CLUES, LOW_QUALITY: Return `None` expression
2. UNKNOWN_INTRO: Filter intros, compute expression
3. Default: Compute expression from all matches

### Rust Logic

**File**: `src/license_detection/detection/analysis.rs:308-352`

```rust
pub(super) fn analyze_detection(matches: &[LicenseMatch], package_license: bool) -> &'static str {
    if matches.is_empty() {
        return "";
    }
    
    if is_undetected_license_matches(matches) {
        return "undetected-license-matches";
    }
    
    if matches.iter().all(|m| m.match_coverage >= 99.99) {
        return "";
    }
    
    if !package_license && is_false_positive(matches) {
        return "false-positive";
    }
    
    if has_correct_license_clue_matches(matches) {
        return "license-clues";
    }
    
    if has_unknown_matches(matches) {
        return "unknown-match";
    }
    
    if matches.iter().any(|m| m.match_coverage < IMPERFECT_MATCH_COVERAGE_THR - 0.01) {
        return "imperfect-match-coverage";
    }
    
    if has_extra_words(matches) {
        return "has-extra-words";
    }
    
    ""
}
```

**Expression determination**: `src/license_detection/detection/analysis.rs:383-395`

```rust
pub fn determine_license_expression(matches: &[LicenseMatch]) -> Result<String, String> {
    if matches.is_empty() {
        return Err("No matches to determine expression from".to_string());
    }
    
    let expressions: Vec<&str> = matches
        .iter()
        .map(|m| m.license_expression.as_str())
        .collect();
    
    combine_expressions(&expressions, CombineRelation::And, false)
        .map_err(|e| format!("Failed to combine expressions: {}", e))
}
```

### Differences

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| Analysis return values | Enum string values | Static string constants | **IDENTICAL** semantics |
| Expression filtering | In `get_detected_license_expression` | In `create_detection_from_group` | **SAME LOGIC**, different location |
| Category names | `"perfect-detection"`, `"imperfect-match-coverage"` | `""`, `"imperfect-match-coverage"` | **DIFFERENCE**: Rust uses empty string for perfect |

**BEHAVIORAL DIFFERENCE**:
- Python `analyze_detection()` returns `DetectionCategory.PERFECT_DETECTION.value` = `"perfect-detection"`
- Rust `analyze_detection()` returns `""` for perfect detection

This affects detection_log content:
- Python: `detection_log = ["perfect-detection"]`
- Rust: `detection_log = [""]`

However, Python's actual behavior in `get_detected_license_expression()`:
```python
else:
    if TRACE_ANALYSIS:
        logger_debug(f'analysis not-combined')
    matches_for_expression = license_matches
```

For perfect detection, Python falls through to the `else` clause and doesn't add anything to `detection_log`. So **both are equivalent** - neither adds log entry for perfect detection.

---

## 4. Detection Classification

### Python Logic

**File**: `reference/scancode-toolkit/src/licensedcode/detection.py:1760-1817`

```python
def analyze_detection(license_matches, package_license=False):
    if is_undetected_license_matches(license_matches=license_matches):
        return DetectionCategory.UNDETECTED_LICENSE.value
    
    elif has_unknown_intro_before_detection(license_matches=license_matches):
        return DetectionCategory.UNKNOWN_INTRO_BEFORE_DETECTION.value
    
    elif has_references_to_local_files(license_matches=license_matches):
        return DetectionCategory.UNKNOWN_FILE_REFERENCE_LOCAL.value
    
    elif not package_license and is_false_positive(...):
        return DetectionCategory.FALSE_POSITVE.value
    
    elif not package_license and has_correct_license_clue_matches(...):
        return DetectionCategory.LICENSE_CLUES.value
    
    elif is_correct_detection_non_unknown(license_matches=license_matches):
        return DetectionCategory.PERFECT_DETECTION.value
    
    elif has_unknown_matches(license_matches=license_matches):
        return DetectionCategory.UNKNOWN_MATCH.value
    
    elif not package_license and is_low_quality_matches(license_matches=license_matches):
        return DetectionCategory.LOW_QUALITY_MATCH_FRAGMENTS.value
    
    elif is_match_coverage_less_than_threshold(..., threshold=IMPERFECT_MATCH_COVERAGE_THR):
        return DetectionCategory.IMPERFECT_COVERAGE.value
    
    elif has_extra_words(license_matches=license_matches):
        return DetectionCategory.EXTRA_WORDS.value
    
    else:
        return DetectionCategory.PERFECT_DETECTION.value
```

**Priority order** (most to least important):
1. Undetected license
2. Unknown intro before detection
3. References to local files
4. False positive (if not package_license)
5. License clues (if not package_license)
6. Perfect detection (non-unknown)
7. Unknown match
8. Low quality matches (if not package_license)
9. Imperfect coverage
10. Extra words
11. Default: perfect detection

### Rust Logic

**File**: `src/license_detection/detection/analysis.rs:308-352`

```rust
pub(super) fn analyze_detection(matches: &[LicenseMatch], package_license: bool) -> &'static str {
    if matches.is_empty() {
        return "";
    }
    
    if is_undetected_license_matches(matches) {
        return "undetected-license-matches";
    }
    
    if matches.iter().all(|m| m.match_coverage >= 99.99) {
        return "";
    }
    
    if !package_license && is_false_positive(matches) {
        return "false-positive";
    }
    
    if has_correct_license_clue_matches(matches) {
        return "license-clues";
    }
    
    if has_unknown_matches(matches) {
        return "unknown-match";
    }
    
    if matches.iter().any(|m| m.match_coverage < IMPERFECT_MATCH_COVERAGE_THR - 0.01) {
        return "imperfect-match-coverage";
    }
    
    if has_extra_words(matches) {
        return "has-extra-words";
    }
    
    ""
}
```

### Differences

| Check | Python | Rust | Status |
|-------|--------|------|--------|
| Undetected license | ✅ Line 1769 | ✅ Line 314 | **PARITY** |
| Unknown intro before detection | ✅ Line 1772 | ❌ **MISSING** | **GAP** |
| References to local files | ✅ Line 1775 | ❌ **MISSING** | **GAP** |
| False positive | ✅ Line 1780 | ✅ Line 324 | **PARITY** |
| License clues | ✅ Line 1786 | ✅ Line 329 | **PARITY** |
| Perfect detection (non-unknown) | ✅ Line 1792 | ❌ **NOT EXPLICIT** | **POTENTIAL GAP** |
| Unknown match | ✅ Line 1797 | ✅ Line 334 | **PARITY** |
| Low quality matches | ✅ Line 1800 | ❌ **MISSING** | **GAP** |
| Imperfect coverage | ✅ Line 1805 | ✅ Line 339 | **PARITY** |
| Extra words | ✅ Line 1812 | ✅ Line 347 | **PARITY** |

**MAJOR GAPS IDENTIFIED**:

1. **Unknown intro before detection** (Python: `has_unknown_intro_before_detection`)
   - Rust has `has_unknown_intro_before_detection()` function (analysis.rs:177-221) but **doesn't call it** in `analyze_detection()`
   - This affects expression filtering logic

2. **References to local files** (Python: `has_references_to_local_files`)
   - Rust has `has_references_to_local_files()` (analysis.rs:301-303) but **doesn't call it** in `analyze_detection()`
   - This affects detection category assignment

3. **Low quality matches** (Python: `is_low_quality_matches`)
   - Rust has `is_low_quality_matches()` (analysis.rs:151-157) but **doesn't call it** in `analyze_detection()`
   - This affects detection filtering for low-quality cases

4. **Perfect detection non-unknown** (Python: `is_correct_detection_non_unknown`)
   - Python checks: all matches perfect AND no unknowns AND no extra words
   - Rust only checks: all matches have 100% coverage (line 319)
   - This may cause Rust to classify some detections as perfect when Python wouldn't

---

## 5. Identifier Generation

### Python Logic

**File**: `reference/scancode-toolkit/src/licensedcode/detection.py:305-341`

```python
@property
def _identifier(self):
    """Return an unique identifier based on match contents."""
    data = []
    for match in self.matches:
        matched_text = match.matched_text
        if isinstance(matched_text, typing.Callable):
            matched_text = matched_text()
        if not isinstance(matched_text, str):
            matched_text = repr(matched_text)
        
        tokenized_matched_text = tuple(query_tokenizer(matched_text))
        
        identifier = (
            match.rule.identifier,
            match.score(),
            tokenized_matched_text,
        )
        data.append(identifier)
    
    return get_uuid_on_content(content=data)

@property
def identifier_with_expression(self):
    id_safe_expression = python_safe_name(s=str(self.license_expression))
    return "{}-{}".format(id_safe_expression, self._identifier)
```

**get_uuid_on_content**: Line 513-520
```python
def get_uuid_on_content(content):
    identifier_string = repr(tuple(content))
    md_hash = sha1(identifier_string.encode('utf-8'))
    return str(uuid.UUID(hex=md_hash.hexdigest()[:32]))
```

### Rust Logic

**File**: `src/license_detection/detection/identifier.rs:45-56`

```rust
pub(super) fn compute_content_identifier(matches: &[LicenseMatch]) -> String {
    let content: Vec<(&str, f32, Vec<String>)> = matches
        .iter()
        .map(|m| {
            let matched_text = m.matched_text.as_deref().unwrap_or("");
            let tokens = tokenize_without_stopwords(matched_text);
            (m.rule_identifier.as_str(), m.score, tokens)
        })
        .collect();
    
    get_uuid_on_content(&content)
}
```

**UUID generation**: Line 29-43
```rust
pub(super) fn get_uuid_on_content(content: &[(&str, f32, Vec<String>)]) -> String {
    let repr_str = format_python_tuple_repr(content);
    
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(repr_str.as_bytes());
    let hash = hasher.finalize();
    let hex_str = hex::encode(hash);
    
    let uuid_hex = &hex_str[..32];
    
    uuid::Uuid::parse_str(uuid_hex)
        .map(|u| u.to_string())
        .unwrap_or_else(|_| uuid_hex.to_string())
}
```

**Python tuple repr**: Line 58-79
```rust
pub(super) fn format_python_tuple_repr(content: &[(&str, f32, Vec<String>)]) -> String {
    let mut result = String::from("(");
    
    for (i, (rule_id, score, tokens)) in content.iter().enumerate() {
        if i > 0 {
            result.push_str(", ");
        }
        result.push_str(&format!(
            "({}, {}, {})",
            python_str_repr(rule_id),
            format_score_for_repr(*score),
            python_token_tuple_repr(tokens)
        ));
    }
    
    if content.len() == 1 {
        result.push(',');
    }
    result.push(')');
    
    result
}
```

### Differences

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| Tokenizer | `query_tokenizer()` | `tokenize_without_stopwords()` | **NEEDS VERIFICATION** - must produce identical output |
| Score type | `match.score()` (method call) | `m.score` (field access) | **IDENTICAL** if score is pre-computed |
| Matched text handling | Callable support, repr fallback | `Option<String>` field | **POTENTIAL DIFFERENCE** |
| Python repr format | Python `repr(tuple)` | Rust `format_python_tuple_repr()` | **NEEDS VERIFICATION** - must match exactly |
| Single-element tuple | `repr((item,))` has trailing comma | Rust explicitly adds `,` for single element | **MATCHES** |

**CRITICAL VERIFICATION NEEDED**:

The UUID generation relies on exact string representation matching Python's `repr()`. The Rust implementation attempts to replicate this, but subtle differences could cause different identifiers:

1. String escaping (quotes, backslashes)
2. Float formatting (Python uses `repr(float)`, Rust uses `{:?}`)
3. Token tuple representation

**Example potential difference**:
```python
# Python
repr(95.0)  # '95.0'
repr(100.0) # '100.0'
```

```rust
// Rust
format!("{:?}", 95.0)  // "95.0"
format!("{:?}", 100.0) // "100.0"
```

These appear to match, but edge cases (infinity, NaN, very large numbers) may differ.

---

## 6. File Region Tracking

### Python Logic

**File**: `reference/scancode-toolkit/src/licensedcode/detection.py:149-162`

```python
@attr.s
class FileRegion:
    path = attr.ib(type=str)
    start_line = attr.ib(type=int)
    end_line = attr.ib(type=int)
```

**Usage**: Line 293-303
```python
def get_file_region(self, path):
    start_line, end_line = self.get_start_end_line()
    return FileRegion(
        path=path,
        start_line=start_line,
        end_line=end_line,
    )

def get_start_end_line(self):
    if isinstance(self.matches[0], dict):
        start_line = min([match['start_line'] for match in self.matches])
        end_line = max([match['end_line'] for match in self.matches])
    else:
        start_line = min([match.start_line for match in self.matches])
        end_line = max([match.end_line for match in self.matches])
    return start_line, end_line
```

### Rust Logic

**File**: `src/license_detection/detection/types.rs:63-71`

```rust
#[derive(Debug, Clone)]
pub struct FileRegion {
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
}
```

**Usage**: `src/license_detection/detection/mod.rs:128-134`
```rust
if group.start_line > 0 {
    detection.file_region = Some(FileRegion {
        path: String::new(),  // Path set later
        start_line: group.start_line,
        end_line: group.end_line,
    });
}
```

**DetectionGroup::new**: `types.rs:14-32`
```rust
impl DetectionGroup {
    pub fn new(matches: Vec<LicenseMatch>) -> Self {
        if matches.is_empty() {
            return Self {
                matches,
                start_line: 0,
                end_line: 0,
            };
        }
        
        let start_line = matches.iter().map(|m| m.start_line).min().unwrap_or(0);
        let end_line = matches.iter().map(|m| m.end_line).max().unwrap_or(0);
        
        Self {
            matches,
            start_line,
            end_line,
        }
    }
}
```

### Differences

| Aspect | Python | Rust | Impact |
|--------|--------|------|--------|
| Path population | Set at `get_file_region(path)` call | Set to `String::new()` initially | **DIFFERENCE** - Rust defers path setting |
| Empty match handling | Not explicit (assumes non-empty) | Returns `start_line=0, end_line=0` | **DIFFERENCE** - Rust has explicit handling |
| Type for matches | `dict` or `LicenseMatch` | Only `LicenseMatch` | Rust is cleaner (no dual type support) |

**POTENTIAL ISSUE**:
- Rust sets `path: String::new()` in `populate_detection_from_group`
- Python requires explicit `path` parameter to `get_file_region()`
- This means Rust's `file_region.path` may be empty unless explicitly populated elsewhere

---

## Summary of Findings

### Critical Issues (Must Fix)

1. **Missing classification checks in `analyze_detection()`**:
   - `has_unknown_intro_before_detection()` - function exists but not called
   - `has_references_to_local_files()` - function exists but not called  
   - `is_low_quality_matches()` - function exists but not called

2. **Identifier generation verification needed**:
   - Ensure `format_python_tuple_repr()` produces exact Python repr output
   - Verify tokenizer equivalence: `query_tokenizer()` vs `tokenize_without_stopwords()`
   - Test edge cases: empty tokens, special characters, very long strings

### Behavioral Differences (May Need Attention)

3. **Detection creation for no-expression cases**:
   - Python: Returns `None` (no detection object)
   - Rust: Returns `LicenseDetection` with `license_expression: None`
   - Affects: FALSE_POSITIVE, LICENSE_CLUES, LOW_QUALITY_MATCH_FRAGMENTS

4. **Perfect detection classification**:
   - Python: Explicit check for non-unknown, no extra words
   - Rust: Only checks coverage >= 100%
   - May classify differently for edge cases

5. **File region path**:
   - Python: Explicit path parameter
   - Rust: Empty string placeholder
   - Requires verification that path is set correctly in output pipeline

### Minor Issues (Low Priority)

6. **Detection log for perfect matches**:
   - Both use empty/implicit empty, but naming conventions differ slightly

7. **Generator vs Vec return**:
   - Python: Generator (lazy evaluation)
   - Rust: Vec (eager evaluation)
   - Performance tradeoff, not behavioral difference

---

## Recommendations

1. **Add missing classification checks** to `analyze_detection()` in order:
   ```rust
   // After undetected check:
   if has_unknown_intro_before_detection(matches) {
       return DETECTION_LOG_UNKNOWN_INTRO_FOLLOWED_BY_MATCH;
   }
   
   if has_references_to_local_files(matches) {
       return DETECTION_LOG_UNKNOWN_REFERENCE_TO_LOCAL_FILE;
   }
   
   // After unknown match check:
   if !package_license && is_low_quality_matches(matches) {
       return DETECTION_LOG_LOW_QUALITY_MATCHES;
   }
   ```

2. **Add tests for identifier generation** comparing Python/Rust output for:
   - Single match with simple text
   - Multiple matches with complex expressions
   - Edge cases: empty tokens, special characters, unicode

3. **Add integration test** for full detection pipeline comparing Python/Rust output on real files

4. **Document behavior difference** for detection creation when expression is None (may need alignment)

5. **Verify file_region.path** is populated correctly in the output pipeline
