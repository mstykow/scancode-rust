# License Detection Implementation Plan

> **Status**: 🟡 Active — core engine implemented; output, CLI, and downstream parity gaps remain open
> **Priority**: P1 - High Priority Core Feature
> **Estimated Effort**: Multi-phase follow-up; depends on output and CLI wiring
> **Dependencies**: [LICENSE_DETECTION_ARCHITECTURE.md](../../LICENSE_DETECTION_ARCHITECTURE.md), [CLI_PLAN.md](../infrastructure/CLI_PLAN.md), [OUTPUT_FORMATS_PLAN.md](../output/OUTPUT_FORMATS_PLAN.md), [SCAN_RESULT_SHAPING_PLAN.md](../post-processing/SCAN_RESULT_SHAPING_PLAN.md), [PLAN-019-file-region-and-unique-detection.md](../../license-detection/PLAN-019-file-region-and-unique-detection.md)

## Overview

The Rust license-detection engine itself is implemented, but Provenant still has
real ScanCode parity gaps in the **public license-result surface**.

This plan tracks the missing work between the current engine and the user-facing
behavior users actually compare against Python ScanCode:

- file-level `license_detections` vs `license_clues`
- top-level unique `license_detections`
- top-level `license_references` and `license_rule_references`
- license diagnostics and matched-text diagnostics
- CLI flag parity for the remaining license options
- downstream consumers such as SPDX writers and clue filtering

The evergreen architecture document remains the source of truth for the
implemented engine internals. This plan is the temporary tracker for the parity
work that is still missing.

## Why This Plan Exists Again

The previous license-detection implementation plan was retired once the core
engine landed. That was correct for engine implementation, but it left no active
place to track the remaining parity work around output shape, CLI flags, and
downstream consumers.

Those gaps are now large enough and user-visible enough that they need an active
implementation plan again.

## Scope

### What This Covers

- File-level license output parity:
  - `license_detections`
  - `license_clues`
  - `detected_license_expression`
  - `detected_license_expression_spdx`
  - `percentage_of_license_text`
- Detection-level diagnostics and matched-text diagnostics:
  - `detection_log`
  - `matched_text`
  - `matched_text_diagnostics`
- Top-level license output parity:
  - unique `license_detections`
  - `license_references`
  - `license_rule_references`
- Package-level license-detection parity where it affects output and reporting:
  - `license_detections`
  - `other_license_detections`
- CLI parity for ScanCode license-related flags
- Downstream consumers blocked on these gaps, especially SPDX output and
  clue-related post-processing behavior

### What This Does Not Cover

- Copyright/email/URL detection parity (separate plans)
- Summary/tallies logic except where those reducers depend on missing license
  surfaces
- Package parser declared-license normalization that is already implemented
- General output-format plumbing unrelated to license semantics

## Current State in Rust

### Implemented

- ✅ Core multi-strategy license-detection engine
- ✅ Public file/package `license_detections`
- ✅ Public package `other_license_detections`
- ✅ `--license` CLI flag
- ✅ `--license-rules-path` CLI flag
- ✅ Rust `--include-text` flag for matched text in output
- ✅ Internal clue/reference-aware rule and match kinds
- ✅ Internal detection diagnostics (`detection_log`)
- ✅ Internal unknown-license engine support
- ✅ Top-level output model fields for `license_references` and
  `license_rule_references`
- ✅ `--from-json` round-trip preservation of preexisting
  `license_references` / `license_rule_references`

### Known Public Parity Gaps

- ❌ No emitted file-level `license_clues` field
- ❌ Clue-only internal detections are dropped before public serialization
- ❌ No emitted detection-level `detection_log`
- ❌ No emitted `matched_text_diagnostics`
- ❌ No emitted `percentage_of_license_text`
- ❌ No top-level unique `license_detections`
- ❌ No live generation of top-level `license_references`
- ❌ No live generation of top-level `license_rule_references`
- ❌ `--filter-clues` is not license-aware because clue semantics are not carried
  into the public output model
- ❌ SPDX output still hardcodes `NOASSERTION` / `NONE` instead of exporting real
  declared/detected license conclusions

### Known CLI Parity Gaps

- ❌ No `--license-score`
- ❌ No upstream-named `--license-text` flag (Rust currently exposes
  `--include-text` instead)
- ❌ No `--license-text-diagnostics`
- ❌ No `--license-diagnostics`
- ❌ No `--unknown-licenses`
- ❌ No `--license-references`
- ❌ No `--license-url-template`
- ⚠️ Upstream `--is-license-text` is no longer a live parity target; current
  parity should instead track the emitted `percentage_of_license_text` field

## Root Causes

### 1. Internal detection information is narrowed too early

The engine still distinguishes clue/reference-style matches and carries
`detection_log` internally, but the scanner conversion step currently emits only
the reduced public `LicenseDetection` shape.

That leaves Provenant without a public place to preserve:

- clue-only detections
- diagnostic classifications
- file-region-dependent aggregation metadata

### 2. Top-level license sections are modeled but not produced

`license_references` and `license_rule_references` already exist in the top-level
output schema, but live scans do not populate them yet.

Today they are effectively pass-through data for `--from-json`, not generated
native-scan output.

### 3. Unique-detection aggregation is still unimplemented

The engine and post-processing layers do not yet have the Python-style file
region and unique-detection aggregation step that feeds top-level license output
and reference-following behavior. The focused sub-plan for that work is
[PLAN-019-file-region-and-unique-detection.md](../../license-detection/PLAN-019-file-region-and-unique-detection.md).

### 4. CLI parity drift accumulated after the engine landed

The repository still has a mix of:

- current Rust-only flag names such as `--include-text`
- upstream flags that are not implemented yet
- outdated references to flags that are no longer current upstream targets

## Relationship to Other Plans

- **[CLI_PLAN.md](../infrastructure/CLI_PLAN.md)** owns the flag inventory and
  runtime CLI gating.
- **[SCAN_RESULT_SHAPING_PLAN.md](../post-processing/SCAN_RESULT_SHAPING_PLAN.md)**
  owns `--filter-clues`; this plan owns the missing license semantics that make
  license-aware clue filtering possible.
- **[OUTPUT_FORMATS_PLAN.md](../output/OUTPUT_FORMATS_PLAN.md)** and
  **[PARITY_SCORECARD.md](../output/PARITY_SCORECARD.md)** own format-specific
  output claims such as SPDX parity.
- **[PLAN-019-file-region-and-unique-detection.md](../../license-detection/PLAN-019-file-region-and-unique-detection.md)**
  owns the focused file-region and unique-detection design work used by this
  broader parity plan.

## Implementation Phases

1. **Phase 0 — Documentation and parity inventory**
   - Re-open an active license-detection plan
   - Correct stale CLI/output parity documents
   - Cross-link focused sub-plans

2. **Phase 1 — Public detection-shape parity**
   - Preserve enough internal detection classification to emit
     `license_clues` and `detection_log`
   - Add `matched_text_diagnostics`
   - Add `percentage_of_license_text`

3. **Phase 2 — Top-level license aggregation parity**
   - Implement unique-detection aggregation
   - Emit top-level `license_detections`
   - Generate live `license_references` and `license_rule_references`

4. **Phase 3 — CLI flag parity**
   - Resolve `--include-text` vs `--license-text`
   - Add remaining ScanCode-compatible license flags
   - Align help text and docs with the actual runtime surface

5. **Phase 4 — Downstream consumer parity**
   - Make `--filter-clues` license-aware where appropriate
   - Feed SPDX writers with real license conclusions/info-from-files
   - Audit summary/tally consumers of package `other_license_detections`

## Verify-First Gap List

These are the highest-risk user-visible parity gaps and should be verified first
whenever work resumes here:

1. file-level `license_clues`
2. `detection_log`
3. `matched_text_diagnostics`
4. top-level unique `license_detections`
5. live `license_references` / `license_rule_references`
6. `percentage_of_license_text`
7. `--license-text` naming parity
8. SPDX license conclusion/info export

## Success Criteria

- [ ] Provenant emits the ScanCode-style split between `license_detections` and
      `license_clues`
- [ ] License diagnostics are available when the corresponding CLI behavior is
      enabled
- [ ] Top-level unique `license_detections` are generated on native scans
- [ ] `license_references` and `license_rule_references` are generated on native
      scans instead of only being preserved from input JSON
- [ ] The CLI plan accurately reflects the implemented and pending license flags
- [ ] SPDX writers consume real license data instead of placeholder conclusions
- [ ] Evergreen docs describe the current public output shape accurately

## Related Documents

- [LICENSE_DETECTION_ARCHITECTURE.md](../../LICENSE_DETECTION_ARCHITECTURE.md)
- [CLI_PLAN.md](../infrastructure/CLI_PLAN.md)
- [OUTPUT_FORMATS_PLAN.md](../output/OUTPUT_FORMATS_PLAN.md)
- [PARITY_SCORECARD.md](../output/PARITY_SCORECARD.md)
- [SCAN_RESULT_SHAPING_PLAN.md](../post-processing/SCAN_RESULT_SHAPING_PLAN.md)
- [PLAN-019-file-region-and-unique-detection.md](../../license-detection/PLAN-019-file-region-and-unique-detection.md)
- [GAPS.md](../../license-detection/GAPS.md)

## Notes

- `GAPS.md` remains the place for deferred license-detection gaps that we are
  intentionally not fixing right now. This plan is for active parity work.
- Upstream documentation around some license flags has drifted over time;
  fixture-backed and code-backed behavior should remain the primary parity target.
