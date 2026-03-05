# License Detection Engine Audit

## Objective

Systematic comparison of the license detection engine between Python ScanCode Toolkit (reference) and Rust implementation to identify all differences, both intentional and unintentional.

## Scope

- **Reference**: `reference/scancode-toolkit/src/licensedcode/` (Python)
- **Implementation**: `src/license_detection/` (Rust)
- **Focus**: Logic, algorithms, data structures, edge case handling
- **Out of Scope**: Bug fixes, code changes

## Methodology

1. **Layer-by-layer analysis**: Start from high-level pipeline, drill down to components
2. **Subagent-driven investigation**: Each topic investigated by specialized agents
3. **Documented findings**: Results recorded in dedicated audit documents
4. **Final synthesis**: Condensed report highlighting all differences

## Audit Progress

### Phase 1: Pipeline Overview ✅
- [x] Python pipeline architecture (PYTHON_PIPELINE.md)
- [x] Rust pipeline architecture (RUST_PIPELINE.md)

### Phase 2: Core Components ✅
- [x] License database/index structure (LICENSE_DATABASE.md)
- [x] Query/tokenization (QUERY_TOKENIZATION.md)
- [x] Matching algorithms (MATCHING_ALGORITHMS.md)
- [x] Match refinement (MATCH_REFINEMENT.md)
- [x] Scoring (SCORING.md)
- [x] Rule engine (RULE_ENGINE.md)
- [x] License expression handling (EXPRESSION_HANDLING.md)
- [x] Detection assembly (DETECTION_ASSEMBLY.md)

### Phase 3: Data & Resources ✅
- [x] SPDX license data (SPDX_DATA.md)
- [x] Constants and thresholds (CONSTANTS_THRESHOLDS.md)

### Phase 4: Integration Points ✅
- [x] CLI integration and output format (CLI_OUTPUT.md)

## Documents

| Document | Topic | Status |
|----------|-------|--------|
| [PYTHON_PIPELINE.md](./PYTHON_PIPELINE.md) | Python pipeline architecture | ✅ Complete |
| [RUST_PIPELINE.md](./RUST_PIPELINE.md) | Rust pipeline architecture | ✅ Complete |
| [LICENSE_DATABASE.md](./LICENSE_DATABASE.md) | License index and storage | ✅ Complete |
| [QUERY_TOKENIZATION.md](./QUERY_TOKENIZATION.md) | Query construction/tokenization | ✅ Complete |
| [MATCHING_ALGORITHMS.md](./MATCHING_ALGORITHMS.md) | Core matching logic | ✅ Complete |
| [MATCH_REFINEMENT.md](./MATCH_REFINEMENT.md) | Match filtering/merging | ✅ Complete |
| [SCORING.md](./SCORING.md) | Confidence calculation | ✅ Complete |
| [RULE_ENGINE.md](./RULE_ENGINE.md) | Rule loading/thresholds | ✅ Complete |
| [EXPRESSION_HANDLING.md](./EXPRESSION_HANDLING.md) | License expression parsing | ✅ Complete |
| [DETECTION_ASSEMBLY.md](./DETECTION_ASSEMBLY.md) | Detection grouping | ✅ Complete |
| [SPDX_DATA.md](./SPDX_DATA.md) | SPDX license data/mapping | ✅ Complete |
| [CONSTANTS_THRESHOLDS.md](./CONSTANTS_THRESHOLDS.md) | Constants comparison | ✅ Complete |
| [CLI_OUTPUT.md](./CLI_OUTPUT.md) | CLI and output format | ✅ Complete |
| [DIFFERENCES.md](./DIFFERENCES.md) | **Final condensed report** | ✅ Complete |

## Summary

**Audit Complete**: 14 documents covering all aspects of the license detection engine.

**Key Finding**: 22 differences identified, categorized as:
- **7 Critical** (affect detection results)
- **5 High Priority** (affect golden tests)
- **5 Medium Priority** (affect output/completeness)
- **5 Low Priority** (implementation details)

See [DIFFERENCES.md](./DIFFERENCES.md) for the complete analysis.

## Status

**Audit Complete**: 2026-03-05
