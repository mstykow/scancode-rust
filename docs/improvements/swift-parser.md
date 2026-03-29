# Swift Ecosystem: Root Package and Assembly Improvements

## Summary

**🐛 Bug Fix + 🔍 Enhanced Extraction**: Rust now assembles Swift package metadata with artifact-aware precedence instead of letting whichever Swift file happens to be present over- or under-assert top-level package identity, and it preserves Swift parser identity on malformed manifest fallback rows.

## Problem surface

Swift package metadata can come from several adjacent artifacts, including manifest-derived data, resolved files, and `swift-show-dependencies` output. Parser-level extraction alone was not enough because the important remaining behavior lives at scan and assembly time.

Without Swift-specific precedence rules and intent boundaries, scans could:

- miss the top-level package entirely
- let dependency-oriented files overwrite manifest-owned root metadata
- lose resolved version information when the richer graph surface is absent
- flatten every resolved pin into a direct root dependency when only lockfile data is meant to enrich declared manifest intent
- attach nested Swift resources to the wrong package root

## Rust improvement

Rust applies Swift-specific assembly rules with five durable behaviors:

1. **Manifest-owned root package**
   Manifest-derived metadata owns the root package whenever it exists, instead of letting lockfile-like data define top-level identity.

2. **Show-dependencies contributes the dependency graph**
   `swift-show-dependencies` data can replace or enrich the dependency graph without taking ownership of the root package metadata.

3. **Resolved fallback when richer graph data is absent**
   Resolved files can still improve dependency version fidelity when show-dependencies output is not present, but only by enriching manifest-known dependencies instead of re-declaring every pin as a direct root dependency.

4. **Resolved-only package emission**
   Repositories that only contain resolved data still produce useful package-level results instead of disappearing from assembled output.

5. **Nested-root resource isolation**
   Nested Swift package roots keep their own files, rather than being accidentally claimed by a parent package.

6. **Identified malformed-manifest fallback**
   When Swift manifest data cannot be read or parsed, Rust still emits a Swift manifest fallback row with Swift parser identity instead of an anonymous empty package record.

## Why this matters

- **Correct top-level package identity**: Swift scans emit the intended root package instead of deriving it from the wrong artifact
- **Better dependency fidelity**: each Swift artifact contributes where it is strongest without corrupting adjacent metadata
- **Less overstated intent**: manifest and lockfile artifacts stop claiming runtime/directness they cannot actually prove
- **Safer package ownership**: nested Swift packages no longer inherit the wrong file assignments
- **More useful partial scans**: resolved-only repositories still produce meaningful package results
- **Clearer malformed-input output**: Swift manifest failures still point back to the Swift manifest parser and datasource instead of disappearing into untyped fallback output

## Relationship to `swift-show-dependencies-parser.md`

`swift-show-dependencies-parser.md` documents the parser-level improvement for extracting a fuller dependency graph from `swift-show-dependencies.deplock`.

This document covers the broader ecosystem-level behavior, how manifest, resolved, and show-dependencies artifacts are combined into coherent Swift scan output.
