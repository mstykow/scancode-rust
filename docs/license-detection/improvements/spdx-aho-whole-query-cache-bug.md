# Python SPDX/AHO Whole-Query Cache Bug

**Date**: 2026-03-12
**Component**: Python reference matcher pipeline
**Type**: Reference Bug Documentation

## Problem

The Python reference intends to subtract SPDX-line tokens before later matchers run, but exact AHO still sees those tokens in normal `match_query()` flow.

This happens for the OpenJ9/OpenJDK composite family and any similar file where:

- SPDX matching runs first,
- the whole-query run was already cached,
- exact AHO reuses that cached whole-query run.

## Root Cause

`match_query()` creates and caches `whole_query_run` before any matcher runs in `reference/scancode-playground/src/licensedcode/index.py:983`.

SPDX matching then subtracts the SPDX match span in `reference/scancode-playground/src/licensedcode/index.py:672` via:

- `Query.subtract()` in `reference/scancode-playground/src/licensedcode/query.py:328`
- `QueryRun.subtract()` in `reference/scancode-playground/src/licensedcode/query.py:863`

But `Query.whole_query_run()` is memoized in `reference/scancode-playground/src/licensedcode/query.py:306`, and its cached `_high_matchables` / `_low_matchables` are computed lazily and then retained in `reference/scancode-playground/src/licensedcode/query.py:845` and `reference/scancode-playground/src/licensedcode/query.py:857`.

Exact AHO then reuses that stale cached run in `reference/scancode-playground/src/licensedcode/index.py:682`, and `match_aho.exact_match()` trusts `query_run.matchables` in `reference/scancode-playground/src/licensedcode/match_aho.py:108` and `reference/scancode-playground/src/licensedcode/match_aho.py:141`.

So the subtraction updates `Query.high_matchables` / `Query.low_matchables`, but not the already-cached whole-query snapshot used by AHO.

## Consequence

Python's effective behavior is not the same as its apparent intent:

- SPDX matching finds the SPDX identifier line.
- Exact AHO can still match the same SPDX-line tokens.
- Approximate matching can also observe stale whole-query state when near-duplicate logic uses `whole_query_run` in `reference/scancode-playground/src/licensedcode/index.py:744`.

For `reference/scancode-playground/tests/licensedcode/data/datadriven/external/spdx/complex-readme.txt:12`, current Python keeps the exact AHO tag rule at `reference/scancode-playground/src/licensedcode/data/rules/epl-2.0_or_apache-2.0_or_gpl-2.0_with_classpath-exception-2.0_or_gpl-2.0_with_openjdk-exception_5.RULE:8` visible even after SPDX subtraction.

If the whole-query run is rebuilt after SPDX subtraction, that exact AHO tag disappears and only the synthetic SPDX match remains for that line.

## Why `matched_qspans` Does Not Fix This

`already_matched_qspans` is only updated after matches are produced in `reference/scancode-playground/src/licensedcode/index.py:1056`.

It is then used for:

- stop checks in `reference/scancode-playground/src/licensedcode/index.py:1061`
- approximate prechecks in `reference/scancode-playground/src/licensedcode/index.py:828`

It does not alter the cached `whole_query_run` that exact AHO consumes.

## Recommended Rust Behavior

Rust should intentionally mimic Python's **effective** behavior for parity: exact AHO should still be able to see SPDX-line tokens during this stage.

However, Rust should not reproduce the bug as an accidental stale cache.

The safest analogue is to model this explicitly:

- keep an immutable whole-query snapshot for exact AHO stage input,
- keep separate mutable query matchable state for SPDX bookkeeping and later filtering,
- keep `matched_qspans` as an explicit overlay, not as hidden cache coupling.

This preserves current Python-visible results while avoiding dependence on stale cached `QueryRun` internals.

## Suggested Python Fix

If the Python reference were to be fixed independently, the minimal semantic fix would be one of:

- invalidate `Query._whole_query_run` when `Query.subtract()` changes query matchables, or
- stop creating `whole_query_run` before SPDX matching, or
- pass explicit matchable spans into exact/approximate matching instead of relying on cached `QueryRun` state.

That fix would change current output for files like `reference/scancode-playground/tests/licensedcode/data/datadriven/external/spdx/complex-readme.txt` and should therefore be treated as a behavioral change, not a refactor.
