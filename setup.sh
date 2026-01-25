#!/bin/bash
#
# Setup script for scancode-rust development
#
# This script initializes/updates the SPDX License List Data submodule to the latest version.
# The license data in resources/licenses/json/details/ is embedded into the binary
# at compile time using the include_dir! macro.
#
# Sparse checkout is used to only fetch the json/details directory (~693 license files),
# reducing disk usage by ~90% compared to cloning the entire repository.
#
# Run this script:
# - Before building from source for the first time
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
    echo "⚠️  Remember to commit the submodule update:"
    echo "   git add resources/licenses"
    echo "   git commit -m 'chore: update SPDX license data'"
else
    echo "✅ License data already up to date (${CURRENT_COMMIT:0:7})"
fi

echo ""
echo "Submodule setup complete. Run 'cargo build --release' to embed it into the binary."
