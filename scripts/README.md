# Scripts Documentation

## URL Validation Script

### Purpose

`validate_urls.py` systematically validates all URLs in production documentation and Rust docstrings to catch broken links before they reach users.

### What It Validates

**Included**:

- All markdown files in `docs/` (except `docs/archived/`)
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
  continue-on-error: true  # Informational only - doesn't block PRs
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
