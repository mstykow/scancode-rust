# Releasing Provenant

This guide documents the maintainer release flow for `provenant`.

## Overview

Releases are driven locally with `release.sh`, which wraps `cargo release` and ensures the embedded license data is refreshed before publishing.

The published crate name is `provenant-cli`, while the installed binary and product name remain `provenant` / Provenant.

## Prerequisites

Before cutting a release, make sure you have:

- A clean working tree
- The `reference/scancode-toolkit/` submodule initialized via `./setup.sh`
- `cargo-release` installed locally
- A valid crates.io login in your Cargo credentials
- GPG signing configured for git tags

Install `cargo-release` if needed:

```sh
cargo install cargo-release
```

Authenticate with crates.io if needed:

```sh
cargo login
```

## Preflight Checks

Before the actual release, verify the repository is in good shape:

```sh
cargo test --all --release --verbose
npm ci
npm run check:docs
```

The GitHub `Quality Checks` workflow runs formatting, clippy, compilation, tests, golden tests, and documentation checks. It is best to start from a branch and commit state where those checks are already green.

## Release Commands

Always start with a dry run:

```sh
./release.sh patch
```

When the dry run looks correct, perform the real release:

```sh
./release.sh patch --execute
```

Supported release types:

- `patch` updates `X.Y.Z` to `X.Y.(Z+1)`
- `minor` updates `X.Y.Z` to `X.(Y+1).0`
- `major` updates `X.Y.Z` to `(X+1).0.0`

## What `release.sh` Does

On every release attempt, the script:

1. Checks that the ScanCode reference submodule is present.
2. Fetches the latest `origin/develop` for `reference/scancode-toolkit`.
3. Updates the submodule checkout if the upstream commit changed.
4. Regenerates `resources/license_detection/license_index_loader.msgpack.zst`.
5. In `--execute` mode, commits that license-data refresh as `chore: update license rules/licenses to latest` when needed.
6. Runs `cargo release <patch|minor|major>` in dry-run or execute mode.

The repository is configured so `cargo release`:

- Creates the release commit as `chore: release vX.Y.Z`
- Regenerates `xtask/Cargo.lock` after bumping the crate version and before creating the release commit
- Creates a GPG-signed tag `vX.Y.Z`
- Publishes the crate to crates.io
- Pushes the commit and tag to GitHub

## GitHub Release Automation

Pushing the `vX.Y.Z` tag triggers `.github/workflows/release.yml`.

That workflow:

- Builds release binaries for:
  - `x86_64-unknown-linux-gnu`
  - `aarch64-unknown-linux-gnu`
  - `aarch64-apple-darwin`
  - `x86_64-pc-windows-msvc`
- Packages each build as `.tar.gz` or `.zip`
- Generates SHA256 checksum files
- Creates a GitHub Release and uploads all generated assets

If the tag contains `-`, GitHub marks the release as a prerelease.

## After Starting the Release

Monitor the [GitHub Actions release workflow](https://github.com/mstykow/provenant/actions) and the resulting [GitHub Releases page](https://github.com/mstykow/provenant/releases).

Verify:

- The crates.io publish step succeeded
- The tag and release commit are present on the remote
- The GitHub Release contains all expected platform archives and checksum files

## Common Failure Points

- Missing submodule setup: run `./setup.sh`
- Missing crates.io credentials: run `cargo login`
- Missing GPG configuration: `cargo release` cannot create the signed tag
- Dirty working tree: clean up local changes before retrying
