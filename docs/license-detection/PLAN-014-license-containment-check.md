# PLAN-014: License Expression Containment Checking

> **Status**: Not Started
> **Priority**: P1 — Critical Correctness Issue
> **Estimated Effort**: 2-3 days
> **Created**: 2026-02-17

## Problem Statement

The Rust license detection implementation uses **line-based containment checks** instead of **token-position-based qspan containment**, causing approximately 5 golden tests to fail. This is a fundamental architectural difference in how Python and Rust determine whether one license match "contains" another.

### Current Behavior (Incorrect)

In `src/license_detection/match_refine.rs:152-179`:

```rust
fn filter_contained_matches(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    // ...
    let is_contained = kept.iter().any(|kept_match: &&LicenseMatch| {
        current.start_line >= kept_match.start_line
            && current.end_line <= kept_match.end_line
            && current.matched_length <= kept_match.matched_length
    });
    // ...
}
```

This compares **line numbers** instead of **token positions**.

### Python Reference Behavior (Correct)

In `reference/scancode-toolkit/src/licensedcode/match.py:438-448`:

```python
def __contains__(self, other):
    """
    Return True if qspan contains other.qspan and ispan contains other.ispan.
    """
    return other.qspan in self.qspan and other.ispan in self.ispan

def qcontains(self, other):
    """
    Return True if qspan contains other.qspan.
    """
    return other.qspan in self.qspan
```

The key difference:
- **qspan**: Query span - positions in the input text's token stream
- **ispan**: Index span - positions in the rule's token stream
- Both use **token positions**, not line numbers

---

## Python Reference Analysis

### 1. Span Class (Token Position Tracking)

**File**: `reference/scancode-toolkit/src/licensedcode/spans.py`

```python
class Span(Set):
    """
    Represent ranges of integers (such as tokens positions) as a set of integers.
    A Span is hashable and not meant to be modified once created, like a frozenset.
    It is equivalent to a sparse closed interval.
    """

    @property
    def start(self):
        if not self._set:
            raise TypeError('Empty Span has no start.')
        return self._set[0]

    @property
    def end(self):
        if not self._set:
            raise TypeError('Empty Span has no end.')
        return self._set[-1]

    def __contains__(self, other):
        """
        Return True if this span contains other span (where other is a Span, an
        int or an ints set).
        """
        if isinstance(other, Span):
            return self._set.issuperset(other._set)
        # ...
```

Key insight: `Span` is a **set of token positions**, not a line range.

### 2. Match.qspan and Match.ispan

**File**: `reference/scancode-toolkit/src/licensedcode/match.py:410-430`:

```python
@property
def qstart(self):
    return self.qspan.start

@property
def qend(self):
    return self.qspan.end

@property
def istart(self):
    return self.ispan.start

@property
def iend(self):
    return self.ispan.end
```

- **qspan**: Token positions in the query (input text)
- **ispan**: Token positions in the rule (license template)
- **hispan**: High-value token positions in the rule

### 3. licensing_contains Method

**File**: `reference/scancode-toolkit/src/licensedcode/match.py:388-392`:

```python
def licensing_contains(self, other):
    """
    Return True if this match licensing contains the other match licensing.
    """
    return self.rule.licensing_contains(other.rule)
```

**File**: `reference/scancode-toolkit/src/licensedcode/models.py:2065-2073`:

```python
def licensing_contains(self, other):
    """
    Return True if this rule licensing contains the other rule licensing.
    """
    if self.license_expression and other.license_expression:
        return self.licensing.contains(
            expression1=self.license_expression_object,
            expression2=other.license_expression_object,
        )
```

This uses the `license_expression` Python package to check if one license expression semantically contains another (e.g., "MIT OR Apache-2.0" contains "MIT").

### 4. Usage in filter_overlapping_matches

**File**: `reference/scancode-toolkit/src/licensedcode/match.py:1374-1468`:

```python
# MEDIUM next case
if (current_match.licensing_contains(next_match)
    and current_match.len() >= next_match.len()
    and current_match.hilen() >= next_match.hilen()
):
    # Remove next_match - current contains it
    discarded_append(matches_pop(j))
    continue

if (next_match.licensing_contains(current_match)
    and current_match.len() <= next_match.len()
    and current_match.hilen() <= next_match.hilen()
):
    # Remove current_match - next contains it
    discarded_append(matches_pop(i))
    i -= 1
    break

# SMALL next case with surround check
if (small_next
    and current_match.surround(next_match)
    and current_match.licensing_contains(next_match)
    and current_match.len() >= next_match.len()
    and current_match.hilen() >= next_match.hilen()
):
    discarded_append(matches_pop(j))
    continue
```

The `surround()` method checks line boundaries:

```python
def surround(self, other):
    return self.start <= other.start and self.end >= other.end
```

But `licensing_contains()` checks **license expression semantics**.

---

## Rust Code Analysis

### Current Implementation

**File**: `src/license_detection/match_refine.rs`

#### 1. filter_contained_matches (Line 152-179)

```rust
fn filter_contained_matches(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    if matches.len() < 2 {
        return matches.to_vec();
    }

    let mut sorted: Vec<&LicenseMatch> = matches.iter().collect();
    sorted.sort_by(|a, b| {
        a.start_line
            .cmp(&b.start_line)
            .then_with(|| b.matched_length.cmp(&a.matched_length))
    });

    let mut kept = Vec::new();

    for current in sorted {
        let is_contained = kept.iter().any(|kept_match: &&LicenseMatch| {
            current.start_line >= kept_match.start_line
                && current.end_line <= kept_match.end_line
                && current.matched_length <= kept_match.matched_length
        });

        if !is_contained {
            kept.push(current);
        }
    }

    kept.into_iter().cloned().collect()
}
```

**Problem**: Uses `start_line`/`end_line` instead of `qstart`/`qend`.

#### 2. licensing_contains_approx (Line 241-243)

```rust
fn licensing_contains_approx(current: &LicenseMatch, other: &LicenseMatch) -> bool {
    current.matched_length >= other.matched_length * 2
}
```

**Problem**: This is a heuristic approximation, not the actual license expression containment check.

#### 3. filter_overlapping_matches (Line 245-420)

Uses `licensing_contains_approx` in several places (lines 337, 345, 356, 364, 376, 386):

```rust
if medium_next {
    if licensing_contains_approx(&matches[i], &matches[j])
        && current_len_val >= next_len_val
        && current_hilen >= next_hilen
    {
        discarded.push(matches.remove(j));
        continue;
    }
    // ...
}
```

### LicenseMatch Struct

**File**: `src/license_detection/models.rs:175-224`:

```rust
pub struct LicenseMatch {
    pub license_expression: String,
    pub license_expression_spdx: String,
    pub from_file: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub matcher: String,
    pub score: f32,
    pub matched_length: usize,
    pub match_coverage: f32,
    pub rule_relevance: u8,
    pub rule_identifier: String,
    pub rule_url: String,
    pub matched_text: Option<String>,
    pub referenced_filenames: Option<Vec<String>>,
    pub is_license_intro: bool,
    pub is_license_clue: bool,
}
```

**Missing Fields**:
- `qstart` - Start token position in query
- `qend` - End token position in query
- `ispan_start` - Start position in rule
- `ispan_end` - End position in rule
- `hispan` - High-value token positions

---

## Proposed Changes

### Phase 1: Add Token Position Tracking to LicenseMatch

#### 1.1 Add Fields to LicenseMatch

**File**: `src/license_detection/models.rs`

```rust
pub struct LicenseMatch {
    // ... existing fields ...
    
    /// Start token position in the query (input text)
    pub qstart: usize,
    
    /// End token position in the query (inclusive)
    pub qend: usize,
    
    /// Start position in the rule (license template)
    pub ispan_start: usize,
    
    /// End position in the rule (inclusive)
    pub ispan_end: usize,
    
    /// Number of high-value tokens matched
    pub hilen: usize,
}
```

#### 1.2 Update Default Implementation

```rust
impl Default for LicenseMatch {
    fn default() -> Self {
        Self {
            // ... existing defaults ...
            qstart: 0,
            qend: 0,
            ispan_start: 0,
            ispan_end: 0,
            hilen: 0,
        }
    }
}
```

### Phase 2: Update Matchers to Populate Token Positions

Each matcher needs to track and populate token positions:

#### 2.1 hash_match.rs

The hash matcher already has access to query token positions. Update `hash_match()` to populate `qstart`/`qend`:

```rust
// In hash_match function, when creating LicenseMatch:
LicenseMatch {
    // ... existing fields ...
    qstart: query_run.start + match_start_token,
    qend: query_run.start + match_end_token,
    ispan_start: 0,  // Hash matches typically match from start
    ispan_end: rule_token_count - 1,
    hilen: high_token_count,
}
```

#### 2.2 aho_match.rs

**File**: `src/license_detection/aho_match.rs`

The Aho-Corasick matcher already has `qstart` and `qend` calculated (lines 96-97):

```rust
let qstart = qbegin + byte_pos_to_token_pos(byte_start);
let qend = qbegin + byte_pos_to_token_pos(byte_end);
```

Pass these to the LicenseMatch construction.

#### 2.3 seq_match.rs

**File**: `src/license_detection/seq_match.rs`

The sequence matcher has `qbegin` and `qpos` tracking. Update to populate token positions in the result.

#### 2.4 spdx_lid.rs

**File**: `src/license_detection/spdx_lid.rs`

SPDX-LID matches need similar updates.

### Phase 3: Implement Proper Containment Checks

#### 3.1 Implement qcontains Method on LicenseMatch

**File**: `src/license_detection/models.rs`

```rust
impl LicenseMatch {
    /// Check if this match's qspan contains the other match's qspan.
    /// Based on Python: `qcontains()` in match.py:444-448
    pub fn qcontains(&self, other: &LicenseMatch) -> bool {
        self.qstart <= other.qstart && self.qend >= other.qend
    }

    /// Check if this match's ispan contains the other match's ispan.
    pub fn icontains(&self, other: &LicenseMatch) -> bool {
        self.ispan_start <= other.ispan_start && self.ispan_end >= other.ispan_end
    }

    /// Check if this match contains the other match (both qspan and ispan).
    /// Based on Python: `__contains__()` in match.py:438-442
    pub fn contains_match(&self, other: &LicenseMatch) -> bool {
        self.qcontains(other) && self.icontains(other)
    }
}
```

#### 3.2 Implement licensing_contains for Rules

This requires implementing license expression containment logic.

**Option A**: Use the `spdx-expression` crate (if available) or implement custom logic.

**Option B**: Implement a simplified version for common cases:

**File**: `src/license_detection/models.rs` or new file `src/license_detection/expression_contains.rs`

```rust
/// Check if one license expression contains another.
/// 
/// This is a simplified implementation that handles common cases:
/// - "MIT OR Apache-2.0" contains "MIT"
/// - "MIT AND Apache-2.0" contains both "MIT" and "Apache-2.0"
/// - "MIT" contains "MIT"
/// - "MIT" does NOT contain "Apache-2.0"
/// 
/// For full correctness, this should use a proper license expression parser.
pub fn license_expression_contains(container: &str, contained: &str) -> bool {
    // Normalize expressions
    let container_lower = container.to_lowercase();
    let contained_lower = contained.to_lowercase();
    
    // Exact match
    if container_lower == contained_lower {
        return true;
    }
    
    // Simple case: container is "X OR Y" and contained is X or Y
    if container_lower.contains(" or ") {
        let parts: Vec<&str> = container_lower.split(" or ").collect();
        for part in parts {
            let trimmed = part.trim();
            if trimmed == contained_lower {
                return true;
            }
        }
    }
    
    // Simple case: container is "X AND Y" and contained is X or Y
    if container_lower.contains(" and ") {
        let parts: Vec<&str> = container_lower.split(" and ").collect();
        for part in parts {
            let trimmed = part.trim();
            if trimmed == contained_lower {
                return true;
            }
        }
    }
    
    // TODO: Implement proper license expression containment using
    // the license_expression library semantics
    false
}
```

**Better approach**: Port the `license-expression` Python library logic or find a Rust equivalent.

#### 3.3 Update filter_contained_matches

**File**: `src/license_detection/match_refine.rs`

```rust
fn filter_contained_matches(matches: &[LicenseMatch]) -> Vec<LicenseMatch> {
    if matches.len() < 2 {
        return matches.to_vec();
    }

    // Sort by qstart, then by matched_length (longer first)
    let mut sorted: Vec<&LicenseMatch> = matches.iter().collect();
    sorted.sort_by(|a, b| {
        a.qstart
            .cmp(&b.qstart)
            .then_with(|| b.qend.cmp(&a.qend))  // Longer spans first
    });

    let mut kept = Vec::new();

    for current in sorted {
        let is_contained = kept.iter().any(|kept_match: &&LicenseMatch| {
            // Check qspan containment (token positions)
            kept_match.qcontains(current)
        });

        if !is_contained {
            kept.push(current);
        }
    }

    kept.into_iter().cloned().collect()
}
```

#### 3.4 Update filter_overlapping_matches

Replace `licensing_contains_approx` with proper `licensing_contains`:

```rust
/// Check if the license expression of `current` contains the license expression of `other`.
/// Based on Python: `licensing_contains()` in match.py:388-392
fn licensing_contains(current: &LicenseMatch, other: &LicenseMatch, index: &LicenseIndex) -> bool {
    // Get the license expression objects for both matches
    let current_expr = &current.license_expression;
    let other_expr = &other.license_expression;
    
    // Use proper license expression containment
    license_expression_contains(current_expr, other_expr)
}
```

Then update all usages in `filter_overlapping_matches`:

```rust
// Before:
if licensing_contains_approx(&matches[i], &matches[j]) ...

// After:
if licensing_contains(&matches[i], &matches[j], index) ...
```

### Phase 4: Update Query and QueryRun to Track Positions

Ensure the `Query` and `QueryRun` structs properly track token positions:

**File**: `src/license_detection/query.rs`

The `QueryRun` already has position tracking. Verify that:
- `start` is the absolute token position in the query
- `end` is the absolute token position in the query
- `line_for_pos()` correctly maps token positions to line numbers

---

## Testing Strategy

### 1. Unit Tests for Containment

Add unit tests to `src/license_detection/match_refine_test.rs`:

```rust
#[test]
fn test_qcontains_simple() {
    let outer = LicenseMatch {
        qstart: 0,
        qend: 10,
        ..Default::default()
    };
    let inner = LicenseMatch {
        qstart: 2,
        qend: 8,
        ..Default::default()
    };
    assert!(outer.qcontains(&inner));
    assert!(!inner.qcontains(&outer));
}

#[test]
fn test_qcontains_overlapping_but_not_contained() {
    let a = LicenseMatch {
        qstart: 0,
        qend: 5,
        ..Default::default()
    };
    let b = LicenseMatch {
        qstart: 3,
        qend: 10,
        ..Default::default()
    };
    assert!(!a.qcontains(&b));
    assert!(!b.qcontains(&a));
}

#[test]
fn test_filter_contained_with_token_positions() {
    let matches = vec![
        LicenseMatch {
            qstart: 0,
            qend: 20,
            start_line: 1,
            end_line: 5,
            matched_length: 20,
            ..Default::default()
        },
        LicenseMatch {
            qstart: 5,
            qend: 15,
            start_line: 2,
            end_line: 4,
            matched_length: 10,
            ..Default::default()
        },
    ];
    let filtered = filter_contained_matches(&matches);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].qstart, 0);
    assert_eq!(filtered[0].qend, 20);
}
```

### 2. License Expression Containment Tests

```rust
#[test]
fn test_license_expression_contains_or() {
    assert!(license_expression_contains("mit OR apache-2.0", "mit"));
    assert!(license_expression_contains("mit OR apache-2.0", "apache-2.0"));
    assert!(!license_expression_contains("mit OR apache-2.0", "gpl-3.0"));
}

#[test]
fn test_license_expression_contains_and() {
    assert!(license_expression_contains("mit AND apache-2.0", "mit"));
    assert!(license_expression_contains("mit AND apache-2.0", "apache-2.0"));
}

#[test]
fn test_license_expression_contains_exact() {
    assert!(license_expression_contains("mit", "mit"));
    assert!(!license_expression_contains("mit", "apache-2.0"));
}
```

### 3. Golden Test Verification

Run the golden test suite to verify the fix:

```bash
cargo test license_detection_golden
```

The specific failing tests should now pass. Look for tests involving:
- Overlapping license matches
- License expression containment (e.g., "MIT OR Apache" vs "MIT")
- Multi-license files

### 4. Python Reference Comparison

For cases where the logic is unclear, compare against Python:

```bash
cd reference/scancode-toolkit
./scancode --license <test-file> --json-pp -
```

---

## Implementation Order

1. **Add token position fields** to `LicenseMatch` (models.rs)
2. **Update matchers** to populate token positions:
   - hash_match.rs
   - aho_match.rs  
   - seq_match.rs
   - spdx_lid.rs
   - unknown_match.rs
3. **Implement qcontains method** on LicenseMatch
4. **Implement licensing_contains** for license expressions
5. **Update filter_contained_matches** to use token positions
6. **Update filter_overlapping_matches** to use proper containment
7. **Add unit tests** for containment logic
8. **Run golden tests** to verify fix

---

## Risks and Considerations

### 1. License Expression Parsing Complexity

The `license_expression` Python library has sophisticated logic for:
- Parsing SPDX expressions
- Handling operators (AND, OR, WITH)
- Exception handling (e.g., "GPL-2.0 WITH Classpath-exception-2.0")
- License equivalence

**Mitigation**: Start with a simplified implementation for common cases, then expand.

### 2. Backward Compatibility

Adding new fields to `LicenseMatch` is a breaking change for the public API.

**Mitigation**: The fields can be added with default values; existing code continues to work.

### 3. Performance Impact

Token position tracking adds minimal overhead (just storing a few usize values per match).

### 4. Test Coverage

Some edge cases may not be covered by existing tests.

**Mitigation**: Add specific unit tests for containment logic before changing the implementation.

---

## Success Criteria

1. All golden tests pass
2. `filter_contained_matches` uses token positions (qspan) instead of line numbers
3. `licensing_contains` properly checks license expression containment
4. Unit tests cover containment logic with >90% coverage
5. No performance regression (verify with benchmarks)

---

## References

- Python Span class: `reference/scancode-toolkit/src/licensedcode/spans.py`
- Python Match class: `reference/scancode-toolkit/src/licensedcode/match.py`
- Python Rule.licensing_contains: `reference/scancode-toolkit/src/licensedcode/models.py:2065-2073`
- Rust match_refine.rs: `src/license_detection/match_refine.rs`
- Rust models.rs: `src/license_detection/models.rs`
