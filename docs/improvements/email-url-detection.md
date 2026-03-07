# Email/URL Detection

## Type

- 🐛 Bug Fix + 🔍 Enhanced Extraction + 🛡️ Security

## Python Reference Status

- Email regex limits TLDs to 2-4 characters, missing valid longer TLDs.
- IPv6/private IP handling has known correctness issues.
- URL canonicalization/error handling is less explicit.

## Rust Implementation Status

- `src/finder/emails.rs` uses modern regex + threshold/uniqueness controls.
- `src/finder/urls.rs` applies ordered URL cleaning/filtering and credential stripping.
- `src/finder/host.rs` and `src/finder/junk_data.rs` implement host/IP and junk filtering.
- Scanner/CLI integration is implemented (`--email`, `--max-email`, `--url`, `--max-url`).
- Supported images can feed EXIF/XMP metadata text into the existing finder pipeline.
- Golden fixtures are local to this repo (`testdata/plugin_email_url/`) to avoid submodule coupling.

## Impact

- Better correctness and parity confidence for email/URL extraction.
- Additional beyond-parity detections from EXIF/XMP image metadata on supported image formats.
- Safer handling of sensitive URL credential data.
- Stable, repo-owned regression coverage independent of Python submodule state.
