# Improvement: Correct Handling of Solo Match Exception

## Python Bug

Python's `filter_matches_missing_required_phrases()` at `match.py:2172-2175` has a bug in the solo match exception:

```python
# never discard a solo match, unless matched to "is_continuous" or "is_required_phrase" rule
if len(matches) == 1:
    rule = matches[0]  # BUG: This is a LicenseMatch, not a Rule!
    if not (rule.is_continuous or rule.is_required_phrase):
        return matches, []
```

### The Bug

1. `rule = matches[0]` assigns a `LicenseMatch` object, not a `Rule` object
2. `rule.is_continuous` accesses a **method** on LicenseMatch (not calling it), which in Python is a method object that is always truthy
3. The condition `not (truthy_method_object or ...)` evaluates to `False`
4. The solo exception **never triggers** - the return statement is never executed

### Consequences

- All solo matches go through the full filter, including unknown/stopword checks
- Matches that might have been protected by the exception are still filtered
- This affects solo matches to rules without `is_continuous` or `is_required_phrase` flags

## Rust Implementation

Rust correctly implements the solo exception:

```rust
if matches.len() == 1
    && let Some(rid) = parse_rule_id(&matches[0].rule_identifier)
    && let Some(rule) = index.rules_by_rid.get(rid)  // Correctly gets the Rule
    && !(rule.is_continuous || rule.is_required_phrase)  // Correctly checks boolean fields
{
    return (matches.to_vec(), Vec::new());
}
```

However, this correct behavior causes **different results** from Python:

- Rust keeps solo matches to non-continuous/non-required-phrase rules immediately
- Python still runs these matches through the filter

## Fix for Compatibility

To achieve bug-for-bug compatibility with Python, **remove the solo exception** from Rust. This ensures:

1. Identical behavior to Python
2. All matches go through the same filter logic
3. Golden tests pass with expected results

## See Also

- `docs/license-detection/BUG-007-solo-exception-difference.md` - Detailed analysis
- Python reference: `reference/scancode-toolkit/src/licensedcode/match.py:2172-2175`
