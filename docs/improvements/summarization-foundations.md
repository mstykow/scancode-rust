# Summarization Foundations Improvements

## Summary

Rust now has the first reusable post-processing summary foundation layer, even though full Python `summarycode` parity is still not complete.

This incremental layer adds:

1. file-level key-file classification flags
2. package metadata promotion from key files
3. top-level `summary` output structure
4. initial non-license-dependent summary fields such as `declared_holder`, `primary_language`, and `other_languages`

## Why This Matters

Historically, parser/assembly parity work could associate files to packages, but there was no generic summary layer that turned those relationships into higher-level scan output.

That meant ecosystem-specific fixes like Ruby nested `LICENSE` handling had nowhere to publish their effect except ad hoc package fields.

This foundation creates the missing bridge between:

- parser/assembly/package relationships
- file classification
- package metadata promotion
- top-level scan summary output

## Implemented Foundations

### Key-file tagging

Files can now be tagged with:

- `is_legal`
- `is_manifest`
- `is_readme`
- `is_top_level`
- `is_key_file`

These flags are driven by package association, file references, and package-root context rather than only raw filesystem depth.

### Package metadata promotion

When package metadata is missing, key files can now backfill:

- `declared_license_expression`
- `declared_license_expression_spdx`
- `license_detections`
- `copyright`
- `holder`

### Summary output model

Top-level output now supports a `summary` block that can evolve incrementally without forcing all ecosystems to wait for full summarizer parity.

### Initial non-license-dependent summary fields

The current incremental layer now computes:

- `declared_holder`
- `primary_language`
- `other_languages`

These use already-available package/file metadata and do not depend on full license-tally parity.

## Not Full Python Parity Yet

This does **not** mean Rust now matches Python `summarycode` completely.

Still open:

- license tallies
- copyright tallies
- package tallies
- full `license_clarity_score` heuristic parity
- facets
- generated code detection
- broader summary output parity

## Validation

- `cargo test --bin scancode-rust`
- `cargo test --test output_format_golden`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`

## Related Plans

- `docs/implementation-plans/post-processing/SUMMARIZATION_PLAN.md`
- `docs/implementation-plans/package-detection/PARSER_PLAN.md`
