# Scripts Documentation

## Golden Fixture Helper Scripts

### Parser Golden Snapshots

`update_parser_golden.sh` updates parser `.expected.json` golden snapshots by invoking the `update-parser-golden` binary.

**Why this exists**: it regenerates parser golden expectations directly from parser output so fixture updates stay deterministic and aligned with parser behavior.

Show CLI help:

```bash
cargo run --bin update-parser-golden -- --help
```

CLI arguments:

- `<ParserType>`: parser struct name (for example `NpmParser`)
- `<input_file>`: fixture input file to parse
- `<output_file>`: `.expected.json` file to write
- `--list`: list all registered parser types

```bash
./scripts/update_parser_golden.sh <ParserType> <input_file> <output_file>
```

### Copyright Golden YAML Fixtures

`update_copyright_golden.sh` syncs and updates copyright golden YAML fixtures (authors / ics / copyrights).

**Why this exists**: it keeps copyright golden YAMLs in sync with the Rust detector and with Rust-owned fixture policy while preserving reviewable mismatch workflows.

Show CLI help:

```bash
cargo run --bin update-copyright-golden -- --help
```

CLI arguments:

- `<authors|ics|copyrights>`: fixture suite to process
- `--list-mismatches`: print files where Python reference expectations differ from current Rust detector output (parity precheck)
- `--show-diff`: print missing/extra summary for those Python-reference parity mismatches (plus samples with `--filter`)
- `--filter PATTERN`: limit processing to paths containing `PATTERN`
- `--sync-actual`: write expected values from current Rust detector output
- `--write`: apply file updates (without it, command is dry-run)

`ics` here refers to the Android Ice Cream Sandwich (Android 4.0) fixture corpus from ScanCode reference tests.

The updater also removes legacy `expected_failures` keys, preventing Python xfail metadata from being reintroduced into Rust-owned fixtures.

Important distinction: this command is a maintenance/sync tool. Golden tests compare Rust detector output to local Rust-owned fixture YAMLs; `--list-mismatches` compares Rust detector output to Python reference expectations to decide whether a sync is parity-safe.

### Expected Workflow (Copyright Fixtures)

Use this workflow when maintaining `testdata/copyright-golden/*`:

1. **Check Python-reference parity impact first**
   - Run:

     ```bash
     ./scripts/update_copyright_golden.sh copyrights --list-mismatches --show-diff
     ```

   - Purpose: identify fixtures where current Rust output diverges from upstream Python reference expectations.

2. **If parity is acceptable for a fixture, sync from Python reference**
   - Run with `--write` (optionally with `--filter`):

     ```bash
     ./scripts/update_copyright_golden.sh copyrights --filter <pattern> --write
     ```

   - This is a **selective, parity-gated sync** from `reference/scancode-toolkit/tests/cluecode/data/...`.

3. **If divergence is intentional or Rust-specific, update to Rust actuals**
   - Run:

     ```bash
     ./scripts/update_copyright_golden.sh copyrights --sync-actual --filter <pattern> --write
     ```

   - Use this for accepted Rust improvements or known intentional differences.

4. **Validate with tests**
   - Run golden tests after updates to confirm repository expectations are coherent.

Notes:

- Python sync workflow applies to **copyright fixtures only**.
- Parser golden updater (`update-parser-golden`) does **not** sync from Python reference; it always generates expectations from Rust parser output.

```bash
./scripts/update_copyright_golden.sh <authors|ics|copyrights> [--list-mismatches] [--show-diff] [--filter PATTERN] [--write]
```

Useful examples:

```bash
./scripts/update_copyright_golden.sh copyrights --list-mismatches --show-diff
./scripts/update_copyright_golden.sh copyrights --filter essential_smoke --write
```

## URL Validation Script

### Purpose

`validate_urls.py` systematically validates all URLs in production documentation and Rust docstrings to catch broken links before they reach users.

### What It Validates

**Included**:

- All markdown files in `docs/`
- Root markdown files: `README.md`, `AGENTS.md`, etc.
- Rust docstrings (`///` and `//!`) in `src/`

**Excluded** (not our responsibility):

- `reference/` - Python ScanCode Toolkit submodule (upstream)
- `resources/licenses/` - SPDX license data submodule (upstream)
- `testdata/` - Test fixtures and sample data
- `target/` - Build artifacts
- `.sisyphus/` - Session data
- Test files: `*_test.rs`, files in `tests/` directories

### Usage

```bash
# Manual run
python3 scripts/validate_urls.py

# Exit codes:
#   0 = All URLs valid
#   1 = Some URLs failed validation
```

### Output

The script provides:

- Progress updates during validation
- Failed URLs with file locations
- Summary statistics
- Skipped URLs (templates, placeholders)

Example output:

```text
❌ FAIL: https://example.com/broken
   Reason: HTTP 404
   Found in:
     - docs/ARCHITECTURE.md:42

✅ 137 URLs validated successfully
❌ 3 URLs failed validation
```

### CI/CD Integration

**Configured in** `.github/workflows/docs-quality.yml`:

```yaml
- name: Validate Documentation URLs
  run: python3 scripts/validate_urls.py
  continue-on-error: true # Informational only - doesn't block PRs
```

Runs on:

- Every push to `main` (when docs or scripts change)
- Every pull request (when docs change)

**Note**: URL validation is informational only and does not block PRs. This prevents contributors from being blocked by:

- URLs that don't exist yet on remote (unpushed changes)
- Sites blocking CI user agents (e.g., crates.io)
- Transient network failures

### When It Reports Failures

If the check reports broken URLs:

1. **Review the output** - Check which URLs are broken
2. **Fix actual broken links** - Update or remove genuinely broken URLs in our docs
3. **Ignore expected failures**:

- URLs to unpushed GitHub paths (will resolve after push)
- crates.io URLs (blocks CI user agents, validated in allowlist)
- Submodule URLs in `reference/` or `resources/` (not our responsibility)

### Maintenance

**No regular maintenance needed** - The script automatically:

- Skips template URLs (containing `{`, `<`, `...`)
- Handles relative URLs and fragments
- Validates in parallel (10 concurrent requests)
- Times out after 10 seconds per URL

---

## URL Extraction Script

### Purpose

`extract_urls.sh` is a helper script for extracting all URLs from documentation (used during development/debugging).

### Usage

```bash
./scripts/extract_urls.sh > all_urls.txt
```

**Note**: This script is for manual inspection only, not used in CI/CD.
