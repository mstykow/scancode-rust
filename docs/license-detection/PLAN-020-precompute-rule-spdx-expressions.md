# Plan 020: Precompute Rule SPDX Expressions

## Context

Rust currently keeps ScanCode license expressions as the canonical internal rule
and detection identity, then derives SPDX expressions later during detection
assembly.

This is the right high-level model, but it leaves some rough edges:

- some matchers do not know a trustworthy SPDX expression at match creation time,
- SPDX conversion work happens later than necessary,
- and internal match data can drift toward placeholder values if the pipeline is
  not careful.

## Current Cleanup Direction

As an immediate improvement, internal `LicenseMatch.license_expression_spdx`
should remain optional and only be populated when the value is actually known.

## Follow-Up Idea

Precompute SPDX expressions for rules during rule/license loading or index
construction, while keeping ScanCode expressions as the canonical internal keyspace.

That would mean:

1. `Rule` keeps its existing ScanCode `license_expression`.
2. `Rule` also carries a precomputed optional SPDX rendering.
3. Exact/hash/aho/seq matchers can populate match-level SPDX expressions from the
   rule directly when available.
4. Detection assembly only needs late SPDX conversion as a fallback for cases
   that cannot be precomputed safely.

## Expected Improvements

- removes repeated SPDX conversion work from later pipeline stages,
- reduces the need for fallback logic in detection assembly,
- makes match-level SPDX data more trustworthy,
- avoids accidental reuse of ScanCode expressions in SPDX-only fields,
- keeps the internal ScanCode keyspace while improving output-facing data quality.

## Important Constraint

This should **not** eliminate the internal ScanCode keyspace.

ScanCode keys are still needed as the canonical internal identity because:

- not every license has an SPDX identifier,
- multiple ScanCode keys may map to the same SPDX id,
- and the rule/license database is authored in ScanCode keys.

So this plan is about earlier rendering, not replacing internal identity.
