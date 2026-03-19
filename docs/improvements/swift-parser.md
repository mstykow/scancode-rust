# Swift Ecosystem: Root Package and Assembly Improvements

## Summary

**🐛 Bug Fix + 🔍 Enhanced Extraction**: Rust now assembles Swift package metadata with artifact-aware precedence instead of letting whichever Swift file happens to be present over- or under-assert top-level package identity.

## Problem surface

Swift package metadata can come from several adjacent artifacts, including manifest-derived data, resolved files, and `swift-show-dependencies` output. Parser-level extraction alone was not enough because the important remaining behavior lives at scan and assembly time.

Without Swift-specific precedence rules, scans could:

- miss the top-level package entirely
- let dependency-oriented files overwrite manifest-owned root metadata
- lose resolved version information when the richer graph surface is absent
- attach nested Swift resources to the wrong package root

## Rust improvement

Rust applies Swift-specific assembly rules with five durable behaviors:

1. **Manifest-owned root package**
   Manifest-derived metadata owns the root package whenever it exists, instead of letting lockfile-like data define top-level identity.

2. **Show-dependencies contributes the dependency graph**
   `swift-show-dependencies` data can replace or enrich the dependency graph without taking ownership of the root package metadata.

3. **Resolved fallback when richer graph data is absent**
   Resolved files can still improve dependency version fidelity when show-dependencies output is not present.

4. **Resolved-only package emission**
   Repositories that only contain resolved data still produce useful package-level results instead of disappearing from assembled output.

5. **Nested-root resource isolation**
   Nested Swift package roots keep their own files, rather than being accidentally claimed by a parent package.

## Why this matters

- **Correct top-level package identity**: Swift scans emit the intended root package instead of deriving it from the wrong artifact
- **Better dependency fidelity**: each Swift artifact contributes where it is strongest without corrupting adjacent metadata
- **Safer package ownership**: nested Swift packages no longer inherit the wrong file assignments
- **More useful partial scans**: resolved-only repositories still produce meaningful package results

## Relationship to `swift-show-dependencies-parser.md`

`swift-show-dependencies-parser.md` documents the parser-level improvement for extracting a fuller dependency graph from `swift-show-dependencies.deplock`.

This document covers the broader ecosystem-level behavior, how manifest, resolved, and show-dependencies artifacts are combined into coherent Swift scan output.
