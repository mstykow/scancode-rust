# License Detection Gaps

This file tracks known license-detection parity gaps that we are intentionally
not fixing right now.

## `lic2/bsd-new_156.pdf`

- Fixture: `testdata/license-golden/datadriven/lic2/bsd-new_156.pdf`
- Expected golden output: `bsd-new`
- Current Rust behavior: no usable text is extracted from this PDF, so raw
  license detection never runs on the document content.
- Python behavior: extracts embedded PDF text successfully and then detects the
  BSD notice.

Status:

- Known gap.
- The golden fixture is temporarily skipped in the Rust golden test suite.

Why skipped:

- This is an input-extraction problem, not a matcher/refinement problem.
- Previous attempts at improving the current PDF path did not recover text for
  this file.
- We prefer to defer this until we can implement a reliable PDF extraction fix
  without destabilizing the remaining matcher parity work.

## License and Rule Metadata Parity

- Known gap.
- Rust currently parses some Python-compatible license/rule frontmatter fields
  in `src/license_detection/rules/loader.rs` but does not carry them through to
  the Rust models or any downstream behavior yet.

Currently parsed-but-unused license metadata:

- `owner`
- `osi_license_key`
- `is_exception`
- `standard_notice`

Currently parsed-but-unused rule metadata:

- `skip_for_required_phrase_generation`
- `replaced_by`

Why deferred:

- These are real Python metadata fields, so we want to keep parsing them for
  schema compatibility with upstream rule/license data.
- Rust does not yet implement the corresponding parity features that justify
  using them.

Missing follow-up work:

- extend Rust license/rule models to carry the missing metadata,
- implement Python-style required-phrase generation behavior that uses
  `skip_for_required_phrase_generation`,
- decide how deprecated rule replacements should be modeled and surfaced,
- decide which license metadata fields belong in Rust output or internal APIs.
