#!/bin/bash
#
# Setup script for Provenant development
#
# This script initializes git submodules and, when Node tooling is present, installs git hooks.
# License rules and licenses are in the reference/scancode-toolkit submodule.
#
# You only need to run this script if you:
# - Are building from source for the first time
# - Want to install git hooks after installing Node dependencies
# - Want to update to the latest license rules/licenses
#
# Run this script:
# - Before building from source for the first time

set -e

ARTIFACT="resources/license_detection/license_index.zst"
LFS_POINTER_PREFIX="version https://git-lfs.github.com/spec/v1"

echo "Initializing submodules..."
git submodule update --init --filter=blob:none

echo ""
echo "Checking embedded license index artifact..."
if [ ! -f "$ARTIFACT" ] || grep -q "^$LFS_POINTER_PREFIX$" "$ARTIFACT"; then
    echo "Embedded license index is missing or still a Git LFS pointer; regenerating..."
    cargo run --manifest-path xtask/Cargo.toml --bin generate-index-artifact
    echo "Cleaning stale cargo outputs so the regenerated artifact gets re-embedded..."
    cargo clean -p provenant-cli -p provenant-xtask
    echo "✅ Embedded license index generated"
else
    echo "✅ Embedded license index is ready"
fi

echo ""
echo "Installing git hooks..."
if [ -x node_modules/.bin/lefthook ]; then
    npm run hooks:install
    echo "✅ Lefthook hooks installed"
else
    echo "⚠️  Lefthook is not installed yet. Run npm install first (it will auto-install hooks), or run:"
    echo "   npm install"
    echo "   npm run hooks:install"
fi

echo ""
echo "Setup complete."
echo ""
echo "To build: cargo build --release"
echo ""
