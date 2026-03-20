#!/bin/bash
#
# Setup script for Provenant development
#
# This script initializes/updates the SPDX License List Data submodule to the latest version.
# The license data in resources/licenses/json/details/ is embedded into the binary
# at compile time using the include_dir! macro.
#
# NOTE: This script is OPTIONAL for most users. The binary already ships with a
# built-in license index embedded from the checked-in artifact at:
#   resources/license_detection/license_index_loader.msgpack.zst
#
# You only need to run this script if you:
# - Are updating the SPDX license definitions
# - Need to regenerate the embedded license loader artifact
# - Want to use custom license rules with --license-rules-path
#
# Sparse checkout is used to only fetch the json/details directory (~693 license files),
# reducing disk usage by ~90% compared to cloning the entire repository.
#
# Run this script:
# - Before building from source for the first time (optional)
# - Anytime you want to update to the latest SPDX license definitions

set -e

echo "Initializing submodule..."
git submodule update --init --depth=1

echo "Configuring sparse checkout for license data..."
cd resources/licenses
git sparse-checkout init --cone
git sparse-checkout set json/details

echo "Updating to latest license data..."
CURRENT_COMMIT=$(git rev-parse HEAD)
git fetch origin main --depth=1
# Suppress "detached HEAD" warning - this is expected for submodules
git -c advice.detachedHead=false checkout origin/main
NEW_COMMIT=$(git rev-parse HEAD)

cd ../..

if [ "$CURRENT_COMMIT" != "$NEW_COMMIT" ]; then
    echo "✅ License data updated: ${CURRENT_COMMIT:0:7} → ${NEW_COMMIT:0:7}"
    echo "⚠️  Remember to update the embedded license loader artifact and commit the submodule update:"
    echo "   ./scripts/update_license_loader_artifact.sh"
    echo "   git add resources/licenses"
    echo "   git commit -m 'chore: update SPDX license data'"
else
    echo "✅ License data already up to date (${CURRENT_COMMIT:0:7})"
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
