#!/usr/bin/env bash
set -uo pipefail

if ! command -v cargo-machete &> /dev/null; then
    echo "Installing cargo-machete..."
    cargo install cargo-machete
fi

# cargo machete exits 1 when *any* Cargo.toml has unused deps (including testdata fixtures).
# We only care about our own Cargo.toml, so we capture output and check manually.
# Runs without --with-metadata to avoid generating stray Cargo.lock files in submodules.
# False positives from renamed crates (e.g. md-5 -> md5) are handled via
# [package.metadata.cargo-machete] ignored list in Cargo.toml.
output=$(cargo machete 2>&1 || true)

if echo "$output" | grep -q "^scancode-rust -- ./Cargo.toml:"; then
    echo "Unused dependencies found in Cargo.toml:"
    echo "$output" | sed -n '/^scancode-rust/,/^$/p'
    exit 1
fi

echo "No unused dependencies in Cargo.toml."
