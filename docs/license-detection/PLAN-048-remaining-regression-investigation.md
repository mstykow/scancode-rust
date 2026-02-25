# PLAN-048: Remaining -3 Regression Investigation

## Status: SUPERSEDED - Issues extracted to dedicated plans

## Summary

This investigation identified several issues. All actionable items have been extracted to dedicated plans:

| Issue | New Plan | Priority |
|-------|----------|----------|
| `combine_matches` missing validation | PLAN-049 | HIGH |
| NuGet SPDX pattern | PLAN-050 | MEDIUM |
| hispan reconstruction | PLAN-051 | MEDIUM |
| `restore_non_overlapping` lines vs tokens | PLAN-030 (existing) | LOW |
| Extra licensing_contains check | Documented as intentional extension | NO ACTION |

**Current State:**

- Baseline: 3580 passed, 583 failed
- Current: 3576 passed, 587 failed (-4 total)

---

## Hypotheses Investigated and Dismissed

All 6 original hypotheses were investigated and found **NOT to be the cause**:

| Hypothesis | Result | Evidence |
|------------|--------|----------|
| String sorting difference | ❌ | Both use identical sorting keys; `#<rid>` only in test code |
| Grouping key difference | ❌ | Both group by `rule_identifier` identically |
| Parsing `#<number>` format | ❌ | `parse_rule_id()` is `#[cfg(test)]` only |
| Hash lookups wrong format | ❌ | All production code uses `m.rid` directly |
| Golden test comparison | ❌ | Only compares `license_expression`, not `rule_identifier` |
| Length/format assumptions | ❌ | No length checks or format assumptions found |

---

## Issues Found and Existing Documentation

### P1: `restore_non_overlapping` Uses Lines Instead of Tokens

**Status**: ✅ Already documented in PLAN-030

**Location**: `match_refine.rs:702-728`

**Finding**: Python uses token positions (`qspan`) for intersection checks, Rust uses line numbers.

**Reference**: PLAN-030 documents this thoroughly but marks it "deferred" because it doesn't address root cause of golden test failures. However, it may still contribute to remaining regressions.

**Action**: See PLAN-030 for implementation details.

---

### P2: `combine_matches()` Missing Rule Validation (NEW FINDING)

**Status**: ❌ **NEW - Not documented in any existing plan**

**Location**: `match_refine.rs:106-146`

**Finding**: Rust's `combine_matches()` does NOT validate that matches have the same `rule_identifier` before combining. Python throws `TypeError` if rules differ.

**Python** (match.py:642-646):

```python
def combine(self, other):
    if self.rule != other.rule:
        raise TypeError('Cannot combine matches with different rules')
```

**Rust**:

```rust
fn combine_matches(a: &LicenseMatch, b: &LicenseMatch) -> LicenseMatch {
    let mut merged = a.clone();
    // No validation that a and b have the same rule_identifier!
```

**Impact**: If matches from different rules ever end up in the same group, Rust silently merges them with undefined behavior.

**Priority**: HIGH

**Action**: Add validation: `assert_eq!(a.rule_identifier, b.rule_identifier)` or return error.

---

### P3: Missing NuGet SPDX Pattern Detection

**Status**: ⚠️ Partially documented (symptom in PLAN-023, not root cause)

**Location**: `query.rs:371-374`

**Finding**: Rust only checks for `["spdx", "license", "identifier"]` pattern, missing NuGet's `["licenses", "nuget", "org"]` pattern.

**Python** (query.py:255-264):

```python
spdxid = [dic_get(u'spdx'), dic_get(u'license'), dic_get(u'identifier')]
nuget_spdx_id = [dic_get(u'licenses'), dic_get(u'nuget'), dic_get(u'org')]
self.spdx_lid_token_ids = [x for x in [spdxid, nuget_spdx_id] if x != [None, None, None]]
```

**Rust**:

```rust
let is_spdx_prefix = first_three == ["spdx", "license", "identifier"]
    || first_three == ["spdx", "licence", "identifier"];
// Missing: NuGet pattern check!
```

**Impact**: NuGet SPDX URLs like `https://licenses.nuget.org/MIT` won't be detected.

**Reference**: PLAN-023 mentions `nuget/nuget_test_url_155.txt` failing with empty result.

**Priority**: MEDIUM

---

### P4: hispan Reconstruction May Be Incorrect

**Status**: ⚠️ Related concepts in PLAN-014, PLAN-016, PLAN-017 but not this specific bug

**Location**: `match_refine.rs:119-126`

**Finding**: Rust stores only `hilen` (count), reconstructs hispan positions from `rule_start_token`. If original hispan wasn't a contiguous range, reconstruction is wrong.

**Python**: Stores `hispan` as actual `Span` (set of token positions).

**Rust**:

```rust
let a_hispan: HashSet<usize> = (a.rule_start_token..a.rule_start_token + a.hilen)
    .filter(|&p| a.ispan().contains(&p))
    .collect();
```

**Impact**: Edge cases where hispan is non-contiguous may have incorrect positions after merge.

**Priority**: MEDIUM

---

### P5: Extra `licensing_contains_match` Check in `filter_contained_matches`

**Status**: ✅ Already documented as intentional extension

**Location**: `match_refine.rs:357`

**Finding**: Rust adds `licensing_contains_match()` to `filter_contained_matches()`. Python does NOT use expression containment here.

**References**:

- PLAN-027: Documented as implemented extension
- PLAN-044: Documents this as "Low - extension, not parity issue"
- PLAN-046: Documents as "CONFIRMED BUG" but also notes it may be beneficial

**Current Decision**: Keep as intentional extension (filters more than Python, but correctly).

**Priority**: LOW - No action needed

---

## Priority Action Items

| Priority | Issue | Status | Action |
|----------|-------|--------|--------|
| P1 | `restore_non_overlapping` lines vs tokens | PLAN-030 | Implement when time permits |
| **P2** | `combine_matches` missing validation | **NEW** | Add `rule_identifier` equality check |
| P3 | Missing NuGet SPDX pattern | Partial | Add NuGet pattern to query.rs |
| P4 | hispan reconstruction | Partial | Consider storing positions explicitly |
| P5 | Extra licensing_contains check | Documented | No action (intentional) |

---

## Files to Modify

| File | Changes | Priority |
|------|---------|----------|
| `src/license_detection/match_refine.rs` | P2 (validation), P4 (hispan) | HIGH |
| `src/license_detection/query.rs` | P3 (NuGet pattern) | MEDIUM |

---

## Verification Steps

1. Implement P2 (add validation to `combine_matches`)
2. Run golden tests - check if regression changes
3. Implement P3 (NuGet pattern)
4. Run golden tests - check if regression changes
5. Consider P1 (PLAN-030) if still needed

---

## References

- PLAN-030: Full analysis of `restore_non_overlapping` issue
- PLAN-027: Expression combination fixes
- PLAN-044: `filter_contained_matches` parity analysis
- PLAN-046: Root cause analysis
- PLAN-047: The rid field implementation that triggered this investigation
