# Gradle Module Metadata Parser

**Parser**: `GradleModuleParser`

## Why This Exists

Python ScanCode currently has multiple open attempts at parsing Gradle `.module` metadata, but no merged handler. Provenant now parses published Gradle module metadata directly.

## What We Extract

- Maven package identity from `component.group`, `component.module`, and `component.version`
- artifact size and checksums from published variant `files`
- deduplicated dependencies across non-documentation variants
- scope inference from `org.gradle.usage` and variant names
- per-package file references for published artifacts
- producer metadata such as `formatVersion` and `createdBy.gradle.version`
- preserved variant metadata for `dependencyConstraints` and `available-at`

## Reference limitation

The Python reference does not currently provide merged Gradle `.module` support, so publication metadata is thinner than modern Gradle-native projects expect.

## Rust behavior

Rust parses published Gradle module metadata directly and preserves artifact, dependency, constraint, and variant information that build-script parsing alone cannot recover reliably.

## Impact

- Better JVM dependency visibility for published Gradle metadata
- Better artifact provenance than build-script parsing alone
- Better coverage of modern Gradle-native publication semantics
