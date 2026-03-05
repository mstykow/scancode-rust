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

### Phase 1: Pipeline Overview
- [x] Python pipeline architecture documented (PYTHON_PIPELINE.md)
- [x] Rust pipeline architecture documented (RUST_PIPELINE.md)
- [ ] High-level flow comparison

### Phase 2: Core Components
- [x] Query construction and tokenization (QUERY_TOKENIZATION.md)
- [ ] License database/index structure
- [ ] Text preprocessing
- [ ] Matching algorithms
- [ ] Confidence scoring
- [ ] Rule engine
- [ ] License expression handling

### Phase 3: Data & Resources
- [ ] SPDX license data
- [ ] License texts and rules
- [ ] Configuration and thresholds

### Phase 4: Integration Points
- [ ] CLI integration
- [ ] Output format
- [ ] Error handling

## Documents

- [PYTHON_PIPELINE.md](./PYTHON_PIPELINE.md) - Python pipeline architecture
- [RUST_PIPELINE.md](./RUST_PIPELINE.md) - Rust pipeline architecture
- [QUERY_TOKENIZATION.md](./QUERY_TOKENIZATION.md) - Query construction and tokenization
- [LICENSE_DATABASE.md](./LICENSE_DATABASE.md) - License index and storage
- [MATCHING_ALGORITHM.md](./MATCHING_ALGORITHM.md) - Core matching logic
- [SCORING.md](./SCORING.md) - Confidence calculation
- [RULE_ENGINE.md](./RULE_ENGINE.md) - Rule-based detection
- [DIFFERENCES.md](./DIFFERENCES.md) - Final condensed report

## Status

**Current Phase**: 2 - Core Components
**Started**: 2026-03-05

### Completed
- [x] Python pipeline architecture documented (PYTHON_PIPELINE.md)
- [x] Rust pipeline architecture documented (RUST_PIPELINE.md)
- [x] Query construction and tokenization documented (QUERY_TOKENIZATION.md)
