#!/bin/bash
#
# Setup script for Provenant development
#
# This script initializes git submodules and installs pre-commit hooks.
#
# The license detection index is already embedded in the binary at:
#   resources/license_detection/license_index_loader.msgpack.zst
#
# The resources/scancode-licenses submodule uses sparse checkout to fetch only
# the license rules and licenses directories (~180MB vs ~500MB+ for full repo).
#
# You only need to run this script if you:
# - Are building from source for the first time
# - Want to install pre-commit hooks
# - Want to update to the latest license rules/licenses
#
# Run this script:
# - Before building from source for the first time

set -e

echo "Initializing submodules..."
git submodule update --init --filter=blob:none

# Configure sparse checkout and update license data submodule
if [ -d "resources/scancode-licenses" ]; then
    echo "Configuring sparse checkout for license data..."
    cd resources/scancode-licenses
    git sparse-checkout init --no-cone
    git sparse-checkout set src/licensedcode/data/rules/ src/licensedcode/data/licenses/

    echo "Updating to latest license data..."
    CURRENT_COMMIT=$(git rev-parse HEAD 2>/dev/null || echo "none")
    git fetch origin develop --depth=1
    git -c advice.detachedHead=false checkout origin/develop
    NEW_COMMIT=$(git rev-parse HEAD)
    cd ../..

    if [ "$CURRENT_COMMIT" != "$NEW_COMMIT" ]; then
        echo "✅ License data updated: ${CURRENT_COMMIT:0:7} → ${NEW_COMMIT:0:7}"
        echo "⚠️  Remember to update the embedded license loader artifact and commit the submodule update:"
        echo "   cargo run --manifest-path xtask/Cargo.toml --bin generate-license-loader-artifact"
        echo "   git add resources/scancode-licenses"
        echo "   git commit -m 'chore: update license rules/licenses'"
    else
        echo "✅ License data already up to date (${CURRENT_COMMIT:0:7})"
    fi
fi

echo ""
echo "Installing pre-commit hooks..."
if command -v pre-commit >/dev/null 2>&1; then
    pre-commit install
    echo "✅ Pre-commit hooks installed"
else
    echo "⚠️  pre-commit is not installed. Install it, then run:"
    echo "   pre-commit install"
fi

echo ""
echo "Setup complete."
echo ""
echo "To build: cargo build --release"
echo ""
