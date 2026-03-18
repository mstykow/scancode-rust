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
