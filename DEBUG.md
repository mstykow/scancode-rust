# License Detection Debug Notes

## Current Status

**Golden Tests:** 156 passed, 135 failed (as of commit `b714310`)

---

## Open Issues

### Issue 1: YAML Frontmatter Parsing Bug (High Priority)

**Problem:** 25 license files fail to load with "empty text content" warnings. Files with PGP signatures or dashes in content are incorrectly parsed.

**Example warnings:**

```text
Warning: Failed to parse license file reference/scancode-toolkit/src/licensedcode/data/licenses/tcp-wrappers.LICENSE: License file has empty text content and is not deprecated/unknown/generic
```

**Root Cause:** Rust uses naive string split which matches `---` anywhere in the file:

```rust
// src/license_detection/rules/loader.rs:232, 347
let parts: Vec<&str> = content.split("---").collect();
```

This incorrectly splits on:

- `-----BEGIN PGP SIGNED MESSAGE-----` (PGP signatures)
- Any `---` appearing in license text

**Example:** `tcp-wrappers.LICENSE` contains PGP-signed text. The naive split produces 9 parts, with `parts[2]` being empty.

**Python Behavior:** Python uses regex with MULTILINE flag (`frontmatter.py`):

```python
FM_BOUNDARY = re.compile(r"^-{3,}\s*$", re.MULTILINE)
_, fm, content = self.FM_BOUNDARY.split(text, 2)
```

This matches `---` only at line boundaries.

**Fix Attempt:** Replaced with regex:

```rust
static FM_BOUNDARY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^---+\s*$").expect("Invalid frontmatter boundary regex")
});

fn split_frontmatter(content: &str) -> Vec<&str> {
    FM_BOUNDARY.splitn(content, 3).collect()
}
```

**Result:** 156 → 142 passed (14-test regression)

**Why it failed:** The regex split produces different whitespace handling. The YAML content has leading `\n` which affects parsing. Needs more investigation to match Python's exact behavior.

**Recommended Fix:**

1. Study Python's exact whitespace handling in `frontmatter.py`
2. Ensure regex split produces identical parts to Python's split
3. Add unit tests comparing Python vs Rust split results for edge cases:
   - Files with PGP signatures
   - Files with dashes in content
   - Files with varying numbers of dashes (--- vs ----)

**Files Affected:**

| File | Location |
|------|----------|
| `src/license_detection/rules/loader.rs` | Lines 232, 347 - YAML split logic |
| `reference/scancode-toolkit/src/licensedcode/frontmatter.py` | Lines 49-54 (Python reference) |

**Impact:**

- 25 license files fail to load (including `tcp-wrappers`, `ofl-1.1`, etc.)
- These licenses cannot be detected until fixed
- Likely affects detection accuracy for many test cases

---

### Issue 2: Unknown License Intros Appear in Expressions (Medium Priority)

**Problem:** Files produce detections with "unknown" in license expressions when they shouldn't.

**Example:** `COPYING.gplv3`

- **Expected:** `["gpl-3.0"]`
- **Actual:** 8 detections including `"gpl-3.0 AND unknown"`

**Root Cause:** Rust builds license expressions from ALL matches, including license intro matches that should be filtered.

**Python Behavior:** Python filters intros through two-step process (`detection.py`):

1. **`analyze_detection()`** (line 1760): Returns category `UNKNOWN_INTRO_BEFORE_DETECTION` when an unknown intro is followed by a proper license match.

2. **`get_detected_license_expression()`** (lines 1510-1514): Filters intros before building expression:

   ```python
   elif analysis == DetectionCategory.UNKNOWN_INTRO_BEFORE_DETECTION.value:
       matches_for_expression = filter_license_intros(license_matches)
   ```

3. **`is_license_intro()`** (lines 1349-1365): A match is an intro if:
   - Rule has `is_license_intro` OR `is_license_clue` OR `license_expression == 'free-unknown'`
   - AND matcher is exact (`MATCH_AHO_EXACT`) OR coverage is 100%

**Previous Fix Attempt:** 5-test regression, reverted.

**Why it failed:**

1. Expression was built BEFORE category analysis (wrong order)
2. `is_unknown_intro()` logic was incomplete - didn't check rule fields
3. Missing the "exact matcher OR 100% coverage" condition

**Recommended Fix:**

1. **Reorder `create_detection_from_group()`** in `src/license_detection/detection.rs`:
   - Analyze category FIRST
   - Filter matches based on category
   - THEN build expression

2. **Fix `is_unknown_intro()` to match Python**:

   ```rust
   fn is_unknown_intro(m: &LicenseMatch) -> bool {
       let is_intro = m.is_license_intro || m.is_license_clue || 
                      m.license_expression == "free-unknown";
       let is_exact = m.matcher == "2-aho" || m.match_coverage >= 99.99;
       is_intro && is_exact
   }
   ```

3. **Ensure `is_license_intro` is populated** in `LicenseMatch` from rule data.

**Files Affected:**

| File | Location |
|------|----------|
| `src/license_detection/detection.rs` | `create_detection_from_group()` - reorder logic |
| `src/license_detection/models.rs` | `LicenseMatch` - needs `is_license_intro`, `is_license_clue` fields |
| `reference/scancode-toolkit/src/licensedcode/detection.py` | Lines 1349-1365, 1510-1514 |

---

### Issue 3: Deprecated Rules Handling (Low Priority)

**Problem:** Test `camellia_bsd.c` expected `bsd-2-clause-first-lines`, got `freebsd-doc` (a deprecated rule).

**Python Behavior:** Python **skips deprecated rules by default** (`models.py:1103-1104`):

```python
# always skip deprecated rules
rules = [r for r in rules if not r.is_deprecated]
```

Deprecated rules have `replaced_by` pointing to the new license key.

**Previous Fix Attempt:** Skip deprecated rules during index building.

**Result:** 19-test regression (160 → 141 passed).

**Why it failed:**

- Some tests explicitly expect deprecated license expressions (e.g., `freebsd-doc_*.txt` tests)
- The `freebsd-doc.LICENSE` file is NOT deprecated - only `freebsd-doc_5.RULE` is deprecated
- Tests should match against the non-deprecated LICENSE file

**Recommended Fix:**

1. **Keep skipping deprecated rules** (matches Python's default behavior)
2. **Update golden tests** that expect deprecated expressions to expect the replacement expressions
3. **Alternative:** Add `--with-deprecated` flag for backwards compatibility

**Files Affected:**

| File | Location |
|------|----------|
| `src/license_detection/index/builder.rs` | Rule loading - filter deprecated |
| `src/license_detection/rules/loader.rs` | Load `is_deprecated` from rule files |
| `testdata/license-golden/datadriven/lic1/freebsd-doc_*.txt.EXPECTED` | May need updates |

---

## Fixed Issues

### Pipeline Short-Circuit (Fixed in `b714310`)

**Problem:** Matchers were skipped when high-coverage matches found, missing partial licenses.

**Fix:** Removed `has_perfect_match`/`has_high_coverage` short-circuit logic. All matchers now always run.

**File:** `src/license_detection/mod.rs`

### Line Number Calculation (Fixed in `0d72a6e`)

**Problem:** All matches appeared to span the entire document.

**Fix:** Added `QueryRun::line_for_pos()` and used match positions instead of query run boundaries.

**Files:** `src/license_detection/query.rs`, `src/license_detection/aho_match.rs`, `src/license_detection/hash_match.rs`

### GPL False Positive Filter (Fixed in `0d72a6e`)

**Problem:** GPL matches with `matched_length <= 3` caused false positives.

**Fix:** Added `filter_short_gpl_matches()` to remove short GPL matches.

**File:** `src/license_detection/match_refine.rs`

### Rule Sorting (Fixed in `a0952ea`)

**Problem:** Rule IDs differed between Python and Rust due to missing sort.

**Fix:** Added `all_rules.sort()` before assigning rule IDs in `builder.rs`.

**Files:** `src/license_detection/index/builder.rs`, `src/license_detection/models.rs`
