# Summary, Tallies & Analysis Implementation Plan

> **Status**: 🟡 In Progress — shared provenance cleanup, the full current tally stack, file facets/by-facet tallies, package-preferred summary origin, generated-file detection, initial summary/tally CLI gating, complete active `score/` fixture parity, and a broad active summary parity subset are implemented; package tallies and only the residual summary/classify edge cases remain open
> **Priority**: P2 - Medium Priority (Post-Processing Feature)
> **Estimated Effort**: 3-4 weeks
> **Dependencies**: [LICENSE_DETECTION_ARCHITECTURE.md](../../LICENSE_DETECTION_ARCHITECTURE.md), [COPYRIGHT_DETECTION_PLAN.md](../text-detection/COPYRIGHT_DETECTION_PLAN.md), [ASSEMBLY_PLAN.md](../package-detection/ASSEMBLY_PLAN.md)

## Overview

This plan covers the remaining ScanCode-compatible **summary and tally surface**: codebase summary output, codebase/file/directory tallies, license clarity scoring, key-file classification, facets, and generated-code detection.

These features are the main post-processing value layer that turns raw file/package findings into project-level answers users actually consume: what the project is primarily licensed under, which files are key licensing files, how clear the licensing story is, and which licenses/copyrights/packages dominate the scan.

Upstream implements these behaviors across multiple plugins (`--summary`, `--tallies`, `--license-clarity-score`, `--classify`, `--facet`, `--generated`). Provenant tracks them in one plan because they share the same data flow and should be implemented against one coherent summary model.

## Recommendation

**Implement summarization next.**

Why:

- It is the broader, non-deprecated parity surface in ScanCode.
- Provenant already ships meaningful foundations for it.
- It unlocks multiple pending CLI options and user-facing outputs.
- Consolidation has now been intentionally deferred as a compatibility-only feature, so summarization is the clear remaining post-processing priority.

The practical order should be:

1. shared provenance cleanup in `src/main.rs` so key-file enrichment stops implicitly redefining package declared-license fields
2. summarization parity work (summary, tallies, clarity, classify/facet/generated support)

## Why This Feature Is Justified

For a drop-in ScanCode replacement, summarization is **not optional**.

- Official ScanCode docs still present `--summary`, `--tallies`, `--license-clarity-score`, `--classify`, `--facet`, and `--generated` as active user-facing features.
- These are the main project-level reporting features used for triage, dashboards, compliance review, and quick human interpretation of a scan.
- Provenant already has a partial summary foundation, so finishing this area yields high parity value for relatively low architectural risk.

## Upstream Parity Targets

The current parity target is the actual reference implementation and test surface, not every stale name in upstream docs.

### Must-match upstream behavior

- `--classify`
- `--facet`
- `--generated`
- `--license-clarity-score`
- `--summary`
- `--tallies`
- `--tallies-with-details`
- `--tallies-key-files`
- `--tallies-by-facet`

### Naming caveat

Upstream documentation still mentions names such as `--summary-key-files`, `--summary-by-facet`, and `--summary-with-details` in some places, but the live implementation and active plugin/test surface use the `--tallies-*` family. Provenant should target the real implemented CLI surface and, if helpful, document the upstream doc drift rather than reproducing it as a separate feature set.

## Architectural Boundary

Summarization is a **consumer**, not a normalizer.

- **Parsers** should already provide manifest-derived declared package-license data when the source field is trustworthy enough to normalize.
- **Summarization** should read package-declared metadata plus discovered file/resource evidence and turn them into:
  - tallies
  - clarity scoring
  - scan-level summary expressions
  - classification/facets
- **Summarization should not become the primary place that decides a package's declared license**.

## Scope

### What This Covers

- **Scan Summary**: Top-level project summary output (`declared_license_expression`, holder/language summaries, other values)
- **License Tallies**: Count and categorize licenses across the codebase
- **Copyright Tallies**: Aggregate copyright holders and statements
- **Package Tallies**: Count packages by ecosystem
- **License Clarity Score**: Calculate project-level license clarity metrics
- **Key-File Classification**: Mark likely licensing/manifest/readme files for summary logic and user inspection
- **Facet Assignment**: Tag files with facets (`core`, `dev`, `tests`, `docs`, `data`, `examples`)
- **Generated Code Detection**: Identify auto-generated files
- **Detailed Tallies**: File- and directory-level tallies where upstream exposes them via `--tallies-with-details`

### What This Doesn't Cover

- License policy evaluation (separate feature)
- Package consolidation (covered by `CONSOLIDATION_PLAN.md` in this directory)
- Output formatting (covered by OUTPUT_FORMATS_PLAN.md)

## Python Reference Implementation

**Location**: `reference/scancode-toolkit/src/summarycode/`

**Key Components**:

- `tallies.py` - License, copyright, package, author, holder, and language tallies
- `score.py` - License clarity scoring
- `classify_plugin.py` - `--classify` plugin and key-file flags
- `classify.py` - File classification rules
- `facet.py` - `--facet` behavior and facet partitioning
- `generated.py` - Generated code detection
- `summarizer.py` - `--summary` generation and reuse of tallies/clarity logic
- `copyright_tallies.py` - Copyright statistics

### Upstream Value Surface

- `--summary` gives users a top-level project view instead of forcing file-by-file interpretation.
- `--tallies` and `--tallies-with-details` support reporting, dashboards, and inventory analysis.
- `--tallies-key-files` narrows reporting to the files most likely to represent project-level licensing intent.
- `--tallies-by-facet` separates shipping code from tests/docs/examples.
- `--license-clarity-score` gives a triage-friendly confidence signal for how clearly licensing is stated.

## Current State in Rust

### Implemented

- ✅ Basic scan statistics (file count, scan time)
- ✅ Output format structure
- ✅ Top-level `summary` output block
- ✅ Top-level `tallies` output block
- ✅ Key-file tagging foundations (`is_legal`, `is_manifest`, `is_readme`, `is_top_level`, `is_key_file`)
- ✅ Package metadata promotion from key files
- ✅ Shared provenance cleanup so key-file license clues no longer mutate package declared-license fields or package detections
- ✅ Initial `license_clarity_score` model/output
- ✅ Core codebase tallies for:
  - `detected_license_expression`
  - `copyrights`
  - `holders`
  - `authors`
  - `programming_language`
- ✅ `tallies_of_key_files` for key-file-only aggregation over the same top-level tally families
- ✅ Per-resource `files[*].tallies` rollups for files and directories over those same tally families
- ✅ File-level `facets` assignment with the ScanCode facet set (`core`, `dev`, `tests`, `docs`, `data`, `examples`)
- ✅ Top-level `tallies_by_facet` buckets over the existing five tally families
- ✅ Initial non-license-dependent summary fields:
  - `declared_holder`
  - `primary_language`
  - `other_languages`
- ✅ Package-preferred summary origin data for:
  - `declared_license_expression`
  - `declared_holder`
  - `primary_language`
- ✅ Initial summary parity rollups for:
  - `other_license_expressions`
  - `other_holders`
- ✅ Initial license-clarity penalties for:
  - ambiguous compound licensing (`-10`)
  - conflicting lower-level license categories (`-20`)
- ✅ Broader classify substrate for top-level/community file handling on normal root-prefixed scans
- ✅ Generated-file detection (`is_generated`) from ScanCode-style conspicuous header clues
- ✅ Initial CLI gating for:
  - `--summary`
  - `--license-clarity-score`
  - `--tallies`
  - `--tallies-key-files`
  - `--tallies-with-details`
  - `--generated`
- ✅ Active summary/score parity improvements for:
  - joined-expression primary-license resolution without false ambiguity
  - score-only mode using key-file resource evidence instead of package-only origin data
  - package-datafile holder fallback ahead of global key-file holder fallback
  - `other_holders` retaining null buckets while pruning only declared holders
  - top-level/community classification on normal root-prefixed scans feeding summary/score correctly
  - empty declared-holder output when no holder can be established
  - primary-language fallback from tallied sources when top-level packages disagree
  - multi-holder aggregation for a single top-level key file
  - score parity for `no_license_text` and `no_license_or_copyright`
  - score parity for single joined-expression declarations without false ambiguity
  - score parity for nested manifest-style key files without declared copyrights
- ✅ Broader classify parity for active fixtures:
  - resource-level `is_top_level` on root directories and their direct children
  - `is_legal` / `is_readme` checks using both file name and base name
  - path-suffix manifest detection across the wider ScanCode manifest set
  - package-data files still treated as manifests when present
  - package-data ancestry now promotes manifest/legal siblings and ancestor directories into the top-level package view where the active `with_package_data` fixture expects it

### Missing

- ❌ Package tallies
- ❌ Full Python-parity license clarity scoring heuristics
- ❌ Full ScanCode `--classify` parity (including remaining classification nuances)
- ❌ Remaining CLI gating/compatibility edge cases for summary/tally/classify/facet/generated options
- ❌ Comprehensive scan summary parity

### Already handled elsewhere

- ✅ Parser-side normalization of trustworthy declared package-license metadata
- ✅ Initial summary consumption of package/key-file declared license data
- ✅ Initial package metadata promotion from key files

### Concrete follow-up before deeper summary parity work

- Shared provenance cleanup is complete.
- `promote_package_metadata_from_key_files(...)` now limits key-file promotion to copyright and holder enrichment.
- Remaining summary work can build on explicit summary/tally outputs instead of implicit package declared-license mutation.

## Implementation Phases

1. **Phase 0**: Shared provenance cleanup so package declared-license fields are no longer implicitly redefined from key-file evidence. ✅
2. **Phase 1**: File classification and key-file tagging foundations ✅
3. **Phase 2**: Package/file metadata promotion foundations ✅
4. **Phase 3**: Initial summary model/output structure ✅
5. **Phase 4**: Initial non-license-dependent summary fields ✅
6. **Phase 5**: Core codebase tallies (`--tallies`) over existing declared/discovered evidence. ✅ for top-level `detected_license_expression`, `copyrights`, `holders`, `authors`, and `programming_language`; package tallies remain open.
7. **Phase 6**: Detailed tally variants (`--tallies-with-details`, `--tallies-key-files`, `--tallies-by-facet`). 🟡 Top-level `tallies_of_key_files`, per-resource `files[*].tallies`, and top-level `tallies_by_facet` are implemented; package tallies and some CLI gating remain open.
8. **Phase 7**: Full license clarity parity. ✅ Complete for the active emitted ScanCode `score/` fixture surface (`basic`, `no_license_text`, `no_license_or_copyright`, `no_license_ambiguity`, `inconsistent_licenses_copyleft`, and `jar`), including joined-expression resolution, score-only key-file evidence, manifest allowlist behavior, ambiguity/conflict penalties, and the current holder-driven score cases.
9. **Phase 8**: Generated-code detection parity plus remaining classify/facet parity gaps. 🟡 Implemented: file-level `is_generated` detection from conspicuous header clues plus the main active classify fixture parity around legal/readme/manifest/community/top-level semantics, including package-data-backed top-level ancestry. Remaining work: heuristic breadth and any residual classify gaps.
10. **Phase 9**: Comprehensive `--summary` parity over the completed tally/clarity/classification inputs. 🟡 Implemented: package-preferred origin fields, `other_license_expressions`/`other_holders`, package-datafile holder fallback, empty declared-holder parity, tallied-language fallback when packages disagree, and the main active ambiguity/holder fixtures. Remaining work: broader package-precedence and the residual summary edge-case fixtures.
11. **Phase 10**: CLI parity wiring for the remaining summary/tally/classify/facet/generated options and regression coverage. 🟡 Implemented: `--summary`, `--license-clarity-score`, `--tallies`, `--tallies-key-files`, `--tallies-with-details`, and `--generated` gating. Remaining work: package-tally CLI surface and broader compatibility edge cases.

## Success Criteria

- [ ] Generates accurate codebase tallies for licenses, copyrights, packages, holders, authors, and languages where upstream does
- [ ] Produces `--summary` output compatible with the ScanCode reference for covered scenarios
- [ ] Calculates license clarity score matching Python semantics
- [ ] Classifies key files and broader file categories compatibly with ScanCode
- [ ] Supports facet-driven and key-file-driven tally variants
- [ ] Detects generated code with documented heuristic behavior
- [ ] Exposes the corresponding CLI options with parity-compatible semantics
- [ ] Golden and integration tests pass

## Related Documents

- **Evergreen**: [`docs/LICENSE_DETECTION_ARCHITECTURE.md`](../../LICENSE_DETECTION_ARCHITECTURE.md) — implemented license-detection engine and match pipeline
- **Implementation**: [`COPYRIGHT_DETECTION_PLAN.md`](../text-detection/COPYRIGHT_DETECTION_PLAN.md) — prerequisite
- **Implementation**: [`ASSEMBLY_PLAN.md`](../package-detection/ASSEMBLY_PLAN.md) — prerequisite
- **Evergreen**: [`docs/ARCHITECTURE.md`](../../ARCHITECTURE.md) — broader processing pipeline

## Notes

- Some summarization foundations can land before full detector parity (for example key-file tagging, package metadata promotion, initial summary fields, and primary-language/holder derivation).
- Full parity for tallies and Python-style scoring still depends on richer discovered-license/copyright coverage and clearer package-vs-file provenance.
- The recent parser-side declared-license normalization work reduces one gap for summarization consumers, but it does not remove the need for summary tallies, facets, generated-code detection, or scan-level aggregation.
- This plan intentionally groups some upstream pre-scan/scan/post-scan features together because they converge on one post-processing summary surface in Provenant.
- Implement incrementally, but preserve the user-visible dependency chain: classification/facets/generated feed tallies and clarity; tallies and clarity feed full summary parity.
- License clarity score is a key metric for compliance teams.
