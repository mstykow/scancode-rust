# Session Handoff

## Current objective

- Continue reducing release-mode golden test failures by aligning Rust license detection with the Python reference.
- The current active target is **GPL candidate ranking parity**, centered on `testdata/license-golden/datadriven/lic1/gpl-2.0-plus_33.txt` and `src/license_detection/seq_match/candidates.rs`.
- Secondary queued targets remain `testdata/license-golden/datadriven/unknown/scea.txt`, `testdata/license-golden/datadriven/unknown/cigna-go-you-mobile-app-eula.txt`, and an `is_spdx_exception()` audit.
- User rule: do not invent new heuristics when Python behavior is known; prefer the smallest Python-aligned fix.

## Process constraints from the user

- Never implement without planning first.
- Use subagents for investigation, planning, implementation, and verification whenever avoidable.
- Every subagent must be explicitly told to read `AGENTS.md`.
- Every new plan must be verified by a separate no-code-change subagent against:
  - current codebase state,
  - Python reference alignment,
  - completeness/detail,
  - `docs/TESTING_STRATEGY.md`.
- After implementation, use another no-code-change subagent to verify completeness, Python alignment, tests, ignored tests, coverage, and dead/unused code.
- Commit whenever the full golden failing count improves; include before/after counts in the commit message.
- Local/manual golden runs must use `--release` only. This guidance has already been added to `AGENTS.md`.

## Important repo state

- Working tree is dirty. `git status --short` currently shows user/worktree changes in:
  - `.gitignore`
  - `AGENTS.md`
  - `src/license_detection/detection/analysis.rs`
  - `src/license_detection/detection/mod.rs`
  - `src/license_detection/golden_test.rs`
  - `src/license_detection/models/license.rs`
  - `src/main.rs`
  - `src/utils/text.rs`
  - untracked `debug_gpl3_candidate.py`
- These changes were already present outside the latest parity-only commit work. Do not revert unrelated user changes.
- Latest committed state on this branch is now `56261c96` - `Use qspan_bounds() for overlap filtering parity (18->18)`.
- `SESSION.md` is tracked now and should be kept current as work progresses.

## Confirmed baseline and prior good commits

- Safe committed baseline: `13f10ef2`
- Full release golden failing count at that baseline: `51`
- Good commits already landed:
  - `572fb467` - `Fix SPDX-LID OR-chain rendering and require release golden runs (56->54)`
  - `47badd24` - `Fix OpenJ9 SPDX/AHO notice matching (54->53)`
  - `13f10ef2` - `Fix Unicode seq-container collapse in SPDX HTML (53->51)`

## What those earlier fixes accomplished

- Fixed SPDX expression rendering parity in files like `complex3.java` and `misc.c`.
- Improved OpenJ9/SPDX notice-family handling for files like:
  - `testdata/license-golden/datadriven/external/spdx/complex2.html`
  - `testdata/license-golden/datadriven/external/spdx/complex-readme.txt`
  - `testdata/license-golden/datadriven/external/spdx/uboot.c`
- Added a narrow raw-match fix for `complex-short.html`, reducing failures from `53` to `51`.

## Failed experiments that were rolled back

These were tried and then abandoned because they caused broader regressions:

- Broad exact-vs-seq containment override -> `228` failures.
- Broad post-AHO subtraction of long exact license-text matches -> `67` failures.
- Broadened seq-wrapper collapse for `securable-module.js` -> `58` failures.
- Exact-two-segment wrapper attempt for `securable-module.js` -> `75` failures.

After each failed experiment, the repo was restored to the safe `51`-failure baseline.

## Golden test harness facts

- Golden tests compare raw `detect_matches()` output, not grouped detections.
- Confirmed in `src/license_detection/golden_test.rs:163`, `src/license_detection/golden_test.rs:167`, `src/license_detection/golden_test.rs:172`, `src/license_detection/golden_test.rs:184`.
- This means fixes for golden mismatches often need to target raw matching behavior rather than grouping/detection post-processing.

## Active target: `png.h`

### Fixture and expected output

- File: `testdata/license-golden/datadriven/external/slic-tests/png.h`
- Expected YAML: `testdata/license-golden/datadriven/external/slic-tests/png.h.yml`
- Expected `license_expressions`:
  - `libpng`
  - `libpng`

### Python/reference behavior

- Python final raw matches for `png.h` are exactly:
  - `libpng_27.RULE` via `2-aho` near line 8
  - `libpng.SPDX.RULE` via `3-seq` across the later long libpng block
- Python builds real query runs in `reference/scancode-toolkit/src/licensedcode/query.py:527` and `reference/scancode-toolkit/src/licensedcode/query.py:568`.
- Python approximate matching iterates those runs in `reference/scancode-toolkit/src/licensedcode/index.py:724`.
- Python containment/overlap ordering relevant to this case:
  - `filter_contained_matches()` sorts by `(qspan.start, -hilen(), -len(), matcher_order)` at `reference/scancode-toolkit/src/licensedcode/match.py:1099`.
  - `filter_overlapping_matches()` uses the same sort key at `reference/scancode-toolkit/src/licensedcode/match.py:1220`.
  - Python has the `extra_large_next and current.len() >= next.len()` discard rule at `reference/scancode-toolkit/src/licensedcode/match.py:1326`.

### Latest confirmed root cause

The ambiguity about `png.h` is resolved:

- On the current branch, Rust does **not** produce the full `libpng.SPDX.RULE` raw match before refine.
- The loss point is **upstream of refine**: query-run construction is effectively disabled, so Phase 4 approximate matching never explores Python-like sub-runs for this file.
- Current Rust behavior:
  - final raw matches include `libpng_27.RULE` and `libpng_4.RULE`
  - full Python-style `libpng.SPDX.RULE` candidate is missing
- Why:
  - `detect_matches()` relies on query-run seq matching around `src/license_detection/mod.rs:578`
  - `Query::query_runs()` reads `query_run_ranges` at `src/license_detection/query/mod.rs:171`
  - real query-run construction is in `compute_query_runs()` and wired from `src/license_detection/query/mod.rs:330`
  - earlier investigation showed query-run construction had been disabled entirely; current code now has a real path in place, so any next agent must re-check actual current behavior before changing more code
- In the most recent no-code-change investigation, the confirmed explanation was:
  - Rust currently only effectively matches the whole run in the failing case, causing `libpng_4.RULE` to survive instead of the broader Python SPDX rule.
  - Python keeps `libpng.SPDX.RULE` because its query-run construction and approximate-match flow generate the larger candidate, after which containment removes the shorter `_4` rule.

## Subagent investigation result to trust

I ran a no-code-change subagent to reconfirm the current loss point. Its findings:

- `libpng.SPDX.RULE` is missing before Rust refine for `png.h` on the current branch.
- The minimal Python-aligned fix surface is:
  - `src/license_detection/query/mod.rs`
  - `src/license_detection/mod.rs`
- The actual gap is restoring Python-like query-run construction and making both detection entrypoints consume those runs the same way Python does.

## Verified plan for the next implementation

I used one planning subagent and one separate plan-verification subagent. The corrected plan is:

1. In `src/license_detection/query/mod.rs`, preserve per-line non-stopword known vs unknown token information for run classification.
2. Reintroduce Python-equivalent base query-run construction:
   - split on junk-line thresholds,
   - treat empty, unknown-only, low-only, and digits-only lines appropriately,
   - do not emit a final digits-only run.
3. Include Python long-line pseudo-line splitting (`MAX_TOKEN_PER_LINE = 25`) in that run-building path, or explicitly defer it with a strong rationale. The plan verifier said base parity is incomplete without considering this.
4. Keep `USE_RULE_STARTS` / `break_on_boundaries()` out of scope.
5. Replace any remaining disabled/stubbed query-run logic with real run construction, and rewrite tests that still encode disabled behavior.
6. In `src/license_detection/mod.rs`, extract a shared approximate-matching helper used by both `detect()` and `detect_matches()`:
   - near-dupe pass on whole query,
   - subtract matched spans,
   - synthetic OpenJ9 notice path,
   - regular sequence matching over `query.query_runs()`.
7. Remove the extra whole-query regular sequence phase that exists only in `detect_matches()`.
8. Add focused regressions:
   - unit tests in `src/license_detection/query/test.rs` for run-splitting behavior,
   - a raw-match regression in `src/license_detection/tests.rs` for `png.h` asserting the observable raw output.
9. Verify with focused tests, then `cargo test --all --verbose`, then release golden tests.

## Testing guidance from plan verification

- `docs/TESTING_STRATEGY.md:272` supports reproducing a bug with a unit test plus fix plus verification.
- `docs/TESTING_STRATEGY.md:408` and `docs/TESTING_STRATEGY.md:409` say final verification should include:
  - `cargo test --all --verbose`
  - `cargo test --all --verbose --features golden-tests`
- For local/manual golden work, still use release mode per `AGENTS.md`.
- The plan verifier warned that a full golden run is evidence, but not a zero-failure acceptance gate for the `png.h` work alone because unrelated failures like `securable-module.js` may remain.

## Existing targeted tests already in the tree

Relevant tests already present in `src/license_detection/tests.rs`:

- `test_png_h_detect_matches_recovers_bounded_libpng_seq_match` at `src/license_detection/tests.rs:662`
  - currently asserts a bounded `libpng` seq match around lines 362-386.
  - This may need updating because the verified plan expects Python-aligned raw behavior, likely a broader SPDX-backed `libpng` seq match and no leftover `unknown-license-reference` hits.
- Guardrail tests for the earlier SPDX/OpenJ9 work:
  - `test_spdx_complex_short_html_keeps_exact_unicode_matches_and_drops_seq_container`
  - `test_spdx_complex_readme_detect_matches_recovers_bounded_notice_preamble_seq_match`
  - `test_spdx_complex_readme_detect_matches_keeps_nearby_embedded_matches`

Relevant query-run tests already present in `src/license_detection/query/test.rs`:

- `test_query_run_splitting_single_run`
- `test_query_run_splitting_with_empty_lines`
- `test_query_run_splitting_below_threshold`
- `test_query_run_splitting_empty_query`

The plan verifier specifically said to add or update tests for:

- unknown-only lines counting toward threshold,
- low-only lines counting toward threshold,
- digits-only lines counting toward threshold but not becoming a final digits-only run,
- threshold boundaries,
- long-line splitting.

## `securable-module.js` status

- Still unresolved at the `51` baseline.
- Python keeps two exact AHO matches (`_1.RULE` and `_6.RULE`).
- Rust collapses them into one seq wrapper.
- Multiple fixes were tried and rolled back due regressions.
- Do not let `png.h` work accidentally paper over `securable-module.js` with broad heuristics.

## Relevant files for the active work

### Process and docs

- `AGENTS.md`
- `docs/TESTING_STRATEGY.md`

### Rust files

- `src/license_detection/mod.rs`
- `src/license_detection/query/mod.rs`
- `src/license_detection/query/test.rs`
- `src/license_detection/tests.rs`
- `src/license_detection/golden_test.rs`
- `src/license_detection/match_refine/mod.rs`
- `src/license_detection/match_refine/handle_overlaps.rs`

### Python reference

- `reference/scancode-toolkit/src/licensedcode/query.py`
- `reference/scancode-toolkit/src/licensedcode/index.py`
- `reference/scancode-toolkit/src/licensedcode/match.py`
- `reference/scancode-toolkit/src/licensedcode/data/rules/libpng.SPDX.RULE`
- `reference/scancode-toolkit/src/licensedcode/data/rules/libpng_4.RULE`

### Active fixtures

- `testdata/license-golden/datadriven/external/slic-tests/png.h`
- `testdata/license-golden/datadriven/external/slic-tests/png.h.yml`
- `testdata/license-golden/datadriven/external/slic-tests/securable-module.js`
- `testdata/license-golden/datadriven/external/slic-tests/securable-module.js.yml`

## Useful commands

Release-mode golden count:

```bash
cargo test --release -q --features golden-tests --lib license_detection::golden_test 2>&1 | tee /tmp/golden_tests.log | grep "failed, 0 skipped" | sed 's/.*, \([0-9]*\) failed,.*/\1/' | paste -sd+ | bc
```

Full release golden run for this area:

```bash
cargo test --release --features golden-tests --lib license_detection::golden_test
```

Non-golden suite expected by testing strategy before finishing:

```bash
cargo test --all --verbose
```

## Temporary artifacts from earlier investigation

These may still exist and can be useful for debugging, but should not be committed:

- `/tmp/golden_tests.log`
- `/tmp/png_stage_current.out`
- `/tmp/png_stage_initial_refine_items.log`
- `/tmp/png_stage_final_refine_items.log`
- `/tmp/png_current_probe.out`
- `/tmp/png_libpng_refine_trace.json`
- `/tmp/securable-rust.json`
- `/tmp/securable-python.json`
- `/tmp/securable-stage.out`
- `/tmp/securable-loss.out`
- `/tmp/securable-gaps.out`

## What was in progress when interrupted

- I had completed:
  - no-code-change reconfirmation of the `png.h` loss point,
  - a planning pass,
  - a separate verification pass on that plan.
- I then started an implementation subagent task for the query-run / approximate-match alignment work, but that task was interrupted before returning results.
- No implementation result from that interrupted subagent should be assumed.

## Recommended next steps for the next agent

1. Re-check current `src/license_detection/query/mod.rs` and `src/license_detection/mod.rs` before editing, because the working tree is dirty and some run-construction code is already present.
2. Resume with a fresh implementation step that follows the verified plan above.
3. After implementation, use a separate no-code-change subagent to verify:
   - Python alignment,
   - completeness,
   - tests and ignored tests,
   - dead/unused code.
4. Run focused tests, then `cargo test --all --verbose`, then release golden tests.
5. Commit only if the full failing golden count improves below `51`, and include before/after counts in the commit message.

## 2026-03-13 handoff update

- Dirty-tree release golden progression during today's work: `362 -> 90 -> 84`.
- No commit was created in this session; the safe committed baseline is still `13f10ef2` at `51` release golden failures.
- Confirmed keepers from today's work:
  - file-backed query runs stay enabled with the file/golden text-line threshold of `15`.
  - overlap/containment ordering should keep the Python-aligned `len()` tie-break, not `matched_length`.
  - same-expression seq-container filtering should keep using absolute `qspan_positions` rather than span endpoints only.
- Implemented in this session:
  - restored real query-run construction, including long-line pseudo-line splitting and junk-line threshold handling.
  - unified `detect()` and `detect_matches()` on the same approximate-match flow instead of keeping the extra whole-query regular-seq pass.
  - tightened redundant same-expression seq-container dropping with narrow bridge/boundary logic based on absolute qspan coverage.
  - added focused raw-match and query-run regressions around the restored behavior.
- Focused fixtures/guardrails verified passing in this dirty tree:
  - `testdata/license-golden/datadriven/external/slic-tests/png.h`
  - `testdata/license-golden/datadriven/lic3/libevent.LICENSE`
  - `testdata/license-golden/datadriven/external/OS-Licenses-master/zlib.txt`
  - query-run guardrails for unknown-only lines, low-only lines, final digits-only suppression, and long-line pseudo-line splitting.
  - seq-container guardrails for dropping tiny-gap/small-boundary wrappers while keeping a material boundary wrapper.
- This should not be committed yet: full `cargo test --all --verbose` / doctest gate is not clean, and dirty-tree release golden is still `84`, so the branch is not at a safe commit point.
- Next likely target cluster: post-query-run duplicate-collapse and over-merge fallout. Many remaining failures now look like Python keeping multiple raw detections while Rust collapses them to one, plus a smaller unknown/reference cluster.
- Representative remaining fixtures to investigate next:
  - duplicate-collapse cluster: `datadriven/external/glc/XFree86-1.1.t1`, `datadriven/external/glc/HaskellReport.t1`, `datadriven/external/OS-Licenses-master/bsd-2c.txt`, `datadriven/external/fossology-tests/Zlib/Zlib.txt`, `datadriven/lic4/hamcrest.txt`, `datadriven/external/slic-tests/2/NOTICE.txt`
  - unknown/reference or expression-shape cluster: `datadriven/lic1/curl.txt`, `datadriven/unknown/README.md`, `datadriven/lic4/disable_warnings.h`, `datadriven/lic4/openssh.LICENSE`, `datadriven/external/spdx/not-spdx`

## 2026-03-15 committed handoff update

- Latest committed state is `c94bc611` - `Align raw matching flow with Python parity (84->62)`.
- That commit moved the dirty-tree full release golden failing count from `84` to `62` and also passed `cargo test --all --verbose`.
- The parity work fixed a broad raw-matching/duplicate-collapse slice, including `datadriven/external/spdx/complex-readme.txt`, `datadriven/external/slic-tests/2/NOTICE.txt`, `datadriven/external/spdx/not-spdx`, `datadriven/lic4/openssh.LICENSE`, and related same-family cases.
- Safe historical baseline `13f10ef2` at `51` failures still exists, but the active branch is now intentionally committed at `62` because the matching flow is more Python-aligned than the old baseline.
- Confirmed current next likely targets are the `datadriven/lic4/standard-ml-nj_and_x11_and_x11-opengroup*` pair, `datadriven/lic4/x11_danse.txt`, and the separate synthetic-unknown issue in `datadriven/unknown/README.md`.
- Confirmed root-cause split for the `lic4_part2` cluster: the `standard-ml-nj_and_x11_and_x11-opengroup*` files are one expression-shape/raw-parity problem, while `x11_danse.txt` is a separate `unknown-license-reference` issue; `unknown/README.md` remains its own synthetic-unknown problem outside that split.

## 2026-03-15 threshold-preservation handoff update

- Current committed branch stack after the old safe baseline `13f10ef2` is now `c94bc611` and `0bc739eb`.
- New latest commit is `0bc739eb` - `Preserve stored rule thresholds during index build (62->60)`.
- `0bc739eb` fixed index-build threshold preservation so stored per-rule thresholds survive build-time processing, which removed the remaining `standard-ml-nj_and_x11_and_x11-opengroup*` regressions and dropped the full release golden count from `62` to `60`.
- During that threshold-preservation pass, `cargo test --all --verbose` passed; `cargo test --all --verbose --features golden-tests` still fails only in golden partitions.
- In `lic4_part2`, `testdata/license-golden/datadriven/lic4/x11_danse.txt` is now the only remaining failure.
- Likely next targets are `testdata/license-golden/datadriven/lic4/x11_danse.txt` whole-query near-dupe semantics and the separate synthetic-unknown issue in `testdata/license-golden/datadriven/unknown/README.md`.

## 2026-03-16 unknown-parity handoff update

- Current committed branch stack after `13f10ef2` is now `c94bc611`, `0bc739eb`, `e47232d6`.
- Latest commit is `e47232d6` - `Recover overlapping unknown matches for parity (61->57)`.
- Full release golden failing count is now `57`, and the `unknown` partition improved from `6` failures to `2`.
- `e47232d6` fixed overlapping unknown-match parity for `testdata/license-golden/datadriven/unknown/README.md`, `testdata/license-golden/datadriven/unknown/cisco.txt`, `testdata/license-golden/datadriven/unknown/ucware-eula.txt`, and `testdata/license-golden/datadriven/unknown/citrix.txt`.
- Remaining `unknown` fixtures are `testdata/license-golden/datadriven/unknown/cigna-go-you-mobile-app-eula.txt` and `testdata/license-golden/datadriven/unknown/scea.txt`.
- `testdata/license-golden/datadriven/lic4/x11_danse.txt` is still queued as the next focused non-unknown target.

## 2026-03-16 whole-query near-dupe handoff update

- Current committed branch stack after `13f10ef2` is now `c94bc611`, `0bc739eb`, `e47232d6`, `6788b66c`.
- Latest commit is `6788b66c` - `Snapshot whole-query near-dupe matching for parity (57->33)`.
- Full release golden failing count is now `33`, and `lic4_part2` is fully green.
- `6788b66c` fixed `testdata/license-golden/datadriven/lic4/x11_danse.txt` and the broader whole-query near-dupe parity family by snapshotting whole-query near-dupe matches before later span-subtraction/mutation changes could diverge from Python.
- Largest remaining failure clusters are `lic1`, `lic2_part1`, and the 2 remaining `unknown` fixtures: `testdata/license-golden/datadriven/unknown/cigna-go-you-mobile-app-eula.txt` and `testdata/license-golden/datadriven/unknown/scea.txt`.

## 2026-03-16 .LICENSE subtraction handoff update

- Current committed branch stack after `13f10ef2` is now `c94bc611`, `0bc739eb`, `e47232d6`, `6788b66c`, `9f18d580`.
- Latest commit is `9f18d580` - `Subtract long exact .LICENSE matches before seq (33->19)`.
- Full release golden failing count is now `19`.
- `9f18d580` fixed the long-exact-vs-seq mismatch for `.LICENSE`-style fixtures by subtracting long exact matches before regular seq matching, which resolved the Aladdin family including `testdata/license-golden/datadriven/lic2/aladdin-md5_and_not_rsa-md5.txt`.
- This disproved the earlier merge-only hypothesis: the Aladdin fix did not come from merge behavior, it came from the `.LICENSE` subtraction mismatch.
- Next likely focused targets are `testdata/license-golden/datadriven/lic1/eclipse-openj9.LICENSE`, `testdata/license-golden/datadriven/lic1/gpl-2.0-plus_33.txt`, `testdata/license-golden/datadriven/unknown/cigna-go-you-mobile-app-eula.txt`, and `testdata/license-golden/datadriven/unknown/scea.txt`.

## 2026-03-16 OpenJ9 uncovered-block handoff update

- Current committed branch stack after `13f10ef2` is now `c94bc611`, `0bc739eb`, `e47232d6`, `6788b66c`, `9f18d580`, `5990609d`.
- Latest commit is `5990609d` - `Limit synthetic OpenJ9 notice recovery to uncovered blocks (19->18)`.
- Full release golden failing count is now `18`.
- `5990609d` fixed `testdata/license-golden/datadriven/lic1/eclipse-openj9.LICENSE` by constraining synthetic OpenJ9 notice recovery so it only runs on still-uncovered blocks instead of re-wrapping already-covered content.
- `testdata/license-golden/datadriven/external/spdx/complex-readme.txt` remained green after this narrowing.
- Next likely focused targets are `testdata/license-golden/datadriven/unknown/scea.txt`, `testdata/license-golden/datadriven/unknown/cigna-go-you-mobile-app-eula.txt`, GPL candidate ranking in `testdata/license-golden/datadriven/lic1/gpl-2.0-plus_33.txt`, and the remaining `lic2_part1` cluster.

## 2026-03-16 OpenJ9 cleanup and validator directive handoff update

- Current committed branch stack after `13f10ef2` is now `c94bc611`, `0bc739eb`, `e47232d6`, `6788b66c`, `9f18d580`, `5990609d`, `5cf171b5`.
- Latest commit is `5cf171b5` - `Remove Rust-only OpenJ9 matching shortcuts (18->18)`.
- Full release golden failing count remains `18` after removing the Rust-only OpenJ9 path; no immediate golden improvement was required for this cleanup to be correct.
- OpenJ9 audit result: all runtime OpenJ9-specific matching logic lacked Python counterparts and was removed.
- New validator directive: future validation must prioritize Python logic-level parity over preserving outcomes from Rust-only branches; if a Rust branch has no Python counterpart, removal can be the correct fix even when the golden count does not immediately improve.
- Broader special-case audit result: the highest-risk remaining Rust-only candidate is `is_spdx_exception()`; NuGet special handling does exist in Python, but the current Rust handling is narrower rather than wholly Rust-only.
- Next likely focused targets are `testdata/license-golden/datadriven/unknown/scea.txt`, `testdata/license-golden/datadriven/unknown/cigna-go-you-mobile-app-eula.txt`, `testdata/license-golden/datadriven/lic1/gpl-2.0-plus_33.txt`, then an `is_spdx_exception()` audit.

## 2026-03-17 Candidate ranking investigation

- Investigation into `gpl-2.0-plus_33.txt` revealed the root cause is in **candidate ranking**, not containment filtering.
- Python ranks `gpl-3.0_561.RULE` as #229 in candidate list and only keeps top 70.
- Rust generates a `gpl-3.0` match, indicating candidate ranking differs.
- Two Python-parity fixes were made but didn't reduce golden count:
  1. `sort_matches_by_line()` now uses `len()` instead of `matched_length` (matching Python's `m.len()`)
  2. `filter_contained_matches()` and `filter_overlapping_matches()` now use `qspan_bounds()` instead of `end_token` directly
- These fixes are correct for Python parity but don't address the GPL issue.
- The GPL issue requires investigating `src/license_detection/seq_match/candidates.rs` scoring and ranking logic.
- Key Python reference: `match_set.py:354` - `candidates = sorted(filter_dupes(sortable_candidates), reverse=True)[:top]`
- ScoresVector fields (in priority order): `is_highly_resemblant`, `containment`, `resemblance`, `matched_length`
- `matched_length` is rounded as `round(matched_length / 20, 1)` in Python.
- Important likely parity bug: Python uses banker's rounding for `round(x, 1)` while Rust currently uses `f32::round()`-style half-away rounding on scaled values. Example confirmed locally: Python `round(4.35, 1) == 4.3`, while half-away rounding yields `4.4`.
- This can affect all rounded `ScoresVector` fields in Rust candidate ranking: `is_highly_resemblant`, `containment`, `resemblance`, and `matched_length`.
- Separate verification found that rounding is a real parity bug but likely **not** the sole GPL root cause: even with Python-style rounding, `gpl-3.0_561.RULE` still appears to remain outside Python's kept top-70 set.
- Higher-priority candidate-ranking parity gaps to check next:
  - step-1 pre-truncation sort parity before `top_n * 10`
  - multiset-phase `min_high_matched_length` / `min_matched_length` threshold filtering parity
  - dupe-group key quantization also needs Python-compatible rounding once the helper exists
- Suspected fix surface is `src/license_detection/seq_match/candidates.rs`, especially:
  - rounded score construction around `matched_length`
  - Python-compatible 1-decimal rounding helper
  - step-1 candidate sort/truncate parity
  - step-2 threshold filtering parity
  - `ScoresVector` ordering / comparison
  - duplicate filtering and final `top_n` truncation
- Full release golden count remains at 18.

## How to continue from here

1. Re-check `src/license_detection/seq_match/candidates.rs` against Python `reference/scancode-toolkit/src/licensedcode/match_set.py` before editing.
2. Focus on the `gpl-3.0_561.RULE` candidate path for `testdata/license-golden/datadriven/lic1/gpl-2.0-plus_33.txt`:
   - confirm Python ranking inputs for that candidate,
   - confirm Rust ranking inputs for the same candidate,
   - identify the first score component or tie-break that diverges.
3. Prioritize these candidate-ranking details in order:
   - step-1 candidate sort/truncate parity before `top_n * 10`,
   - multiset-phase threshold checks (`min_high_matched_length`, `min_matched_length`, `minimum_containment`),
   - Python `round(x, 1)` parity vs Rust float rounding,
   - `matched_length` scaling parity,
   - `ScoresVector` comparison order,
   - `filter_dupes()` parity,
   - final `top_n` truncation parity.
4. Only after the GPL ranking issue is understood, implement the smallest Python-aligned fix and add a focused regression test for `gpl-2.0-plus_33.txt` raw matches.
5. After implementation, use a separate no-code-change verification subagent, then run:
   - focused GPL tests,
   - `cargo test --all --verbose`,
   - `cargo test --release --features golden-tests --lib license_detection::golden_test`
6. Commit only if the full release golden failing count improves below `18`; include before/after counts in the commit message.

## Useful continuation commands for the next agent

```bash
# Current golden failing count
cargo test --release -q --features golden-tests --lib license_detection::golden_test 2>&1 | tee /tmp/golden_tests.log | grep "failed, 0 skipped" | sed 's/.*, \([0-9]*\) failed,.*/\1/' | paste -sd+ | bc

# Show current failing fixtures
cargo test --release --features golden-tests --lib license_detection::golden_test 2>&1 | grep "mismatch for"

# Python reference ranking area
read reference/scancode-toolkit/src/licensedcode/match_set.py

# Rust ranking area
read src/license_detection/seq_match/candidates.rs
```

## 2026-03-17 candidate-selection parity pass

- A Python-parity pass landed locally in `src/license_detection/seq_match/candidates.rs` and improved the full release golden count from `18` to `17`.
- Main changes in that pass:
  - step-1 candidate pre-truncation sort now uses rounded/full score ordering before `top_n * 10`
  - step-2 multiset refinement now enforces occurrence-based `min_high_matched_length` and `min_matched_length`
  - rounded candidate fields now use a Python-oriented 1-decimal helper instead of scaled Rust `round()`
  - final candidate ordering now falls back after rounded/full vectors consistently with Python tuple ordering
  - focused regressions were added in `src/license_detection/tests.rs` for:
    - `testdata/license-golden/datadriven/lic1/gpl-2.0-plus_33.txt`
    - `testdata/license-golden/datadriven/lic4/kde_licenses_test.txt`
    - `testdata/license-golden/datadriven/lic1/d-zlib_and_gfdl-1.2_and_gpl_and_gpl_and_other.txt`
- Confirmed improvements from this pass:
  - `testdata/license-golden/datadriven/lic1/gpl-2.0-plus_33.txt` is now green
  - `testdata/license-golden/datadriven/lic4/kde_licenses_test.txt` is now green
  - `testdata/license-golden/datadriven/lic1/d-zlib_and_gfdl-1.2_and_gpl_and_gpl_and_other.txt` is now green
- A no-code-change verification subagent confirmed this pass is broadly Python-aligned and does not appear to introduce Rust-only heuristics.

## New follow-up finding after the candidate pass

- The candidate pass also surfaced new ordering-sensitive regressions, especially:
  - `testdata/license-golden/datadriven/lic2/autoconf_aclocal.m4`
  - `testdata/license-golden/datadriven/external/fossology-tests/Dual-license/aclocal.m4`
  - `testdata/license-golden/datadriven/external/fossology-tests/Dual-license/Oracle+Sun_oracle_index.html`
  - `testdata/license-golden/datadriven/external/spdx/complex-short.html`
- Focused investigation on `testdata/license-golden/datadriven/lic2/autoconf_aclocal.m4` found the wrong Rust raw match is `lgpl-3.0_or_lgpl-2.1_1.RULE`.
- That divergence is **not** primarily in step-2 candidate refinement; it appears upstream in query-run shaping:
  - Python keeps that rule far below the step-1 cutoff on a larger file/location-based run.
  - Rust currently builds a smaller run for the same region, which lets the rule survive candidate selection.
- Important conclusion: the next likely fix surface is again `src/license_detection/query/mod.rs`, not another immediate rewrite of `candidates.rs`.
- Separate investigation suggests using `rid` as candidate tie-break is probably parity-safe here because Rust and Python shared approx-matchable rule ordering appears aligned by identifier.

## 2026-03-17 archive + seq-container parity pass

- Current uncommitted work reduces the full release golden count from `17` to `15`.
- Two fixes are responsible for that improvement:
  1. `src/utils/file.rs` now skips generic binary-string extraction for real `.jar` archives (extension + ZIP magic), which fixes `testdata/license-golden/datadriven/lic1/do-not_detect-licenses-in-archive.jar`.
  2. `src/license_detection/mod.rs` now keeps same-expression seq containers unless there are at least two material exact child matches, which fixes `testdata/license-golden/datadriven/lic1/complex.el` by preserving Python's `lgpl-2.0-plus_55.RULE` container.
- Added focused regressions for the seq-container fix in `src/license_detection/tests.rs`.
- No-code verification result:
  - the `.jar` fix is parity-oriented but narrower than Python's broader type-based archive gating
  - the seq-container fix is still Rust-local logic, but moves behavior toward Python and is constrained by targeted tests

## 2026-03-17 physical-line run-shaping fix

- Current uncommitted work reduces the full release golden count from `15` to `14`.
- The change is in `src/license_detection/query/mod.rs`: `compute_query_runs()` now evaluates line quality once per original physical line, even when long lines are split into 25-token pseudo-lines.
- This prevents Rust from splitting runs between synthetic chunks of the same source line.
- Confirmed direct impact:
  - `testdata/license-golden/datadriven/lic1/devdocs_README.md` is now green.
  - `lic1` partition improved from `2` failures to `1`.
- Important verifier caveat:
  - this is a tactical parity improvement, not a literal Python port.
  - Python's long-line handling is still driven by file/type metadata, while Rust still decides from raw text only.
  - So describe this as a targeted run-fragmentation fix, not as complete query parity.

## 2026-03-17 approx-matchable timing parity + stale snapshot cleanup

- Current uncommitted work reduces the full release golden count from `14` to `10`.
- Main parity change: `src/license_detection/index/builder/mod.rs` now computes `approx_matchable_rids` before final `is_small` / `is_tiny` flags are derived, matching Python's index-build ordering.
- Builder regressions were added in `src/license_detection/index/builder/tests.rs` to lock this timing behavior.
- Direct confirmed effects from this pass:
  - `testdata/license-golden/datadriven/lic2/autoconf_aclocal.m4` is now green
  - `testdata/license-golden/datadriven/external/license_tools/spdx-correct.js/test.js` is now green
  - `testdata/license-golden/datadriven/external/licensecheck/fedora/MIT` is now green
  - `testdata/license-golden/datadriven/external/fossology-tests/Dual-license/Oracle+Sun_oracle_index.html` is now green
- Snapshot cleanup:
  - `testdata/license-golden/datadriven/external/fossology-tests/Dual-license/aclocal.m4.yml` was stale relative to current Python and the mirrored upstream reference.
  - Local expected value was updated from `fsfap-no-warranty-disclaimer` to `fsf-ap`.
  - This was snapshot drift, not a Python logic bug.
- New side effect to watch:
  - `testdata/license-golden/datadriven/lic4/xunit.sln` is now failing with an extra `unknown-license-reference`; this appears to be a new cutoff/candidate-pool side effect from the broader approx-matchable parity fix.

## 2026-03-17 qspan-distance parity fix

- Current uncommitted work reduces the full release golden count from `10` to `8`.
- Main code fix: `src/license_detection/models/license_match.rs` now computes `qdistance_to()` using Python span-distance semantics (inclusive ends), rather than Rust's previous exclusive-end shortcut.
- This stops Rust from merging sparse `unknown-license-reference_339.RULE` fragments into synthetic 100% seq matches before minimum-coverage filtering.
- Focused regressions were added for:
  - `testdata/license-golden/datadriven/external/fossology-tests/BSD/BSD-2-Clause_AND_Imlib2.txt`
  - `testdata/license-golden/datadriven/lic4/xunit.sln`
  - unit coverage for gapped qspan distance in `src/license_detection/models/mod_tests.rs`
- Confirmed direct improvements:
  - `testdata/license-golden/datadriven/external/fossology-tests/BSD/BSD-2-Clause_AND_Imlib2.txt` is now green
  - `testdata/license-golden/datadriven/lic4/xunit.sln` is now green
  - `lic4_part2` is fully green again
- Remaining BSD note:
  - `testdata/license-golden/datadriven/external/fossology-tests/BSD/BSD-3-Clause_AND_CC0-1.0.txt` is a different issue: Rust already builds the Python `bsd-new_303.RULE` wrapper, then drops it in same-expression seq-container pruning.

## Current remaining failing fixtures after the qspan-distance fix (8 total)

- `datadriven/external/atarashi/CECILL-C.c`
- `datadriven/external/fossology-tests/BSD/BSD-3-Clause_AND_CC0-1.0.txt`
- `datadriven/external/spdx/complex-short.html`
- `datadriven/lic1/gpl_and_gpl_and_gpl_and_lgpl-2.0_and_other.txt`
- `datadriven/lic2/android-sdk-preview-2015.html`
- `datadriven/lic2/basename.elf`
- `datadriven/lic2/bsd-new_156.pdf`
- `datadriven/unknown/scea.txt`

## Recommended next target from here

1. Strongest next diagnostic targets now:
   - `testdata/license-golden/datadriven/lic2/android-sdk-preview-2015.html` for seq/refine overproduction
   - `testdata/license-golden/datadriven/lic1/gpl_and_gpl_and_gpl_and_lgpl-2.0_and_other.txt` as the last remaining `lic1` case
   - `testdata/license-golden/datadriven/external/fossology-tests/BSD/BSD-3-Clause_AND_CC0-1.0.txt` as a clean same-expression seq-container-pruning case
2. The current known full count is `8`.
3. Re-run the full release golden suite after every targeted fix; the approx-matchable pool and query-run changes have shown broad cutoff effects.
