# Implementation Plans

This directory contains **temporary planning documents** for porting Python ScanCode features to Rust. These are working documents that track implementation progress and will be archived once features are complete.

## Directory Structure

Plans are organized by major feature area:

```text
implementation-plans/
├── package-detection/     # Package manifest parsing and assembly
├── text-detection/        # License, copyright, email/URL detection
├── post-processing/       # Summarization, tallies, classification
├── output/                # Output format support (SPDX, CycloneDX, etc.)
└── infrastructure/        # Plugin system, caching, progress tracking
```

## Active Plans

### Package Detection (`package-detection/`)

- **[PARSER_PLAN.md](package-detection/PARSER_PLAN.md)** - Individual file format parser implementations
  - Status: 🟢 Complete — planned production parser/recognizer coverage is implemented; deferred and future-scope items are documented in [PARSER_PLAN.md](package-detection/PARSER_PLAN.md)

- **[ASSEMBLY_PLAN.md](package-detection/ASSEMBLY_PLAN.md)** - Package assembly roadmap
  - Status: 🟢 Complete — All phases done (sibling merge, nested merge, workspace assembly, file reference resolution)

- **[PARSER_ENHANCEMENT_PLAN.md](package-detection/PARSER_ENHANCEMENT_PLAN.md)** - Cross-cutting parser enhancement and shared declared-license normalization record
  - Status: 🟢 Complete — the shared parser-side declared-license normalization rollout is implemented, and the document is now kept as completed historical/reference documentation

### Text Detection (`text-detection/`)

- **[COPYRIGHT_DETECTION_PLAN.md](text-detection/COPYRIGHT_DETECTION_PLAN.md)** - Copyright statement extraction
  - Status: 🟢 Complete — scanner/runtime ingestion now covers decoded non-UTF text, PDF text, and binary printable strings; Rust also adds supported-image EXIF/XMP metadata as a beyond-parity clue source, and intentional divergences are tracked in the plan

- **[EMAIL_URL_DETECTION_PLAN.md](text-detection/EMAIL_URL_DETECTION_PLAN.md)** - Email and URL extraction
  - Status: 🟢 Complete — scanner/runtime ingestion now covers decoded non-UTF text, PDF text, and binary printable strings; Rust also adds supported-image EXIF/XMP metadata as a beyond-parity clue source, and intentional divergences are tracked in the plan

### Infrastructure (`infrastructure/`)

- **[CLI_PLAN.md](infrastructure/CLI_PLAN.md)** - Command-line interface parameter parity
  - Status: 🟡 Active — implemented and pending CLI parity items are tracked in [CLI_PLAN.md](infrastructure/CLI_PLAN.md)

- **[CACHING_PLAN.md](infrastructure/CACHING_PLAN.md)** - Scan result caching and incremental scanning
  - Status: 🟡 Active — cache CLI/runtime integration is tracked in [CACHING_PLAN.md](infrastructure/CACHING_PLAN.md)

- **[PROGRESS_TRACKING_PLAN.md](infrastructure/PROGRESS_TRACKING_PLAN.md)** - Enhanced progress reporting
  - Status: 🟢 Implemented — progress manager, mode handling, summary/reporting, and integration tests are tracked in the plan document

### Output Formats (`output/`)

- **[OUTPUT_FORMATS_PLAN.md](output/OUTPUT_FORMATS_PLAN.md)** - SPDX, CycloneDX, CSV, YAML, HTML output
  - Status: 🟢 Fixture-backed parity hardening complete across SPDX/CycloneDX/HTML/CSV/JSONL/YAML

- **[PARITY_SCORECARD.md](output/PARITY_SCORECARD.md)** - Format-by-format parity contract and fixture coverage
  - Status: 🟢 Maintained as the canonical output parity contract and verification checklist

## Placeholder Plans (To Be Fleshed Out)

These represent major architectural components not yet implemented. Each will be expanded into a detailed implementation plan when work begins.

### Text Detection (`text-detection/`)

- **[LICENSE_DETECTION_PLAN.md](text-detection/LICENSE_DETECTION_PLAN.md)** - License text detection and matching (transition placeholder)
  - Priority: P0 - Critical; active engine direction is the runtime ScanCode rule-loading architecture tracked in `feat-add-license-parsing`

### Post-Processing (`post-processing/`)

- **[SUMMARIZATION_PLAN.md](post-processing/SUMMARIZATION_PLAN.md)** - License/copyright tallies, facets, classification
  - Priority: P2 - Medium, Effort: 3-4 weeks

- **[CONSOLIDATION_PLAN.md](post-processing/CONSOLIDATION_PLAN.md)** - Resource grouping by origin, package enrichment with discovered licenses/copyrights
  - Priority: P2 - Medium, Effort: 2-3 weeks
  - Dependencies: License detection, copyright detection, package assembly

### Infrastructure (`infrastructure/`)

- **[PLUGIN_SYSTEM_PLAN.md](infrastructure/PLUGIN_SYSTEM_PLAN.md)** - Extensible plugin architecture
  - Priority: P3 - Low, Effort: 3-4 weeks

## Document Lifecycle

1. **Placeholder** - Brief description of component, scope, and dependencies
2. **Planning** - Detailed analysis, design decisions, implementation phases
3. **Active** - Work in progress, updated with status
4. **Complete** - Feature implemented, document retained as completed living documentation or archived if it no longer has ongoing maintainer value

### Documentation Style for Plan Status

- Prefer stable wording (for example: "tracked in the plan document") over point-in-time snapshots.
- Avoid embedding volatile counts, one-off verification snapshots, or temporary pass/fail badges.
- Keep detailed status updates in the linked plan documents and CI/PR logs.
- When referencing internal files or documents, prefer explicit relative Markdown links over plain path text.

## Relationship to Evergreen Docs

These implementation plans are **temporary** and complement the **evergreen**
documentation in [`docs/`](../):

| Evergreen (Permanent)    | Implementation Plans (Temporary)            |
| ------------------------ | ------------------------------------------- |
| `ARCHITECTURE.md`        | Component-specific implementation plans     |
| `HOW_TO_ADD_A_PARSER.md` | `PARSER_PLAN.md`                            |
| `TESTING_STRATEGY.md`    | Test plans within implementation docs       |
| `adr/`                   | Design decisions made during implementation |
| `improvements/`          | Beyond-parity features documented here      |

Once a feature is complete, relevant architectural decisions move to ADRs, and the implementation plan is archived.
