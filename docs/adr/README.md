# Architectural Decision Records (ADRs)

This directory contains records of architectural decisions made during the development of scancode-rust.

## What is an ADR?

An Architectural Decision Record (ADR) is a document that captures an important architectural decision made along with its context and consequences. ADRs help:

- Preserve the reasoning behind key design decisions
- Onboard new contributors by explaining "why" not just "what"
- Avoid revisiting settled decisions without new information
- Document trade-offs and alternatives considered

## Format

Each ADR follows a consistent structure:

- **Status**: Proposed, Accepted, Deprecated, Superseded
- **Context**: The problem or requirement that prompted the decision
- **Decision**: The architectural choice made
- **Consequences**: Trade-offs, benefits, and implications
- **Alternatives Considered**: Other options evaluated

## Index of ADRs

| ADR | Title | Status | Date |
|-----|-------|--------|------|
| [0001](0001-trait-based-parsers.md) | Trait-Based Parser Architecture | Accepted | 2026-02-08 |
| [0002](0002-extraction-vs-detection.md) | Extraction vs Detection Separation | Accepted | 2026-02-08 |
| [0003](0003-golden-test-strategy.md) | Golden Test Strategy | Accepted | 2026-02-08 |
| [0004](0004-security-first-parsing.md) | Security-First Parsing | Accepted | 2026-02-08 |
| [0005](0005-auto-generated-docs.md) | Auto-Generated Documentation | Accepted | 2026-02-08 |

## Creating a New ADR

1. Copy the template: `cp template.md 000N-short-title.md`
2. Fill in the sections with your decision context and rationale
3. Update this README with the new entry
4. Submit for review via pull request

## ADR Lifecycle

- **Proposed**: Under discussion, not yet implemented
- **Accepted**: Decision made and being followed
- **Deprecated**: No longer recommended but not yet replaced
- **Superseded**: Replaced by a newer ADR (link to the replacement)

## Further Reading

- [Documenting Architecture Decisions](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions) by Michael Nygard
- [ADR GitHub Organization](https://adr.github.io/)
