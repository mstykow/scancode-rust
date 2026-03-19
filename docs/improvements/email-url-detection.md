# Email/URL Detection

## Type

- 🐛 Bug Fix + 🔍 Enhanced Extraction + 🛡️ Security

## Python Reference Status

- Email regex limits TLDs to 2-4 characters, missing valid longer TLDs.
- IPv6/private IP handling has known correctness issues.
- URL canonicalization/error handling is less explicit.

## Rust Implementation Status

- Rust uses modern regex handling plus threshold and uniqueness controls for email extraction.
- URL extraction applies ordered cleaning and filtering, including credential stripping.
- Host and IP filtering remove more junk and private-address noise.
- Scanner and CLI integration expose the feature through the existing email and URL flags.
- Supported images can feed EXIF and XMP metadata text into the existing finder pipeline.
- Golden fixtures are owned locally in this repository so the behavior does not depend on external submodule state.

## Impact

- Better correctness and parity confidence for email/URL extraction.
- Additional beyond-parity detections from EXIF/XMP image metadata on supported image formats.
- Safer handling of sensitive URL credential data.
- Stable, repo-owned regression coverage independent of Python submodule state.
